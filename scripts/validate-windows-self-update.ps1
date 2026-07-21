param(
    [Parameter(Mandatory = $true)][string]$CandidateAssets,
    [Parameter(Mandatory = $true)][string]$CandidateVersion,
    [Parameter(Mandatory = $true)][string]$CandidateBinary,
    [Parameter(Mandatory = $true)][string]$PriorArtifacts,
    [Parameter(Mandatory = $true)][string]$PriorVersion,
    [Parameter(Mandatory = $true)][string]$PriorBinary
)

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') {
    throw 'This destructive transition matrix is restricted to an ephemeral GitHub Actions runner.'
}

$CandidateAssets = [IO.Path]::GetFullPath($CandidateAssets)
$CandidateBinary = [IO.Path]::GetFullPath($CandidateBinary)
$PriorArtifacts = [IO.Path]::GetFullPath($PriorArtifacts)
$PriorBinary = [IO.Path]::GetFullPath($PriorBinary)
$env:SD300_CI_RELEASE_TAG = "v$CandidateVersion"
$env:SD300_CI_RELEASE_ASSET_DIR = $CandidateAssets
$env:SD300_CI_MANAGED_BINARY = $CandidateBinary

$globalRoot = Join-Path $env:ProgramFiles 'sd300'
$corporateRoot = Join-Path $env:LOCALAPPDATA 'Programs\sd300'
$managedRoot = Join-Path $env:USERPROFILE '.cargo'
$managedBinary = Join-Path $managedRoot 'bin\sd300.exe'
$managedGuiRoot = Join-Path $env:LOCALAPPDATA 'Programs\SD-300'
$managedGuiShortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\SD-300.lnk'
$managedGuiRegistration = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\SD-300-Managed'
$guiStateRoot = Join-Path $env:APPDATA 'SD-300'
$guiRunKey = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run'
$managedConfigRoot = if ($env:XDG_CONFIG_HOME) { $env:XDG_CONFIG_HOME } else { $env:LOCALAPPDATA }
$managedReceipt = Join-Path $managedConfigRoot 'sd300\sd300-receipt.json'

function Invoke-Checked([string]$FilePath, [string[]]$Arguments, [int[]]$Allowed = @(0)) {
    $process = Start-Process -FilePath $FilePath -ArgumentList $Arguments -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -notin $Allowed) {
        throw "$FilePath exited with $($process.ExitCode); expected $($Allowed -join ', ')"
    }
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

function Remove-MsiByName([string]$DisplayName) {
    foreach ($root in @(
        'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        foreach ($entry in Get-ChildItem -LiteralPath $root) {
            $item = Get-ItemProperty -LiteralPath $entry.PSPath
            if ($item.DisplayName -eq $DisplayName -and $item.WindowsInstaller -eq 1) {
                Invoke-Checked 'msiexec.exe' @('/x', $entry.PSChildName, '/qn', '/norestart') @(0, 1605, 1614, 1641, 3010)
            }
        }
    }
}

function Remove-Inno([string]$Root) {
    $uninstaller = Get-ChildItem -LiteralPath $Root -Filter 'unins*.exe' -File -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($uninstaller) {
        Invoke-Checked $uninstaller.FullName @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    }
}

function Wait-ForCleanup([string]$Root) {
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
        $backups = @(Get-ChildItem -LiteralPath (Join-Path $Root 'bin') -Filter '.sd300-update-backup-*.exe' -File -ErrorAction SilentlyContinue)
        if ($backups.Count -eq 0) { return }
        Start-Sleep -Milliseconds 100
    }
    throw "live-image backup cleanup did not converge under $Root"
}

