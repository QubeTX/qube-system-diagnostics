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
        [pscustomobject]@{
            UpgradeCode = '{A7C3E1D4-9F8B-4E2A-B6D1-3F5C8A0E7B94}'
            Channel = 'msi-global'; Elevated = $true
            Root = (Join-Path $env:ProgramFiles 'sd300')
        },
        [pscustomobject]@{
            UpgradeCode = '{143F59B2-5D4B-4F6F-B258-BB44F9C50CC9}'
            Channel = 'msi-corporate'; Elevated = $false
            Root = (Join-Path $env:LOCALAPPDATA 'Programs\sd300')
        }
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
                Root = [IO.Path]::GetFullPath($family.Root)
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
                Root = [IO.Path]::GetFullPath($family.Root)
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
        $process = Invoke-Sd300Process 'msiexec.exe' @('/x', $Product.ProductCode, '/passive', '/norestart', 'SD300GUIALREADYSTOPPED=1', 'SD300PRESERVEGUISTATE=1') $Product.Elevated
        if ($process.ExitCode -notin @(0, 1605, 1614, 1641, 3010)) {
            throw "$($Product.Channel) uninstall exited with Windows Installer code $($process.ExitCode)"
        }
    } else {
        $process = Invoke-Sd300Process $Product.Uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SD300GUIALREADYSTOPPED', '/PRESERVEGUISTATE') $Product.Elevated
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

function Get-Sd300GuiRoot {
    if (-not $env:LOCALAPPDATA) { throw 'LOCALAPPDATA is unavailable; cannot install the GUI companion' }
    return [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs\SD-300'))
}

function Test-Sd300ManagedGuiRoot([string]$Root) {
    $ownerPath = Join-Path $Root '.sd300-managed-owner.json'
    if (-not (Test-Path -LiteralPath $ownerPath -PathType Leaf)) { return $false }
    try {
        $owner = Get-Content -LiteralPath $ownerPath -Raw | ConvertFrom-Json
        return $owner.schema -eq 1 -and
            $owner.product -eq 'SD-300' -and
            $owner.owner -eq 'powershell-installer'
    } catch {
        return $false
    }
}

function Stop-Sd300OwnedGui([string[]]$OwnedRoots) {
    $ownedGuiPaths = @($OwnedRoots | ForEach-Object {
        if (-not [string]::IsNullOrWhiteSpace($_)) {
            Join-Path ([IO.Path]::GetFullPath($_)) 'app\sd300-gui.exe'
        }
    } | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -Unique)
    if ($ownedGuiPaths.Count -eq 0) { return }

    $running = @(Get-Process -Name 'sd300-gui' -ErrorAction SilentlyContinue)
    if ($running.Count -eq 0) { return }

    if (-not ('Sd300.ManagedInstaller.GuiLifecycle' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace Sd300.ManagedInstaller {
    public static class GuiLifecycle {
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        public static extern IntPtr OpenEvent(uint desiredAccess, bool inheritHandle, string name);
        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetEvent(IntPtr handle);
        [DllImport("kernel32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool CloseHandle(IntPtr handle);
    }
}
'@
    }

    $eventHandle = [Sd300.ManagedInstaller.GuiLifecycle]::OpenEvent(
        0x0002,
        $false,
        'Local\SD300.Gui.Quit.v1'
    )
    if ($eventHandle -eq [IntPtr]::Zero) {
        throw 'an owned SD-300 GUI is running without its authenticated lifecycle endpoint'
    }
    try {
        if (-not [Sd300.ManagedInstaller.GuiLifecycle]::SetEvent($eventHandle)) {
            throw 'the authenticated SD-300 GUI lifecycle endpoint could not be signaled'
        }
    } finally {
        [void][Sd300.ManagedInstaller.GuiLifecycle]::CloseHandle($eventHandle)
    }

    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    do {
        Start-Sleep -Milliseconds 100
        $running = @(Get-Process -Name 'sd300-gui' -ErrorAction SilentlyContinue)
    } while ($running.Count -gt 0 -and [DateTime]::UtcNow -lt $deadline)
    if ($running.Count -gt 0) {
        throw "$($running.Count) SD-300 GUI process(es) did not exit through the authenticated lifecycle endpoint"
    }
}

function Expand-Sd300GuiPayload([string]$Archive, [string]$Destination) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $destinationRoot = [IO.Path]::GetFullPath($Destination).TrimEnd('\') + '\'
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $archivePaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        foreach ($entry in $zip.Entries) {
            $relative = $entry.FullName.Replace('\', '/').TrimEnd('/')
            if (-not $relative) { continue }
            # ExternalAttributes is exposed as a signed Int32 even though ZIP
            # stores it as an unsigned bit field.
            $externalAttributes = ([long]$entry.ExternalAttributes -band 0xFFFFFFFFL)
            $unixFileType = (($externalAttributes -shr 16) -band 0xF000)
            $isDirectoryName = $entry.FullName.EndsWith('/') -or $entry.FullName.EndsWith('\')
            $declaresDirectory = $unixFileType -eq 0x4000
            $declaresRegularFile = $unixFileType -eq 0x8000
            if (($externalAttributes -band [uint32][IO.FileAttributes]::ReparsePoint) -ne 0 -or
                ($unixFileType -ne 0 -and -not $declaresDirectory -and -not $declaresRegularFile) -or
                ($declaresDirectory -and -not $isDirectoryName) -or
                ($declaresRegularFile -and $isDirectoryName)) {
                throw "GUI archive contains a symbolic link or special member: $($entry.FullName)"
            }
            if ($relative -notmatch '^[A-Za-z0-9._/-]+$' -or
                [IO.Path]::IsPathRooted($relative) -or
                @($relative.Split('/') | Where-Object { $_ -eq '.' -or $_ -eq '..' }).Count -gt 0 -or
                -not $archivePaths.Add($relative)) {
                throw "GUI archive contains an unsafe or duplicate path: $($entry.FullName)"
            }
            $resolved = [IO.Path]::GetFullPath((Join-Path $Destination $relative.Replace('/', '\')))
            if (-not $resolved.StartsWith($destinationRoot, [StringComparison]::OrdinalIgnoreCase)) {
                throw "GUI archive path escapes its staging root: $($entry.FullName)"
            }
        }
    } finally {
        $zip.Dispose()
    }
    Expand-Archive -LiteralPath $Archive -DestinationPath $Destination
    $reparsePoints = @(Get-ChildItem -LiteralPath $Destination -Recurse -Force | Where-Object {
        ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
    })
    if ($reparsePoints.Count -gt 0) {
        throw "GUI archive contains a reparse point: $($reparsePoints[0].FullName)"
    }
    $manifestPath = Join-Path $Destination 'install-manifest.json'
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw 'GUI archive has no install-manifest.json'
    }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.schema -ne 1 -or $manifest.product -ne 'SD-300' -or
        $manifest.version -ne $Sd300Version -or $manifest.target -ne 'windows-x86_64' -or
        $manifest.entrypoint -ne 'sd300-gui.exe' -or $manifest.engine -ne 'sd300_engine.dll') {
        throw 'GUI archive identity does not match the requested SD-300 release and target'
    }
    $declared = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in @($manifest.files)) {
        $relative = [string]$file.path
        if ($relative -notmatch '^[A-Za-z0-9._/-]+$' -or
            [IO.Path]::IsPathRooted($relative) -or
            @($relative.Split('/') | Where-Object { $_ -eq '.' -or $_ -eq '..' }).Count -gt 0 -or
            -not $declared.Add($relative)) {
            throw "GUI archive manifest contains an unsafe path: $relative"
        }
        $path = [IO.Path]::GetFullPath((Join-Path $Destination $relative))
        if (-not $path.StartsWith($destinationRoot, [StringComparison]::OrdinalIgnoreCase) -or
            -not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "GUI archive is missing its declared file: $relative"
        }
        if ([long]$file.bytes -ne (Get-Item -LiteralPath $path).Length) {
            throw "GUI archive file has the wrong declared size: $relative"
        }
        $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
        if ($actual -ne [string]$file.sha256) {
            throw "GUI archive file failed manifest verification: $relative"
        }
    }
    $actualFiles = @(Get-ChildItem -LiteralPath $Destination -Recurse -File -Force | Where-Object {
        -not $_.FullName.Equals($manifestPath, [StringComparison]::OrdinalIgnoreCase)
    })
    if ($actualFiles.Count -ne $declared.Count) {
        throw 'GUI archive manifest does not exactly cover the extracted payload'
    }
    foreach ($file in $actualFiles) {
        $relative = $file.FullName.Substring($destinationRoot.Length).Replace('\', '/')
        if (-not $declared.Contains($relative)) {
            throw "GUI archive contains an undeclared file: $relative"
        }
    }
    return $manifest
}

function Test-Sd300GuiPayload([string]$Root) {
    $stdout = Join-Path $tempRoot 'gui-self-test.json'
    $stderr = Join-Path $tempRoot 'gui-self-test.err'
    $process = Start-Process -FilePath (Join-Path $Root 'sd300-gui.exe') `
        -ArgumentList @('--self-test', '--json') -Wait -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    if ($process.ExitCode -ne 0) {
        $detail = if (Test-Path -LiteralPath $stderr) { (Get-Content -LiteralPath $stderr -Raw).Trim() } else { '' }
        throw "GUI companion self-test failed (exit $($process.ExitCode)): $detail"
    }
    $result = Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json
    if (-not $result.success -or $result.product -ne 'SD-300' -or
        $result.product_version -ne $Sd300Version -or $result.abi_version -ne 1 -or
        $result.engine_schema_version -ne 1 -or $result.target_os -ne 'windows' -or
        $result.target_arch -ne 'x86_64') {
        throw 'GUI companion self-test reported an incompatible product, version, ABI, schema, or target'
    }
}

function Install-Sd300GuiPayload([string]$StagedRoot, [string]$CliBinary) {
    $root = Get-Sd300GuiRoot
    if (Test-Path -LiteralPath $root) {
        if (-not (Test-Sd300ManagedGuiRoot $root)) {
            throw "the GUI destination exists without managed PowerShell ownership: $root"
        }
        Remove-Item -LiteralPath $root -Recurse -Force
    }
    $null = New-Item -ItemType Directory -Path $root -Force
    Move-Item -LiteralPath $StagedRoot -Destination (Join-Path $root 'app')
    $ownerJson = @{
        schema = 1
        product = 'SD-300'
        version = $Sd300Version
        owner = 'powershell-installer'
    } | ConvertTo-Json
    [IO.File]::WriteAllText(
        (Join-Path $root '.sd300-managed-owner.json'),
        $ownerJson + "`n",
        [Text.UTF8Encoding]::new($false)
    )

    $programs = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
    $shortcut = Join-Path $programs 'SD-300.lnk'
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut($shortcut)
    $link.TargetPath = Join-Path $root 'app\sd300-gui.exe'
    $link.WorkingDirectory = Join-Path $root 'app'
    $link.Description = 'Open the SD-300 native system monitor'
    $link.Save()

    $uninstallKey = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\SD-300-Managed'
    $null = New-Item -Path $uninstallKey -Force
    New-ItemProperty -Path $uninstallKey -Name DisplayName -Value 'SD-300' -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $uninstallKey -Name DisplayVersion -Value $Sd300Version -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $uninstallKey -Name Publisher -Value 'Emmett S' -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $uninstallKey -Name InstallLocation -Value $root -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $uninstallKey -Name UninstallString -Value ('"' + $CliBinary + '" uninstall') -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $uninstallKey -Name NoModify -Value 1 -PropertyType DWord -Force | Out-Null
    New-ItemProperty -Path $uninstallKey -Name NoRepair -Value 1 -PropertyType DWord -Force | Out-Null

    Test-Sd300GuiPayload (Join-Path $root 'app')
}

function Save-Sd300ManagedState([string]$BackupRoot, [object[]]$NativeProducts) {
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

    $guiRoot = Get-Sd300GuiRoot
    $guiRootExisted = Test-Path -LiteralPath $guiRoot -PathType Container
    $guiWasManaged = $guiRootExisted -and (Test-Sd300ManagedGuiRoot $guiRoot)
    $corporateNativeOwnsRoot = @($NativeProducts | Where-Object { $_.Channel -in @('msi-corporate', 'exe-corporate') }).Count -gt 0
    if ($guiRootExisted -and -not $guiWasManaged -and -not $corporateNativeOwnsRoot) {
        throw "the GUI destination exists without a proven SD-300 owner: $guiRoot"
    }
    $guiBackup = Join-Path $BackupRoot 'gui-root'
    if ($guiRootExisted) {
        Copy-Item -LiteralPath $guiRoot -Destination $guiBackup -Recurse
    }
    $shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\SD-300.lnk'
    $shortcutExisted = Test-Path -LiteralPath $shortcut -PathType Leaf
    $shortcutBackup = Join-Path $BackupRoot 'SD-300.lnk'
    if ($shortcutExisted) {
        Copy-Item -LiteralPath $shortcut -Destination $shortcutBackup
    }
    $uninstallKey = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\SD-300-Managed'
    $uninstallKeyExisted = Test-Path -LiteralPath $uninstallKey
    $uninstallProperties = if ($uninstallKeyExisted) {
        $property = Get-ItemProperty -LiteralPath $uninstallKey
        [pscustomobject]@{
            DisplayName = [string]$property.DisplayName
            DisplayVersion = [string]$property.DisplayVersion
            Publisher = [string]$property.Publisher
            InstallLocation = [string]$property.InstallLocation
            UninstallString = [string]$property.UninstallString
            NoModify = [int]$property.NoModify
            NoRepair = [int]$property.NoRepair
        }
    } else {
        $null
    }

    $environmentKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $false)
    $environmentKeyExisted = $null -ne $environmentKey
    try {
        $userPathName = if ($environmentKey) {
            @($environmentKey.GetValueNames() | Where-Object {
                $_.Equals('Path', [StringComparison]::OrdinalIgnoreCase)
            }) | Select-Object -First 1
        } else {
            $null
        }
        $userPathExisted = $null -ne $userPathName
        $userPathValue = if ($userPathExisted) {
            $environmentKey.GetValue(
                $userPathName,
                $null,
                [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
            )
        } else {
            $null
        }
        $userPathKind = if ($userPathExisted) {
            $environmentKey.GetValueKind($userPathName)
        } else {
            $null
        }
    } finally {
        if ($environmentKey) { $environmentKey.Dispose() }
    }

    $githubPath = if ([string]::IsNullOrWhiteSpace($env:GITHUB_PATH)) {
        $null
    } else {
        [IO.Path]::GetFullPath($env:GITHUB_PATH)
    }
    $githubPathExisted = $false
    $githubPathBackup = Join-Path $BackupRoot 'github-path'
    if ($githubPath) {
        if (Test-Path -LiteralPath $githubPath) {
            $githubPathItem = Get-Item -LiteralPath $githubPath -Force
            if ($githubPathItem.PSIsContainer -or
                ($githubPathItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "GITHUB_PATH is not a regular non-reparse file: $githubPath"
            }
            $githubPathExisted = $true
            Copy-Item -LiteralPath $githubPath -Destination $githubPathBackup
        }
    }

    return [pscustomobject]@{
        ReceiptPath = $receiptPath
        ReceiptExisted = $receiptExisted
        ReceiptBackup = (Join-Path $BackupRoot 'receipt.json')
        PriorPrefix = $priorPrefix
        Binaries = $binaries
        GuiRoot = $guiRoot
        GuiRootExisted = $guiRootExisted
        GuiWasManaged = $guiWasManaged
        GuiBackup = $guiBackup
        Shortcut = $shortcut
        ShortcutExisted = $shortcutExisted
        ShortcutBackup = $shortcutBackup
        UninstallKey = $uninstallKey
        UninstallKeyExisted = $uninstallKeyExisted
        UninstallProperties = $uninstallProperties
        UserPathName = $userPathName
        EnvironmentKeyExisted = $environmentKeyExisted
        UserPathExisted = $userPathExisted
        UserPathValue = $userPathValue
        UserPathKind = $userPathKind
        GithubPath = $githubPath
        GithubPathExisted = $githubPathExisted
        GithubPathBackup = $githubPathBackup
    }
}

function Restore-Sd300ManagedState($State) {
    if ($State.EnvironmentKeyExisted) {
        $environmentKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment')
        try {
            if ($State.UserPathExisted) {
                $environmentKey.SetValue(
                    $State.UserPathName,
                    $State.UserPathValue,
                    [Microsoft.Win32.RegistryValueKind]$State.UserPathKind
                )
            } else {
                $environmentKey.DeleteValue('Path', $false)
            }
        } finally {
            $environmentKey.Dispose()
        }
    } else {
        $environmentKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
        if ($environmentKey) {
            try {
                $environmentKey.DeleteValue('Path', $false)
                if ($environmentKey.ValueCount -ne 0 -or $environmentKey.SubKeyCount -ne 0) {
                    throw 'refusing to remove a concurrently populated user Environment key during rollback'
                }
            } finally {
                $environmentKey.Dispose()
            }
            [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKey('Environment', $false)
        }
    }
    $dummyName = 'sd300-rollback-' + [guid]::NewGuid().ToString('N')
    [Environment]::SetEnvironmentVariable($dummyName, 'sd300-rollback', 'User')
    [Environment]::SetEnvironmentVariable($dummyName, $null, 'User')

    if ($State.GithubPath) {
        if (Test-Path -LiteralPath $State.GithubPath) {
            $githubPathItem = Get-Item -LiteralPath $State.GithubPath -Force
            if ($githubPathItem.PSIsContainer -or
                ($githubPathItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "refusing to overwrite an unexpected GITHUB_PATH object during rollback: $($State.GithubPath)"
            }
        }
        if ($State.GithubPathExisted) {
            $null = New-Item -ItemType Directory -Path (Split-Path -Parent $State.GithubPath) -Force
            Copy-Item -LiteralPath $State.GithubPathBackup -Destination $State.GithubPath -Force
        } else {
            Remove-Item -LiteralPath $State.GithubPath -Force -ErrorAction SilentlyContinue
        }
    }

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
    if (Test-Path -LiteralPath $State.GuiRoot) {
        if (Test-Sd300ManagedGuiRoot $State.GuiRoot) {
            Remove-Item -LiteralPath $State.GuiRoot -Recurse -Force
        } elseif (-not $State.GuiRootExisted) {
            throw "refusing to remove an unowned GUI root during rollback: $($State.GuiRoot)"
        }
    }
    if ($State.GuiRootExisted -and -not (Test-Path -LiteralPath $State.GuiRoot)) {
        Copy-Item -LiteralPath $State.GuiBackup -Destination $State.GuiRoot -Recurse
    }
    if ($State.ShortcutExisted) {
        Copy-Item -LiteralPath $State.ShortcutBackup -Destination $State.Shortcut -Force
    } else {
        Remove-Item -LiteralPath $State.Shortcut -Force -ErrorAction SilentlyContinue
    }
    if ($State.UninstallKeyExisted) {
        $null = New-Item -Path $State.UninstallKey -Force
        New-ItemProperty -Path $State.UninstallKey -Name DisplayName -Value $State.UninstallProperties.DisplayName -PropertyType String -Force | Out-Null
        New-ItemProperty -Path $State.UninstallKey -Name DisplayVersion -Value $State.UninstallProperties.DisplayVersion -PropertyType String -Force | Out-Null
        New-ItemProperty -Path $State.UninstallKey -Name Publisher -Value $State.UninstallProperties.Publisher -PropertyType String -Force | Out-Null
        New-ItemProperty -Path $State.UninstallKey -Name InstallLocation -Value $State.UninstallProperties.InstallLocation -PropertyType String -Force | Out-Null
        New-ItemProperty -Path $State.UninstallKey -Name UninstallString -Value $State.UninstallProperties.UninstallString -PropertyType String -Force | Out-Null
        New-ItemProperty -Path $State.UninstallKey -Name NoModify -Value $State.UninstallProperties.NoModify -PropertyType DWord -Force | Out-Null
        New-ItemProperty -Path $State.UninstallKey -Name NoRepair -Value $State.UninstallProperties.NoRepair -PropertyType DWord -Force | Out-Null
    } else {
        Remove-Item -LiteralPath $State.UninstallKey -Recurse -Force -ErrorAction SilentlyContinue
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

    $guiArchive = Join-Path $tempRoot 'sd300-gui-windows-x86_64.zip'
    $guiSidecar = "$guiArchive.sha256"
    Get-Sd300ReleaseFile 'sd300-gui-windows-x86_64.zip' $guiArchive $headers
    Get-Sd300ReleaseFile 'sd300-gui-windows-x86_64.zip.sha256' $guiSidecar $headers
    Assert-Sd300Sha256 $guiArchive $guiSidecar
    $guiStage = Join-Path $tempRoot 'gui-payload'
    $null = Expand-Sd300GuiPayload $guiArchive $guiStage
    Test-Sd300GuiPayload $guiStage

    $managedState = Save-Sd300ManagedState $tempRoot $native
    $ownedGuiRoots = @($managedState.GuiRoot) + @($native | ForEach-Object { $_.Root })
    Stop-Sd300OwnedGui $ownedGuiRoots

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

    Install-Sd300GuiPayload $guiStage $binary

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
    Test-Sd300GuiPayload (Join-Path (Get-Sd300GuiRoot) 'app')
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
