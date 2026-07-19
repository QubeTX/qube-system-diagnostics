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
$managedConfigRoot = if ($env:XDG_CONFIG_HOME) { $env:XDG_CONFIG_HOME } else { $env:LOCALAPPDATA }
$managedReceipt = Join-Path $managedConfigRoot 'sd300\sd300-receipt.json'

function Invoke-Checked([string]$FilePath, [string[]]$Arguments, [int[]]$Allowed = @(0)) {
    $process = Start-Process -FilePath $FilePath -ArgumentList $Arguments -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -notin $Allowed) {
        throw "$FilePath exited with $($process.ExitCode); expected $($Allowed -join ', ')"
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

function Assert-ManagedUninstall([string]$Binary, [string]$Channel) {
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
            -not (Test-Path -LiteralPath (Split-Path -Parent $managedReceipt))) { return }
        Start-Sleep -Milliseconds 100
    }
    throw "$Channel uninstall left its binary, receipt, or receipt directory"
}

function Assert-NativeUninstall(
    [string]$Binary,
    [string]$Channel,
    [string]$Root,
    [string]$DisplayName
) {
    $binDir = Split-Path -Parent $Binary
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
        $marker = Get-ItemPropertyValue -Path 'Registry::HKEY_CURRENT_USER\Software\SD300' -Name InstallSource -ErrorAction SilentlyContinue
        $ownsPath = (Test-PathListContains ([Environment]::GetEnvironmentVariable('Path', 'User')) $binDir) -or
                    (Test-PathListContains ([Environment]::GetEnvironmentVariable('Path', 'Machine')) $binDir)
        if (-not (Test-Path -LiteralPath $Root) -and
            @(Get-Sd300Registrations $DisplayName).Count -eq 0 -and
            -not $marker -and
            -not $ownsPath -and
            -not (Test-Path -LiteralPath (Split-Path -Parent $managedReceipt))) { return }
        Start-Sleep -Milliseconds 100
    }
    throw "$Channel uninstall left payload, registration, marker, PATH ownership, or an empty managed receipt directory"
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

    Invoke-Checked 'msiexec.exe' @('/i', $priorCorporateMsi, '/qn', '/norestart') @(0, 1641, 3010)
    Assert-Update (Join-Path $corporateRoot 'bin\sd300.exe') 'msi-corporate' $corporateRoot
    Assert-NativeUninstall (Join-Path $corporateRoot 'bin\sd300.exe') 'msi-corporate' $corporateRoot 'SD-300 Corporate'

    Invoke-Checked $priorGlobalExe @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-Update (Join-Path $globalRoot 'bin\sd300.exe') 'exe-global' $globalRoot
    Assert-NativeUninstall (Join-Path $globalRoot 'bin\sd300.exe') 'exe-global' $globalRoot 'SD-300 Global'

    Invoke-Checked $priorCorporateExe @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-Update (Join-Path $corporateRoot 'bin\sd300.exe') 'exe-corporate' $corporateRoot
    Assert-NativeUninstall (Join-Path $corporateRoot 'bin\sd300.exe') 'exe-corporate' $corporateRoot 'SD-300 Corporate'
} finally {
    Remove-Inno $globalRoot
    Remove-Inno $corporateRoot
    Remove-MsiByName 'SD-300 Global'
    Remove-MsiByName 'SD-300 Corporate'
    Remove-Item -LiteralPath $managedBinary, $managedReceipt -Force -ErrorAction SilentlyContinue
    Remove-Item Env:SD300_CI_RELEASE_TAG, Env:SD300_CI_RELEASE_ASSET_DIR, Env:SD300_CI_MANAGED_BINARY -ErrorAction SilentlyContinue
}
