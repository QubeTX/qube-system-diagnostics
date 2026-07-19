# SD-300 managed PowerShell installer.
#
# The release workflow renders the immutable tag/version placeholders, keeps the
# cargo-dist generated installer as sd300-dist-installer.ps1, and publishes this
# stable-name wrapper as sd300-cli-installer.ps1. A deliberately launched fresh
# wrapper is authoritative: after cargo-dist installs and verifies the managed
# PowerShell channel, recognized native MSI/Inno products are uninstalled.

param (
    [switch]$NoModifyPath,
    [switch]$Help
)

$ErrorActionPreference = 'Stop'
$InformationPreference = 'Continue'
$Sd300Tag = '@SD300_TAG@'
$Sd300Version = '@SD300_VERSION@'
$Sd300ReleaseBase = "https://github.com/QubeTX/qube-system-diagnostics/releases/download/$Sd300Tag"
$Sd300RecoveryUrl = 'https://github.com/QubeTX/qube-system-diagnostics/releases/latest'

if ($Help) {
    Write-Information 'SD-300 managed PowerShell installer'
    Write-Information 'Installs the latest managed CLI channel and safely supersedes recognized SD-300 MSI/EXE installs.'
    return
}

function Get-Sd300MsiProducts {
    if (-not ('Sd300.ManagedInstaller.NativeMsi' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
namespace Sd300.ManagedInstaller {
    public static class NativeMsi {
        [DllImport("msi.dll", CharSet = CharSet.Unicode)]
        public static extern uint MsiEnumRelatedProducts(
            string upgradeCode,
            uint reserved,
            uint productIndex,
            StringBuilder productCode);
    }
}
'@
    }

    $families = @(
        [pscustomobject]@{ UpgradeCode = '{A7C3E1D4-9F8B-4E2A-B6D1-3F5C8A0E7B94}'; Channel = 'msi-global'; Elevated = $true },
        [pscustomobject]@{ UpgradeCode = '{143F59B2-5D4B-4F6F-B258-BB44F9C50CC9}'; Channel = 'msi-corporate'; Elevated = $false }
    )
    $products = @()
    foreach ($family in $families) {
        for ($index = 0; ; $index++) {
            $productCode = New-Object System.Text.StringBuilder 39
            $result = [Sd300.ManagedInstaller.NativeMsi]::MsiEnumRelatedProducts(
                $family.UpgradeCode,
                0,
                [uint32]$index,
                $productCode
            )
            if ($result -eq 259) { break }
            if ($result -ne 0) {
                throw "MsiEnumRelatedProducts failed for $($family.Channel) with code $result"
            }
            $products += [pscustomobject]@{
                Kind = 'msi'
                Channel = $family.Channel
                Elevated = $family.Elevated
                ProductCode = $productCode.ToString()
                Uninstaller = 'msiexec.exe'
            }
        }
    }
    return $products
}

function ConvertFrom-Sd300UninstallString([string]$Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return $null }
    if ($Value -match '^\s*"([^"]+)"') { return $Matches[1] }
    return ($Value.Trim() -split '\s+', 2)[0]
}

