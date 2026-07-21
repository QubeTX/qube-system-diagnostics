param(
    [Parameter(Mandatory = $true)][string]$ArtifactDirectory,
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$BuiltBinary
)

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') {
    throw 'This destructive installer matrix is restricted to an ephemeral GitHub Actions runner.'
}

$ArtifactDirectory = [IO.Path]::GetFullPath($ArtifactDirectory)
$BuiltBinary = [IO.Path]::GetFullPath($BuiltBinary)
$globalRoot = Join-Path $env:ProgramFiles 'sd300'
$corporateRoot = Join-Path $env:LOCALAPPDATA 'Programs\sd300'
$managedBinary = Join-Path $env:USERPROFILE '.cargo\bin\sd300.exe'
$managedReceipt = Join-Path $env:LOCALAPPDATA 'sd300\sd300-receipt.json'

function Invoke-Checked([string]$FilePath, [string[]]$Arguments, [int[]]$Allowed = @(0)) {
    $process = Start-Process -FilePath $FilePath -ArgumentList $Arguments -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -notin $Allowed) {
        throw "$FilePath exited with $($process.ExitCode); expected $($Allowed -join ', ')"
    }
    return $process.ExitCode
}

function Invoke-GuiSelfTest([string]$Gui, [string]$Context) {
    $tempBase = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
    $tempRoot = Join-Path $tempBase "sd300-gui-self-test-$([Guid]::NewGuid().ToString('N'))"
    $stdout = Join-Path $tempRoot 'stdout.json'
    $stderr = Join-Path $tempRoot 'stderr.txt'
    $null = New-Item -ItemType Directory -Path $tempRoot -Force
    try {
        $process = Start-Process -FilePath $Gui -WorkingDirectory (Split-Path -Parent $Gui) `
            -ArgumentList @('--self-test', '--json') -Wait -PassThru -WindowStyle Hidden `
            -RedirectStandardOutput $stdout -RedirectStandardError $stderr
        return [pscustomobject]@{
            ExitCode = $process.ExitCode
            Lines = @(Get-Content -LiteralPath $stdout -ErrorAction SilentlyContinue)
            ErrorText = @(Get-Content -LiteralPath $stderr -ErrorAction SilentlyContinue) -join "`n"
            Context = $Context
        }
    }
    finally {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Set-ManagedFixture {
    $null = New-Item -ItemType Directory -Path (Split-Path -Parent $managedBinary) -Force
    $null = New-Item -ItemType Directory -Path (Split-Path -Parent $managedReceipt) -Force
    Copy-Item -LiteralPath $BuiltBinary -Destination $managedBinary -Force
    @{
        provider = @{ source = 'cargo-dist' }
        source = @{ app_name = 'sd300' }
        install_prefix = (Join-Path $env:USERPROFILE '.cargo')
        version = $Version
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $managedReceipt -Encoding utf8
}

function Assert-NativeInstall([string]$Binary, [string]$Channel) {
    if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) { throw "missing native binary: $Binary" }
    $reportedLines = @(& $Binary --version)
    $versionExitCode = $LASTEXITCODE
    $reported = $reportedLines | Select-Object -First 1
    if ($versionExitCode -ne 0 -or $reported -ne "sd300 $Version") {
        throw "unexpected version from $Binary`: $reported"
    }
    if (Test-Path -LiteralPath $managedBinary -PathType Leaf) {
        throw "fresh $Channel install left the managed/Cargo binary behind"
    }
    if (Test-Path -LiteralPath $managedReceipt -PathType Leaf) {
        throw "fresh $Channel install left the managed receipt behind"
    }

    $updateLines = @(& $Binary update --json)
    $updateExitCode = $LASTEXITCODE
    if ($updateExitCode -ne 0 -or $updateLines.Count -ne 1) {
        throw "$Channel update did not emit exactly one successful JSON object (exit $updateExitCode, lines $($updateLines.Count)): $($updateLines -join ' | ')"
    }
    $update = $updateLines[0] | ConvertFrom-Json
    if (-not $update.success -or $update.install_channel -ne $Channel) {
        throw "$Channel ownership was not preserved by update detection: $($updateLines[0])"
    }

    $snapshotLines = @(& $Binary snapshot --json)
    $snapshotExitCode = $LASTEXITCODE
    $snapshot = $snapshotLines | ConvertFrom-Json
    if ($snapshotExitCode -ne 0 -or $snapshot.target_os -ne 'windows' -or $snapshot.capabilities.Count -lt 10) {
        throw "$Channel diagnostic snapshot did not exercise the Windows collector set"
    }

    $gui = Join-Path (Split-Path -Parent (Split-Path -Parent $Binary)) 'app\sd300-gui.exe'
    if (-not (Test-Path -LiteralPath $gui -PathType Leaf)) {
        throw "$Channel GUI companion is missing: $gui"
    }
    $notices = Join-Path (Split-Path -Parent $gui) 'licenses'
    foreach ($notice in @(
        'PRODUCT-LICENSE.md',
        'IBM-PLEX-OFL-1.1.txt',
        'NATIVE-SDK-APACHE-2.0.txt'
    )) {
        $noticePath = Join-Path $notices $notice
        if (-not (Test-Path -LiteralPath $noticePath -PathType Leaf) -or
            (Get-Item -LiteralPath $noticePath).Length -eq 0) {
            throw "$Channel GUI companion is missing required notice: $noticePath"
        }
    }
    $guiResult = Invoke-GuiSelfTest $gui $Channel
    if ($guiResult.ExitCode -ne 0 -or $guiResult.Lines.Count -ne 1 -or
        -not ($guiResult.Lines[0] | ConvertFrom-Json).success) {
        throw "$Channel GUI companion failed its installed self-test (exit $($guiResult.ExitCode)): $($guiResult.ErrorText)"
    }
}

function Start-InstalledGui([string]$Root, [string]$Context) {
    $gui = Join-Path $Root 'app\sd300-gui.exe'
    $process = Start-Process -FilePath $gui -WorkingDirectory (Split-Path -Parent $gui) -PassThru
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        if ($process.HasExited) {
            throw "$Context GUI exited before the lifecycle test could begin (exit $($process.ExitCode))"
        }
        if (Get-Process -Id $process.Id -ErrorAction SilentlyContinue) { return $process }
        Start-Sleep -Milliseconds 100
    }
    throw "$Context GUI did not become observable"
}

function Assert-GuiStopped($Process, [string]$Context) {
    [void]$Process.WaitForExit(10000)
    if (-not $Process.HasExited) {
        throw "$Context did not stop the GUI through its authenticated lifecycle endpoint"
    }
}

function Remove-Inno([string]$Root) {
    $uninstaller = Get-ChildItem -LiteralPath $Root -Filter 'unins*.exe' -File -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($uninstaller) {
        Invoke-Checked $uninstaller.FullName @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    }
}

function Remove-MsiByName([string]$DisplayName) {
    $roots = @(
        'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )
    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        foreach ($entry in Get-ChildItem -LiteralPath $root) {
            $item = Get-ItemProperty -LiteralPath $entry.PSPath
            if ($item.DisplayName -eq $DisplayName -and $item.WindowsInstaller -eq 1) {
                Invoke-Checked 'msiexec.exe' @('/x', $entry.PSChildName, '/qn', '/norestart') @(0, 1605, 1614, 1641, 3010)
            }
        }
    }
}

$globalMsi = Join-Path $ArtifactDirectory 'sd300-windows-x64-global.msi'
$corporateMsi = Join-Path $ArtifactDirectory 'sd300-windows-x64-corporate.msi'
$globalExe = Join-Path $ArtifactDirectory 'sd300-windows-x64-global.exe'
$corporateExe = Join-Path $ArtifactDirectory 'sd300-windows-x64-corporate.exe'

try {
    Set-ManagedFixture
    Invoke-Checked 'msiexec.exe' @('/i', $globalMsi, '/qn', '/norestart') @(0, 1641, 3010)
    Assert-NativeInstall (Join-Path $globalRoot 'bin\sd300.exe') 'msi-global'
    $guiProcess = Start-InstalledGui $globalRoot 'Global MSI repair'
    Invoke-Checked 'msiexec.exe' @('/i', $globalMsi, '/qn', '/norestart', 'REINSTALL=ALL', 'REINSTALLMODE=vomus') @(0, 1641, 3010)
    Assert-GuiStopped $guiProcess 'Global MSI repair'

    $opposite = Invoke-Checked 'msiexec.exe' @('/i', $corporateMsi, '/qn', '/norestart') @(1603)
    if ($opposite -ne 1603 -or (Test-Path -LiteralPath (Join-Path $corporateRoot 'bin\sd300.exe'))) {
        throw 'Corporate MSI did not stop before mutation while Global was registered'
    }

    $guiProcess = Start-InstalledGui $globalRoot 'Global MSI-to-EXE transition'
    Invoke-Checked $globalExe @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-GuiStopped $guiProcess 'Global MSI-to-EXE transition'
    Assert-NativeInstall (Join-Path $globalRoot 'bin\sd300.exe') 'exe-global'
    $guiProcess = Start-InstalledGui $globalRoot 'Global EXE uninstall'
    Remove-Inno $globalRoot
    Assert-GuiStopped $guiProcess 'Global EXE uninstall'

    Set-ManagedFixture
    Invoke-Checked 'msiexec.exe' @('/i', $corporateMsi, '/qn', '/norestart') @(0, 1641, 3010)
    Assert-NativeInstall (Join-Path $corporateRoot 'bin\sd300.exe') 'msi-corporate'
    $guiProcess = Start-InstalledGui $corporateRoot 'Corporate MSI repair'
    Invoke-Checked 'msiexec.exe' @('/i', $corporateMsi, '/qn', '/norestart', 'REINSTALL=ALL', 'REINSTALLMODE=vomus') @(0, 1641, 3010)
    Assert-GuiStopped $guiProcess 'Corporate MSI repair'

    $guiProcess = Start-InstalledGui $corporateRoot 'Corporate MSI-to-EXE transition'
    Invoke-Checked $corporateExe @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-GuiStopped $guiProcess 'Corporate MSI-to-EXE transition'
    Assert-NativeInstall (Join-Path $corporateRoot 'bin\sd300.exe') 'exe-corporate'
} finally {
    Remove-Inno $globalRoot
    Remove-Inno $corporateRoot
    Remove-MsiByName 'SD-300 Global'
    Remove-MsiByName 'SD-300 Corporate'
    Remove-Item -LiteralPath $managedBinary, $managedReceipt -Force -ErrorAction SilentlyContinue
}