function Test-PathListContains([string]$Value, [string]$Expected) {
    if (-not $Value) { return $false }
    $normalized = [IO.Path]::GetFullPath($Expected).TrimEnd('\')
    foreach ($entry in $Value -split ';') {
        if (-not $entry.Trim()) { continue }
        try { $candidate = [IO.Path]::GetFullPath($entry.Trim()).TrimEnd('\') } catch { continue }
        if ($candidate.Equals($normalized, [StringComparison]::OrdinalIgnoreCase)) { return $true }
    }
    return $false
}

function Get-Sd300Registrations([string]$DisplayName) {
    $records = @()
    foreach ($root in @(
        'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        $records += @(Get-ChildItem -LiteralPath $root | Where-Object {
            (Get-ItemProperty -LiteralPath $_.PSPath).DisplayName -eq $DisplayName
        })
    }
    return @($records)
}

function Assert-GuiCompanion([string]$Root, [string]$Channel) {
    $guiRoot = if ($Channel -eq 'powershell-installer') { $managedGuiRoot } else { $Root }
    $gui = Join-Path $guiRoot 'app\sd300-gui.exe'
    $engine = Join-Path $guiRoot 'app\sd300_engine.dll'
    if (-not (Test-Path -LiteralPath $gui -PathType Leaf) -or
        -not (Test-Path -LiteralPath $engine -PathType Leaf)) {
        throw "$Channel update did not install a complete GUI companion under $guiRoot"
    }
    foreach ($notice in @(
        'PRODUCT-LICENSE.md',
        'IBM-PLEX-OFL-1.1.txt',
        'NATIVE-SDK-APACHE-2.0.txt'
    )) {
        $noticePath = Join-Path $guiRoot "app\licenses\$notice"
        if (-not (Test-Path -LiteralPath $noticePath -PathType Leaf) -or
            (Get-Item -LiteralPath $noticePath).Length -eq 0) {
            throw "$Channel update did not install required GUI notice $noticePath"
        }
    }
    $selfTest = Invoke-GuiSelfTest $gui $Channel
    if ($selfTest.ExitCode -ne 0 -or $selfTest.Lines.Count -ne 1) {
        throw "$Channel GUI self-test did not return one successful JSON object (exit $($selfTest.ExitCode), lines $($selfTest.Lines.Count)): $($selfTest.ErrorText)"
    }
    $result = $selfTest.Lines[0] | ConvertFrom-Json
    if (-not $result.success -or $result.product -ne 'SD-300' -or
        $result.product_version -ne $CandidateVersion -or
        $result.abi_version -ne 1 -or $result.engine_schema_version -ne 1 -or
        $result.target_os -ne 'windows' -or $result.target_arch -ne 'x86_64') {
        throw "$Channel GUI self-test reported an incompatible product, version, ABI, schema, or target"
    }
    if ($Channel -eq 'powershell-installer') {
        $ownerPath = Join-Path $managedGuiRoot '.sd300-managed-owner.json'
        $owner = Get-Content -LiteralPath $ownerPath -Raw | ConvertFrom-Json
        if ($owner.schema -ne 1 -or $owner.product -ne 'SD-300' -or
            $owner.version -ne $CandidateVersion -or $owner.owner -ne 'powershell-installer' -or
            -not (Test-Path -LiteralPath $managedGuiShortcut -PathType Leaf) -or
            -not (Test-Path -LiteralPath $managedGuiRegistration)) {
            throw 'managed PowerShell update did not install its proven GUI ownership and discovery integrations'
        }
    }
}

function Assert-CurrentNoop([string]$Binary, [string]$Channel) {
    $lines = @(& $Binary update --json)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0 -or $lines.Count -ne 1) {
        throw "$Channel current-version update did not return one successful JSON object (exit $exitCode, lines $($lines.Count))"
    }
    $result = $lines[0] | ConvertFrom-Json
    if (-not $result.success -or $result.action -ne 'update' -or
        $result.current_version -ne $CandidateVersion -or
        $result.target_version -ne $CandidateVersion -or
        $result.install_channel -ne $Channel -or $result.strategy -ne 'current' -or
        $result.message -ne "SD-300 $CandidateVersion is already current") {
        throw "$Channel complete-current update was not the preserved no-op: $($lines[0])"
    }
}

function Assert-SameVersionGuiRepair([string]$Binary, [string]$Channel, [string]$Root) {
    $guiRoot = if ($Channel -eq 'powershell-installer') { $managedGuiRoot } else { $Root }
    Remove-Item -LiteralPath (Join-Path $guiRoot 'app\sd300_engine.dll') -Force
    $lines = @(& $Binary update --json)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0 -or $lines.Count -ne 1) {
        throw "$Channel same-version repair did not return one successful JSON object (exit $exitCode, lines $($lines.Count)): $($lines -join ' | ')"
    }
    $result = $lines[0] | ConvertFrom-Json
    if (-not $result.success -or $result.action -ne 'update' -or
        $result.current_version -ne $CandidateVersion -or
        $result.target_version -ne $CandidateVersion -or
        $result.install_channel -ne $Channel -or $result.strategy -ne 'same-version-repair') {
        throw "$Channel missing-GUI update did not use the same-version repair path: $($lines[0])"
    }
    Assert-GuiCompanion $Root $Channel
}