function Get-Sd300InnoProducts {
    $families = @(
        [pscustomobject]@{
            Key = 'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{DC74D35F-CBF4-425F-B11E-E9EA87C13CA9}_is1'
            Channel = 'exe-global'; Elevated = $true; Root = (Join-Path $env:ProgramFiles 'sd300')
        },
        [pscustomobject]@{
            Key = 'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\{DC74D35F-CBF4-425F-B11E-E9EA87C13CA9}_is1'
            Channel = 'exe-global'; Elevated = $true; Root = (Join-Path $env:ProgramFiles 'sd300')
        },
        [pscustomobject]@{
            Key = 'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{ED209931-B5C0-43AE-89F6-83EE2C581653}_is1'
            Channel = 'exe-corporate'; Elevated = $false; Root = (Join-Path $env:LOCALAPPDATA 'Programs\sd300')
        }
    )
    $products = @()
    $seen = @{}
    foreach ($family in $families) {
        if (-not (Test-Path -LiteralPath $family.Key)) { continue }
        $entry = Get-ItemProperty -LiteralPath $family.Key
        $uninstaller = ConvertFrom-Sd300UninstallString ([string]$entry.UninstallString)
        if (-not $uninstaller) {
            throw "recognized $($family.Channel) registration has no uninstaller"
        }
        $full = [IO.Path]::GetFullPath($uninstaller)
        $root = [IO.Path]::GetFullPath($family.Root).TrimEnd('\') + '\'
        $leaf = [IO.Path]::GetFileName($full)
        if (-not $full.StartsWith($root, [StringComparison]::OrdinalIgnoreCase) -or
            $leaf -notmatch '^unins\d+\.exe$') {
            throw "recognized $($family.Channel) registration points outside its exact install root; refusing $full"
        }
        if (-not $seen.ContainsKey($full.ToLowerInvariant())) {
            $seen[$full.ToLowerInvariant()] = $true
            $products += [pscustomobject]@{
                Kind = 'inno'
                Channel = $family.Channel
                Elevated = $family.Elevated
                ProductCode = $null
                Uninstaller = $full
            }
        }
    }
    return $products
}

function Get-Sd300NativeProducts {
    $products = @()
    $products += @(Get-Sd300MsiProducts)
    $products += @(Get-Sd300InnoProducts)
    return $products
}

function Invoke-Sd300Process([string]$FilePath, [string[]]$Arguments, [bool]$Elevated) {
    $params = @{
        FilePath = $FilePath
        ArgumentList = $Arguments
        Wait = $true
        PassThru = $true
        WindowStyle = 'Hidden'
    }
    if ($Elevated) { $params.Verb = 'RunAs' }
    return Start-Process @params
}

function Remove-Sd300NativeProduct($Product) {
    Write-Information "Switching SD-300 ownership from $($Product.Channel) to powershell-installer..."
    if ($Product.Kind -eq 'msi') {
        $process = Invoke-Sd300Process 'msiexec.exe' @('/x', $Product.ProductCode, '/passive', '/norestart') $Product.Elevated
        if ($process.ExitCode -notin @(0, 1605, 1614, 1641, 3010)) {
            throw "$($Product.Channel) uninstall exited with Windows Installer code $($process.ExitCode)"
        }
    } else {
        $process = Invoke-Sd300Process $Product.Uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') $Product.Elevated
        if ($process.ExitCode -ne 0) {
            throw "$($Product.Channel) uninstall exited with code $($process.ExitCode)"
        }
    }
}

function Get-Sd300ReceiptPath {
    $root = if ($env:XDG_CONFIG_HOME) { $env:XDG_CONFIG_HOME } else { $env:LOCALAPPDATA }
    if (-not $root) { throw 'LOCALAPPDATA is unavailable; cannot verify the managed installer receipt' }
    return Join-Path $root 'sd300\sd300-receipt.json'
}

function Get-Sd300InstallPrefix {
    if ($env:SD300_INSTALL_DIR) { return [IO.Path]::GetFullPath($env:SD300_INSTALL_DIR) }
    if ($env:CARGO_DIST_FORCE_INSTALL_DIR) { return [IO.Path]::GetFullPath($env:CARGO_DIST_FORCE_INSTALL_DIR) }
    if ($env:CARGO_HOME) { return [IO.Path]::GetFullPath($env:CARGO_HOME) }
    if (-not $env:USERPROFILE) { throw 'USERPROFILE is unavailable; cannot resolve the managed install prefix' }
    return [IO.Path]::GetFullPath((Join-Path $env:USERPROFILE '.cargo'))
}

function Save-Sd300ManagedState([string]$BackupRoot) {
    $receiptPath = Get-Sd300ReceiptPath
    $priorPrefix = $null
    $receiptExisted = Test-Path -LiteralPath $receiptPath -PathType Leaf
    if ($receiptExisted) {
        $priorReceipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
        if ($priorReceipt.provider.source -ne 'cargo-dist' -or
            $priorReceipt.source.app_name -ne 'sd300' -or
            [string]::IsNullOrWhiteSpace([string]$priorReceipt.install_prefix)) {
            throw 'the existing SD-300 managed receipt is ambiguous; preserving it'
        }
        $priorPrefix = [IO.Path]::GetFullPath([string]$priorReceipt.install_prefix)
        Copy-Item -LiteralPath $receiptPath -Destination (Join-Path $BackupRoot 'receipt.json')
    }

    $binaryPaths = @((Join-Path (Get-Sd300InstallPrefix) 'bin\sd300.exe'))
    if ($priorPrefix) { $binaryPaths += (Join-Path $priorPrefix 'bin\sd300.exe') }
    $seen = @{}
    $binaries = @()
    foreach ($candidate in $binaryPaths) {
        $full = [IO.Path]::GetFullPath($candidate)
        $key = $full.ToLowerInvariant()
        if ($seen.ContainsKey($key)) { continue }
        $seen[$key] = $true
        $existed = Test-Path -LiteralPath $full -PathType Leaf
        $backup = $null
        if ($existed) {
            $backup = Join-Path $BackupRoot ("binary-$($binaries.Count).exe")
            Copy-Item -LiteralPath $full -Destination $backup
        }
        $binaries += [pscustomobject]@{ Path = $full; Existed = $existed; Backup = $backup }
    }

    return [pscustomobject]@{
        ReceiptPath = $receiptPath
        ReceiptExisted = $receiptExisted
        ReceiptBackup = (Join-Path $BackupRoot 'receipt.json')
        PriorPrefix = $priorPrefix
        Binaries = $binaries
    }
}

function Restore-Sd300ManagedState($State) {
    foreach ($binary in $State.Binaries) {
        if ($binary.Existed) {
            $null = New-Item -ItemType Directory -Path (Split-Path -Parent $binary.Path) -Force
            Copy-Item -LiteralPath $binary.Backup -Destination $binary.Path -Force
        } else {
            Remove-Item -LiteralPath $binary.Path -Force -ErrorAction SilentlyContinue
        }
    }
    if ($State.ReceiptExisted) {
        $null = New-Item -ItemType Directory -Path (Split-Path -Parent $State.ReceiptPath) -Force
        Copy-Item -LiteralPath $State.ReceiptBackup -Destination $State.ReceiptPath -Force
    } else {
        Remove-Item -LiteralPath $State.ReceiptPath -Force -ErrorAction SilentlyContinue
    }
}

function Get-Sd300ManagedBinary {
    $receiptPath = Get-Sd300ReceiptPath
    if (-not (Test-Path -LiteralPath $receiptPath)) {
        throw "managed installer receipt is missing: $receiptPath"
    }
    $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    if ($receipt.provider.source -ne 'cargo-dist' -or
        $receipt.source.app_name -ne 'sd300' -or
        $receipt.version -ne $Sd300Version) {
        throw 'managed installer receipt does not identify the exact SD-300 release'
    }
    $binary = Join-Path ([string]$receipt.install_prefix) 'bin\sd300.exe'
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "managed SD-300 binary is missing: $binary"
    }
    $reportedLines = @(& $binary --version)
    $versionExitCode = $LASTEXITCODE
    $reported = $reportedLines | Select-Object -First 1
    if ($versionExitCode -ne 0 -or $reported -ne "sd300 $Sd300Version") {
        throw "managed SD-300 binary did not report the expected version $Sd300Version"
    }
    return $binary
}

