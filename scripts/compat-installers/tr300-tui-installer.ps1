# Compatibility router for immutable SD-300 1.4.x Windows updaters.
# Fresh installs use sd300-cli-installer.ps1; this filename exists only because old
# binaries hard-coded cargo-dist's package-derived asset name.

$ErrorActionPreference = 'Stop'
$InformationPreference = 'Continue'
$Sd300Tag = '@SD300_TAG@'
$Sd300Version = '@SD300_VERSION@'
$ReleaseBase = "https://github.com/QubeTX/qube-system-diagnostics/releases/download/$Sd300Tag"

function Get-Sd300MsiChannels {
    if (-not ('Sd300.Compat.Msi' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
namespace Sd300.Compat {
    public static class Msi {
        [DllImport("msi.dll", CharSet = CharSet.Unicode)]
        public static extern uint MsiEnumRelatedProducts(string code, uint reserved, uint index, StringBuilder product);
    }
}
'@
    }
    $families = @(
        [pscustomobject]@{ Code = '{A7C3E1D4-9F8B-4E2A-B6D1-3F5C8A0E7B94}'; Channel = 'msi-global' },
        [pscustomobject]@{ Code = '{143F59B2-5D4B-4F6F-B258-BB44F9C50CC9}'; Channel = 'msi-corporate' }
    )
    $channels = @()
    foreach ($family in $families) {
        for ($index = 0; ; $index++) {
            $product = New-Object Text.StringBuilder 39
            $result = [Sd300.Compat.Msi]::MsiEnumRelatedProducts($family.Code, 0, [uint32]$index, $product)
            if ($result -eq 259) { break }
            if ($result -ne 0) { throw "MSI ownership query failed with code $result" }
            $channels += $family.Channel
        }
    }
    return @($channels)
}

function Get-Sd300InnoChannels {
    $families = @(
        [pscustomobject]@{ Id = 'DC74D35F-CBF4-425F-B11E-E9EA87C13CA9'; Channel = 'exe-global' },
        [pscustomobject]@{ Id = 'ED209931-B5C0-43AE-89F6-83EE2C581653'; Channel = 'exe-corporate' }
    )
    $roots = @(
        'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )
    $channels = @()
    foreach ($family in $families) {
        foreach ($root in $roots) {
            if (Test-Path -LiteralPath "$root\{$($family.Id)}_is1") {
                $channels += $family.Channel
            }
        }
    }
    return @($channels)
}

function Get-Sd300NativeChannel {
    $channels = @(@(Get-Sd300MsiChannels) + @(Get-Sd300InnoChannels) | Sort-Object -Unique)
    $marker = $null
    try {
        $marker = [string](Get-ItemPropertyValue -LiteralPath 'Registry::HKEY_CURRENT_USER\Software\SD300' -Name InstallSource)
    } catch {}
    if ($channels.Count -gt 1) {
        throw "multiple SD-300 native channels are registered: $($channels -join ', ')"
    }
    if ($marker -and $channels.Count -eq 0) {
        throw "the SD-300 install marker names $marker but no native registration proves it"
    }
    if ($marker -and $channels[0] -ne $marker) {
        throw "the SD-300 install marker $marker conflicts with registered channel $($channels[0])"
    }
    if ($channels.Count -eq 1) { return $channels[0] }
    return $null
}

function Save-Sd300Asset([string]$Name, [string]$Directory) {
    $asset = Join-Path $Directory $Name
    $sidecar = "$asset.sha256"
    $headers = @{ 'User-Agent' = 'sd300-compat-updater' }
    foreach ($nameAndPath in @(@($Name, $asset), @("$Name.sha256", $sidecar))) {
        Invoke-WebRequest -UseBasicParsing -Headers $headers -Uri "$ReleaseBase/$($nameAndPath[0])" -OutFile $nameAndPath[1]
    }
    $expected = ((Get-Content -LiteralPath $sidecar -Raw) -split '\s+')[0].ToLowerInvariant()
    if ($expected -notmatch '^[0-9a-f]{64}$') { throw "invalid checksum sidecar for $Name" }
    $actual = (Get-FileHash -LiteralPath $asset -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw "checksum mismatch for $Name" }
    return $asset
}

function Invoke-Sd300Managed([string]$Directory) {
    $script = Save-Sd300Asset 'sd300-cli-installer.ps1' $Directory
    $hostExe = if ($PSVersionTable.PSEdition -eq 'Core') { Join-Path $PSHOME 'pwsh.exe' } else { Join-Path $PSHOME 'powershell.exe' }
    $process = Start-Process -FilePath $hostExe -ArgumentList @(
        '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', $script
    ) -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0) { throw "managed installer exited with code $($process.ExitCode)" }
}

if ($env:SD300_COMPAT_INSTALLER_TEST_ONLY -eq '1') { return }

$temp = Join-Path ([IO.Path]::GetTempPath()) ("sd300-compat-" + [guid]::NewGuid().ToString('N'))
try {
    $null = New-Item -ItemType Directory -Path $temp -Force
    $channel = Get-Sd300NativeChannel
    if (-not $channel) {
        Invoke-Sd300Managed $temp
        return
    }

    $assetName = "sd300-windows-x64-$($channel -replace '^(msi|exe)-', '').$($channel.Substring(0, 3))"
    $asset = Save-Sd300Asset $assetName $temp
    $global = $channel.EndsWith('-global')
    if ($channel.StartsWith('msi-')) {
        $params = @{
            FilePath = 'msiexec.exe'
            ArgumentList = @('/i', $asset, '/passive', '/norestart')
            Wait = $true
            PassThru = $true
            WindowStyle = 'Hidden'
        }
    } else {
        $params = @{
            FilePath = $asset
            ArgumentList = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
            Wait = $true
            PassThru = $true
            WindowStyle = 'Hidden'
        }
    }
    if ($global) { $params.Verb = 'RunAs' }
    Write-Information "Updating SD-300 through its proven $channel channel..."
    $process = Start-Process @params
    if ($process.ExitCode -notin @(0, 1641, 3010)) {
        throw "$channel installer exited with code $($process.ExitCode)"
    }
} finally {
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
