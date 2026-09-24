# Safe on a developer machine: real shortcuts live in a unique temporary folder;
# persistent PATH reads are mocked and no installer transaction is executed.
$ErrorActionPreference = 'Stop'
$names = @('SD300_MANAGED_INSTALLER_TEST_ONLY', 'SD300_INSTALL_DIR',
    'TR300_TUI_INSTALL_DIR', 'CARGO_DIST_FORCE_INSTALL_DIR', 'CARGO_HOME',
    'TR300_TUI_NO_MODIFY_PATH', 'INSTALLER_NO_MODIFY_PATH', 'TR300_TUI_UNMANAGED_INSTALL')
$saved = @{}
foreach ($name in $names) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
    [Environment]::SetEnvironmentVariable($name, $null, 'Process')
}
$oldPath = $env:Path
$userPathBefore = [Environment]::GetEnvironmentVariable('Path', 'User')
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('sd300-discovery-test-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $testRoot
$script:checks = 0
function Assert-True([bool]$Value, [string]$Message) {
    if (-not $Value) { throw $Message }
    $script:checks++
}
function Assert-Fails([scriptblock]$Action, [string]$Expected) {
    try { & $Action } catch {
        Assert-True ($_.Exception.Message -like "*$Expected*") "Unexpected error: $($_.Exception.Message)"
        return
    }
    throw "Expected an error containing: $Expected"
}
try {
    $env:SD300_MANAGED_INSTALLER_TEST_ONLY = '1'
    . (Join-Path $PSScriptRoot 'managed-installers\sd300-installer.ps1')
    Assert-Sd300ManagedOptions
    $env:TR300_TUI_UNMANAGED_INSTALL = Join-Path $testRoot 'unmanaged'
    Assert-Fails { Assert-Sd300ManagedOptions } 'disables the receipt required'
    $env:TR300_TUI_UNMANAGED_INSTALL = $null
    $programs = [Environment]::GetFolderPath('Programs', 'DoNotVerify')
    Assert-True ((Get-Sd300GuiShortcut) -eq (Join-Path $programs 'SD-300.lnk')) 'Shortcut resolver must use the Windows Programs folder'

    # A redirected location with a previously absent parent must work. Do not
    # redirect the machine's actual known folder or register an Installed App.
    $script:fixtureShortcut = Join-Path $testRoot 'Redirected Programs\SD-300.lnk'
    function Get-Sd300GuiShortcut { return $script:fixtureShortcut }
    $root = Join-Path $testRoot 'SD-300 payload'
    $null = New-Item -ItemType Directory -Path (Join-Path $root 'app\assets') -Force
    [IO.File]::WriteAllText((Join-Path $root 'app\sd300-gui.exe'), '')
    [IO.File]::WriteAllText((Join-Path $root 'app\assets\app-icon.ico'), '')
    Install-Sd300GuiShortcut $root
    Assert-True (Test-Path -LiteralPath $script:fixtureShortcut -PathType Leaf) 'Shortcut must be saved under the configured directory'
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut($script:fixtureShortcut)
    Assert-True ($link.TargetPath -eq (Join-Path $root 'app\sd300-gui.exe')) 'Shortcut must launch the GUI directly'
    Assert-True ($link.IconLocation -eq ((Join-Path $root 'app\assets\app-icon.ico') + ',0')) 'Shortcut must carry the custom icon'
    $link.TargetPath = Join-Path $root 'wrong.exe'
    $link.Save()
    Assert-Fails { Assert-Sd300GuiShortcut $root } 'does not point to the installed SD-300 app'
    Remove-Item -LiteralPath $script:fixtureShortcut -Force
    Assert-Fails { Assert-Sd300GuiShortcut $root } 'was not created'
    $script:fixtureShortcut = Join-Path $testRoot 'blocked-parent\SD-300.lnk'
    [IO.File]::WriteAllText((Split-Path -Parent $script:fixtureShortcut), 'not a directory')
    Assert-Fails { Install-Sd300GuiShortcut $root } 'not a directory'

    $env:SD300_INSTALL_DIR = Join-Path $testRoot 'custom CLI'
    $binary = Join-Path $env:SD300_INSTALL_DIR 'bin\sd300.exe'
    $bin = Split-Path -Parent $binary
    $script:fixtureUserPath = ''
    function Get-Sd300UserPathState {
        [pscustomobject]@{ PathValue = $script:fixtureUserPath }
    }
    $env:Path = "$bin;$oldPath"
    Assert-True (Test-Sd300ModifyPath) 'A process-only PATH entry must not suppress persistent installation or rollback ownership'
    Assert-Fails { Assert-Sd300PersistentPath $binary } 'persistent user PATH'
    $script:fixtureUserPath = 'C:\unrelated;%SD300_INSTALL_DIR%\bin\;'
    Assert-Sd300PersistentPath $binary
    Assert-True (Test-Sd300PathContains $script:fixtureUserPath $bin) 'Expanded and trailing-separator PATH entries must be recognized'
    Assert-True (Test-Sd300PathContains $bin.ToUpperInvariant() $bin) 'Windows PATH comparison must be case-insensitive'
    Assert-True (-not (Test-Sd300PathContains ($bin + '-other') $bin)) 'A matching prefix is not a PATH match'
    Assert-True (-not (Test-Sd300PathContains '.\bin' $bin)) 'Relative entries must not prove persistent registration'
    $env:Path = 'C:\session-only;C:\another'
    Update-Sd300SessionPath $binary
    Assert-True ($env:Path -eq "$bin;C:\session-only;C:\another") 'Refresh must preserve unrelated session PATH entries'
    Update-Sd300SessionPath $binary
    Assert-True ($env:Path -eq "$bin;C:\session-only;C:\another") 'Repeated refresh must not duplicate PATH'
    $env:Path = $oldPath

    $child = Join-Path $testRoot 'child fixture.ps1'
    @'
param([switch]$NoModifyPath)
@{ prefix=$env:TR300_TUI_INSTALL_DIR; no_modify=[bool]$NoModifyPath } | ConvertTo-Json -Compress
'@ | Set-Content -LiteralPath $child -Encoding utf8
    $env:TR300_TUI_INSTALL_DIR = Join-Path $testRoot 'lower-priority override'
    $result = (Invoke-Sd300DistInstaller $child) | ConvertFrom-Json
    Assert-True ($result.prefix -eq $env:SD300_INSTALL_DIR) 'Child destination must equal the backed-up SD300_INSTALL_DIR'
    Assert-True (-not $result.no_modify) 'Default install must request PATH registration'
    Assert-True ($env:TR300_TUI_INSTALL_DIR -eq (Join-Path $testRoot 'lower-priority override')) 'Child override must be restored'
    $env:SD300_INSTALL_DIR = $null
    Assert-True ((Get-Sd300InstallPrefix) -eq $env:TR300_TUI_INSTALL_DIR) 'Package-specific prefix must participate in backup resolution'
    $env:SD300_INSTALL_DIR = Join-Path $testRoot 'custom CLI'
    foreach ($option in @('TR300_TUI_NO_MODIFY_PATH', 'INSTALLER_NO_MODIFY_PATH', 'TR300_TUI_UNMANAGED_INSTALL')) {
        [Environment]::SetEnvironmentVariable($option, '1', 'Process')
        Assert-True (-not (Test-Sd300ModifyPath)) "$option must opt out of PATH changes"
        $script:fixtureUserPath = ''
        Assert-Sd300PersistentPath $binary
        Update-Sd300SessionPath $binary
        Assert-True ($env:Path -eq $oldPath) "$option must preserve current PATH"
        $result = (Invoke-Sd300DistInstaller $child) | ConvertFrom-Json
        Assert-True $result.no_modify "$option must be forwarded explicitly to cargo-dist"
        [Environment]::SetEnvironmentVariable($option, $null, 'Process')
    }
    $NoModifyPath = $true
    $result = (Invoke-Sd300DistInstaller $child) | ConvertFrom-Json
    Assert-True $result.no_modify 'NoModifyPath switch must reach the child'
    $NoModifyPath = $false
    'exit 23' | Set-Content -LiteralPath $child -Encoding ascii
    Assert-Fails { Invoke-Sd300DistInstaller $child } 'exited with code 23'
    Assert-True ($env:TR300_TUI_INSTALL_DIR -eq (Join-Path $testRoot 'lower-priority override')) 'Failed child must restore the environment override'
    Assert-True ([Environment]::GetEnvironmentVariable('Path', 'User') -ceq $userPathBefore) 'Fixtures must not mutate persistent user PATH'
    Write-Host "PASS: $script:checks discovery/PATH checks on PowerShell $($PSVersionTable.PSVersion)"
} finally {
    $env:Path = $oldPath
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process') }
    $resolved = [IO.Path]::GetFullPath($testRoot)
    $expectedParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if ($resolved.StartsWith($expectedParent, [StringComparison]::OrdinalIgnoreCase) -and
        [IO.Path]::GetFileName($resolved) -like 'sd300-discovery-test-*') {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