function Get-Sd300ReleaseFile([string]$Name, [string]$Destination, [hashtable]$Headers) {
    if ($env:GITHUB_ACTIONS -eq 'true' -and $env:SD300_CI_RELEASE_ASSET_DIR) {
        $root = [IO.Path]::GetFullPath($env:SD300_CI_RELEASE_ASSET_DIR)
        $source = Join-Path $root $Name
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "GitHub Actions candidate asset is missing: $source"
        }
        Copy-Item -LiteralPath $source -Destination $Destination
        return
    }
    Invoke-WebRequest -UseBasicParsing -Headers $Headers -Uri "$Sd300ReleaseBase/$Name" -OutFile $Destination
}

function Assert-Sd300Sha256([string]$Asset, [string]$Sidecar) {
    $expected = ((Get-Content -LiteralPath $Sidecar -Raw) -split '\s+' |
        Where-Object { $_ -match '^[0-9a-fA-F]{64}$' } |
        Select-Object -First 1)
    if (-not $expected) { throw 'managed installer SHA-256 sidecar is invalid' }
    $actual = (Get-FileHash -LiteralPath $Asset -Algorithm SHA256).Hash
    if ($actual -ne $expected) { throw 'managed installer SHA-256 verification failed' }
}

if ($env:SD300_MANAGED_INSTALLER_TEST_ONLY -eq '1') {
    return
}