function Set-GuiUninstallFixture([string]$Channel) {
    $null = New-Item -ItemType Directory -Path (Join-Path $guiStateRoot 'reports') -Force
    Set-Content -LiteralPath (Join-Path $guiStateRoot 'settings.json') -Value '{"schema_version":1}' -Encoding utf8
    Set-Content -LiteralPath (Join-Path $guiStateRoot 'settings.corrupt-test.json') -Value '{}' -Encoding utf8
    Set-Content -LiteralPath (Join-Path $guiStateRoot '.settings-test.tmp') -Value '{}' -Encoding utf8
    $export = Join-Path $guiStateRoot "reports\$Channel-user-export.json"
    Set-Content -LiteralPath $export -Value '{"preserve":true}' -Encoding utf8
    $null = New-Item -Path $guiRunKey -Force
    New-ItemProperty -Path $guiRunKey -Name 'SD-300' `
        -Value ('"' + $managedGuiRoot + '\app\sd300-gui.exe" --startup') `
        -PropertyType String -Force | Out-Null
    return $export
}

function Assert-GuiUninstallState([string]$Export, [string]$Channel) {
    foreach ($owned in @('settings.json', 'settings.corrupt-test.json', '.settings-test.tmp')) {
        if (Test-Path -LiteralPath (Join-Path $guiStateRoot $owned)) {
            throw "$Channel uninstall left owned GUI state: $owned"
        }
    }
    if ((Get-ItemProperty -LiteralPath $guiRunKey -Name 'SD-300' -ErrorAction SilentlyContinue).'SD-300') {
        throw "$Channel uninstall left its launch-at-login registration"
    }
    if (-not (Test-Path -LiteralPath $Export -PathType Leaf)) {
        throw "$Channel uninstall removed a user-exported report"
    }
    Remove-Item -LiteralPath $Export -Force
    Remove-Item -LiteralPath (Split-Path -Parent $Export) -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $guiStateRoot -Force -ErrorAction SilentlyContinue
}

function Assert-ManagedUninstall([string]$Binary, [string]$Channel) {
    $export = Set-GuiUninstallFixture $Channel
    $lines = @(& $Binary uninstall --json)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0 -or $lines.Count -ne 1) {
        throw "$Channel uninstall did not return one successful JSON object (exit $exitCode, lines $($lines.Count)): $($lines -join ' | ')"
    }
    $result = $lines[0] | ConvertFrom-Json
    if (-not $result.success -or $result.install_channel -ne $Channel) {
        throw "$Channel uninstall reported the wrong owner or outcome: $($lines[0])"
    }
    for ($attempt = 0; $attempt -lt 600; $attempt++) {
        if (-not (Test-Path -LiteralPath $managedBinary) -and
            -not (Test-Path -LiteralPath $managedReceipt) -and
            -not (Test-Path -LiteralPath (Split-Path -Parent $managedReceipt)) -and
            -not (Test-Path -LiteralPath $managedGuiRoot) -and
            -not (Test-Path -LiteralPath $managedGuiShortcut) -and
            -not (Test-Path -LiteralPath $managedGuiRegistration)) {
            Assert-GuiUninstallState $export $Channel
            return
        }
        Start-Sleep -Milliseconds 100
    }
    throw "$Channel uninstall left its CLI, GUI, receipt, ownership registration, or shortcut"
}

function Assert-NativeUninstall(
    [string]$Binary,
    [string]$Channel,
    [string]$Root,
    [string]$DisplayName
) {
    $binDir = Split-Path -Parent $Binary
    $export = Set-GuiUninstallFixture $Channel
    $lines = @(& $Binary uninstall --json)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0 -or $lines.Count -ne 1) {
        throw "$Channel uninstall did not return one successful JSON object (exit $exitCode, lines $($lines.Count)): $($lines -join ' | ')"
    }
    $result = $lines[0] | ConvertFrom-Json
    if (-not $result.success -or $result.install_channel -ne $Channel) {
        throw "$Channel uninstall reported the wrong owner or outcome: $($lines[0])"
    }
    for ($attempt = 0; $attempt -lt 1800; $attempt++) {
        $marker = (Get-ItemProperty -LiteralPath 'Registry::HKEY_CURRENT_USER\Software\SD300' -ErrorAction SilentlyContinue).InstallSource
        $ownsPath = (Test-PathListContains ([Environment]::GetEnvironmentVariable('Path', 'User')) $binDir) -or
                    (Test-PathListContains ([Environment]::GetEnvironmentVariable('Path', 'Machine')) $binDir)
        if (-not (Test-Path -LiteralPath $Root) -and
            @(Get-Sd300Registrations $DisplayName).Count -eq 0 -and
            -not $marker -and
            -not $ownsPath -and
            -not (Test-Path -LiteralPath (Split-Path -Parent $managedReceipt))) {
            Assert-GuiUninstallState $export $Channel
            return
        }
        Start-Sleep -Milliseconds 100
    }
    throw "$Channel uninstall left payload, registration, marker, PATH ownership, or an empty managed receipt directory"
}

