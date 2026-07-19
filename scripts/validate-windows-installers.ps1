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
    if ($LASTEXITCODE -ne 0 -or $updateLines.Count -ne 1) {
        throw "$Channel update did not emit exactly one successful JSON object"
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

    $opposite = Invoke-Checked 'msiexec.exe' @('/i', $corporateMsi, '/qn', '/norestart') @(1603)
    if ($opposite -ne 1603 -or (Test-Path -LiteralPath (Join-Path $corporateRoot 'bin\sd300.exe'))) {
        throw 'Corporate MSI did not stop before mutation while Global was registered'
    }

    Invoke-Checked $globalExe @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-NativeInstall (Join-Path $globalRoot 'bin\sd300.exe') 'exe-global'
    Remove-Inno $globalRoot

    Set-ManagedFixture
    Invoke-Checked 'msiexec.exe' @('/i', $corporateMsi, '/qn', '/norestart') @(0, 1641, 3010)
    Assert-NativeInstall (Join-Path $corporateRoot 'bin\sd300.exe') 'msi-corporate'

    Invoke-Checked $corporateExe @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert-NativeInstall (Join-Path $corporateRoot 'bin\sd300.exe') 'exe-corporate'
} finally {
    Remove-Inno $globalRoot
    Remove-Inno $corporateRoot
    Remove-MsiByName 'SD-300 Global'
    Remove-MsiByName 'SD-300 Corporate'
    Remove-Item -LiteralPath $managedBinary, $managedReceipt -Force -ErrorAction SilentlyContinue
}