$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("sd300-managed-install-" + [guid]::NewGuid().ToString('N'))
$managedState = $null
$transactionStarted = $false
$committed = $false
try {
    $null = New-Item -ItemType Directory -Path $tempRoot -Force
    $native = @(Get-Sd300NativeProducts)
    $managedState = Save-Sd300ManagedState $tempRoot
    $distInstaller = Join-Path $tempRoot 'sd300-dist-installer.ps1'
    [Net.ServicePointManager]::SecurityProtocol =
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    $headers = @{}
    $token = if ($env:SD300_GITHUB_TOKEN) {
        $env:SD300_GITHUB_TOKEN
    } elseif ($env:GITHUB_TOKEN) {
        $env:GITHUB_TOKEN
    } elseif ($env:GH_TOKEN) {
        $env:GH_TOKEN
    } else {
        $null
    }
    if ($token) { $headers.Authorization = "Bearer $token" }
    $distSidecar = "$distInstaller.sha256"
    Get-Sd300ReleaseFile 'sd300-dist-installer.ps1' $distInstaller $headers
    Get-Sd300ReleaseFile 'sd300-dist-installer.ps1.sha256' $distSidecar $headers
    Assert-Sd300Sha256 $distInstaller $distSidecar

    $transactionStarted = $true
    $launcher = if ($PSVersionTable.PSEdition -eq 'Core') {
        Join-Path $PSHOME 'pwsh.exe'
    } else {
        Join-Path $PSHOME 'powershell.exe'
    }
    $childArgs = @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', $distInstaller)
    if ($NoModifyPath) { $childArgs += '-NoModifyPath' }
    & $launcher @childArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo-dist installation exited with code $LASTEXITCODE"
    }

    $binary = Get-Sd300ManagedBinary
    foreach ($product in $native) {
        Remove-Sd300NativeProduct $product
    }
    $remaining = @(Get-Sd300NativeProducts)
    if ($remaining.Count -ne 0) {
        throw "native installer takeover is incomplete: $($remaining.Channel -join ', ') remains registered"
    }

    if ($native.Count -gt 0) {
        $markerKey = 'Registry::HKEY_CURRENT_USER\Software\SD300'
        foreach ($name in @('InstallSource', 'InstallSourceGlobal', 'InstallSourceCorporate')) {
            Remove-ItemProperty -LiteralPath $markerKey -Name $name -ErrorAction SilentlyContinue
        }
    }
    if ($managedState.PriorPrefix) {
        $priorBinary = Join-Path $managedState.PriorPrefix 'bin\sd300.exe'
        $sameBinary = [IO.Path]::GetFullPath($priorBinary).Equals(
            [IO.Path]::GetFullPath($binary),
            [StringComparison]::OrdinalIgnoreCase
        )
        if (-not $sameBinary -and (Test-Path -LiteralPath $priorBinary -PathType Leaf)) {
            Remove-Item -LiteralPath $priorBinary -Force -ErrorAction Stop
        }
    }
    $committed = $true
    Write-Information "SD-300 $Sd300Version is installed through the managed PowerShell channel: $binary"
} catch {
    $failure = $_.Exception.Message
    if ($transactionStarted -and -not $committed -and $managedState) {
        try {
            Restore-Sd300ManagedState $managedState
        } catch {
            $failure += "; restoring the prior managed/Cargo path also failed: $($_.Exception.Message)"
        }
    }
    [Console]::Error.WriteLine("SD-300 managed install failed safely: $failure")
    [Console]::Error.WriteLine("Download a fresh installer: $Sd300RecoveryUrl")
    exit 1
} finally {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