function Assert-ManagedTakeover(
    [string]$Binary,
    [string]$NativeChannel,
    [string]$Root,
    [string]$DisplayName
) {
    $binDir = Split-Path -Parent $Binary
    $lines = @(& $Binary install --json)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0 -or $lines.Count -ne 1) {
        throw "$NativeChannel managed takeover did not return one successful JSON object (exit $exitCode, lines $($lines.Count)): $($lines -join ' | ')"
    }
    $result = $lines[0] | ConvertFrom-Json
    if (-not $result.success -or
        $result.install_channel -ne 'powershell-installer' -or
        $result.target_version -ne $CandidateVersion) {
        throw "$NativeChannel managed takeover reported the wrong owner or outcome: $($lines[0])"
    }
    for ($attempt = 0; $attempt -lt 1800; $attempt++) {
        $marker = (Get-ItemProperty -LiteralPath 'Registry::HKEY_CURRENT_USER\Software\SD300' -ErrorAction SilentlyContinue).InstallSource
        $ownsPath = (Test-PathListContains ([Environment]::GetEnvironmentVariable('Path', 'User')) $binDir) -or
                    (Test-PathListContains ([Environment]::GetEnvironmentVariable('Path', 'Machine')) $binDir)
        if (-not (Test-Path -LiteralPath $Root) -and
            @(Get-Sd300Registrations $DisplayName).Count -eq 0 -and
            -not $marker -and
            -not $ownsPath -and
            (Test-Path -LiteralPath $managedBinary) -and
            (Test-Path -LiteralPath $managedReceipt)) {
            $receipt = Get-Content -LiteralPath $managedReceipt -Raw | ConvertFrom-Json
            $reported = @(& $managedBinary --version) | Select-Object -First 1
            if ($receipt.version -ne $CandidateVersion -or $reported -ne "sd300 $CandidateVersion") {
                throw "$NativeChannel takeover installed an unexpected managed version"
            }
            return
        }
        Start-Sleep -Milliseconds 100
    }
    throw "$NativeChannel takeover left native payload, registration, marker, or PATH ownership"
}

function Assert-Update([string]$Binary, [string]$Channel, [string]$Root) {
    $lines = @(& $Binary update --json)
    $updateExitCode = $LASTEXITCODE
    if ($updateExitCode -ne 0 -or $lines.Count -ne 1) {
        throw "$Channel update did not return exactly one successful JSON object (exit $updateExitCode, lines $($lines.Count)): $($lines -join ' | ')"
    }
    $result = $lines[0] | ConvertFrom-Json
    if (-not $result.success -or $result.install_channel -ne $Channel -or $result.target_version -ne $CandidateVersion) {
        throw "$Channel update changed ownership or version unexpectedly: $($lines[0])"
    }
    $reportedLines = @(& $Binary --version)
    $versionExitCode = $LASTEXITCODE
    $reported = $reportedLines | Select-Object -First 1
    if ($versionExitCode -ne 0 -or $reported -ne "sd300 $CandidateVersion") {
        throw "$Channel replacement reports an unexpected version: $reported"
    }
    Wait-ForCleanup $Root
    Assert-GuiCompanion $Root $Channel
    Assert-CurrentNoop $Binary $Channel
    Assert-SameVersionGuiRepair $Binary $Channel $Root
    Assert-CurrentNoop $Binary $Channel
}

function Set-ManagedPrior {
    $null = New-Item -ItemType Directory -Path (Split-Path -Parent $managedBinary) -Force
    $null = New-Item -ItemType Directory -Path (Split-Path -Parent $managedReceipt) -Force
    Copy-Item -LiteralPath $PriorBinary -Destination $managedBinary -Force
    @{
        provider = @{ source = 'cargo-dist' }
        source = @{ app_name = 'sd300' }
        install_prefix = $managedRoot
        version = $PriorVersion
    } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $managedReceipt -Encoding utf8
}

$priorGlobalMsi = Join-Path $PriorArtifacts 'sd300-prior-global.msi'
$priorCorporateMsi = Join-Path $PriorArtifacts 'sd300-prior-corporate.msi'
$priorGlobalExe = Join-Path $PriorArtifacts 'sd300-prior-global.exe'
$priorCorporateExe = Join-Path $PriorArtifacts 'sd300-prior-corporate.exe'

try {
    Set-ManagedPrior
    Assert-Update $managedBinary 'powershell-installer' $managedRoot
    Assert-ManagedUninstall $managedBinary 'powershell-installer'

    Invoke-Checked 'msiexec.exe' @('/i', $priorGlobalMsi, '/qn', '/norestart') @(0, 1641, 3010)
    Assert-Update (Join-Path $globalRoot 'bin\sd300.exe') 'msi-global' $globalRoot
    Assert-NativeUninstall (Join-Path $globalRoot 'bin\sd300.exe') 'msi-global' $globalRoot 'SD-300 Global'
    Invoke-Checked 'msiexec.exe' @('/i', (Join-Path $CandidateAssets 'sd300-windows-x64-global.msi'), '/qn', '/norestart') @(0, 1641, 3010)
    Assert-ManagedTakeover (Join-Path $globalRoot 'bin\sd300.exe') 'msi-global' $globalRoot 'SD-300 Global'
    Assert-ManagedUninstall $managedBinary 'powershell-installer'

    Invoke-Checked 'msiexec.exe' @('/i', $priorCorporateMsi, '/qn', '/norestart') @(0, 1641, 3010)
    Assert-Update (Join-Path $corporateRoot 'bin\sd300.exe') 'msi-corporate' $corporateRoot
    Assert-NativeUninstall (Join-Path $corporateRoot 'bin\sd300.exe') 'msi-corporate' $corporateRoot 'SD-300 Corporate'
    Invoke-Checked 'msiexec.exe' @('/i', (Join-Path $CandidateAssets 'sd300-windows-x64-corporate.msi'), '/qn', '/norestart') @(0, 1641, 3010)
    Assert-ManagedTakeover (Join-Path $corporateRoot 'bin\sd300.exe') 'msi-corporate' $corporateRoot 'SD-300 Corporate'
    Assert-ManagedUninstall $managedBinary 'powershell-installer'

    Invoke-Checked $priorGlobalExe @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-Update (Join-Path $globalRoot 'bin\sd300.exe') 'exe-global' $globalRoot
    Assert-NativeUninstall (Join-Path $globalRoot 'bin\sd300.exe') 'exe-global' $globalRoot 'SD-300 Global'
    Invoke-Checked (Join-Path $CandidateAssets 'sd300-windows-x64-global.exe') @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-ManagedTakeover (Join-Path $globalRoot 'bin\sd300.exe') 'exe-global' $globalRoot 'SD-300 Global'
    Assert-ManagedUninstall $managedBinary 'powershell-installer'

    Invoke-Checked $priorCorporateExe @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-Update (Join-Path $corporateRoot 'bin\sd300.exe') 'exe-corporate' $corporateRoot
    Assert-NativeUninstall (Join-Path $corporateRoot 'bin\sd300.exe') 'exe-corporate' $corporateRoot 'SD-300 Corporate'
    Invoke-Checked (Join-Path $CandidateAssets 'sd300-windows-x64-corporate.exe') @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-ManagedTakeover (Join-Path $corporateRoot 'bin\sd300.exe') 'exe-corporate' $corporateRoot 'SD-300 Corporate'
    Assert-ManagedUninstall $managedBinary 'powershell-installer'
} finally {
    Remove-Inno $globalRoot
    Remove-Inno $corporateRoot
    Remove-MsiByName 'SD-300 Global'
    Remove-MsiByName 'SD-300 Corporate'
    Remove-Item -LiteralPath $managedBinary, $managedReceipt -Force -ErrorAction SilentlyContinue
    Remove-Item Env:SD300_CI_RELEASE_TAG, Env:SD300_CI_RELEASE_ASSET_DIR, Env:SD300_CI_MANAGED_BINARY -ErrorAction SilentlyContinue
}
