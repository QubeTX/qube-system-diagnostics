[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('windows-x86_64')]
    [string]$Target,

    [Parameter(Mandatory = $true)]
    [string]$Version,

    [string]$PackageRoot = '',
    [string]$OutputDirectory = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not $PackageRoot) {
    $PackageRoot = Join-Path $repoRoot "target\native-gui-stage\$Target\app\zig-out"
}
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repoRoot 'target\native-gui-packages'
}
$package = (Resolve-Path -LiteralPath $PackageRoot).Path
$bin = Join-Path $package 'bin'
$required = @(
    (Join-Path $bin 'sd300-gui.exe'),
    (Join-Path $bin 'sd300_engine.dll'),
    (Join-Path $bin 'assets\icon.png'),
    (Join-Path $bin 'licenses\PRODUCT-LICENSE.md'),
    (Join-Path $bin 'licenses\IBM-PLEX-OFL-1.1.txt'),
    (Join-Path $bin 'licenses\NATIVE-SDK-APACHE-2.0.txt')
)
foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Native GUI package input is incomplete: $path"
    }
}

$stageBase = Join-Path $repoRoot 'target\native-gui-package-stage'
$stage = Join-Path $stageBase $Target
$stageBaseFull = [IO.Path]::GetFullPath($stageBase)
$stageFull = [IO.Path]::GetFullPath($stage)
if (-not $stageFull.StartsWith($stageBaseFull + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to stage outside $stageBaseFull"
}
if (Test-Path -LiteralPath $stageFull) {
    Remove-Item -LiteralPath $stageFull -Recurse -Force
}
$null = New-Item -ItemType Directory -Path (Join-Path $stageFull 'assets') -Force
$null = New-Item -ItemType Directory -Path (Join-Path $stageFull 'licenses') -Force
Copy-Item -LiteralPath $required[0] -Destination (Join-Path $stageFull 'sd300-gui.exe')
Copy-Item -LiteralPath $required[1] -Destination (Join-Path $stageFull 'sd300_engine.dll')
Copy-Item -LiteralPath $required[2] -Destination (Join-Path $stageFull 'assets\icon.png')
Copy-Item -LiteralPath $required[3] -Destination (Join-Path $stageFull 'licenses\PRODUCT-LICENSE.md')
Copy-Item -LiteralPath $required[4] -Destination (Join-Path $stageFull 'licenses\IBM-PLEX-OFL-1.1.txt')
Copy-Item -LiteralPath $required[5] -Destination (Join-Path $stageFull 'licenses\NATIVE-SDK-APACHE-2.0.txt')

$files = @()
foreach ($relative in @(
    'sd300-gui.exe',
    'sd300_engine.dll',
    'assets/icon.png',
    'licenses/PRODUCT-LICENSE.md',
    'licenses/IBM-PLEX-OFL-1.1.txt',
    'licenses/NATIVE-SDK-APACHE-2.0.txt'
)) {
    $nativeRelative = $relative.Replace('/', [IO.Path]::DirectorySeparatorChar)
    $path = Join-Path $stageFull $nativeRelative
    $files += [ordered]@{
        path = $relative
        sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        bytes = (Get-Item -LiteralPath $path).Length
    }
}
$manifest = [ordered]@{
    schema = 1
    product = 'SD-300'
    version = $Version
    target = $Target
    entrypoint = 'sd300-gui.exe'
    engine = 'sd300_engine.dll'
    files = $files
}
[IO.File]::WriteAllText(
    (Join-Path $stageFull 'install-manifest.json'),
    (($manifest | ConvertTo-Json -Depth 6) + "`n"),
    [Text.UTF8Encoding]::new($false)
)

$null = New-Item -ItemType Directory -Path $OutputDirectory -Force
$assetName = 'sd300-gui-windows-x86_64.zip'
$asset = Join-Path $OutputDirectory $assetName
if (Test-Path -LiteralPath $asset) {
    Remove-Item -LiteralPath $asset -Force
}
Compress-Archive -Path (Join-Path $stageFull '*') -DestinationPath $asset -CompressionLevel Optimal

$distributionCheck = Join-Path $PSScriptRoot 'check-native-distribution.ps1'
& $distributionCheck -GuiRoot (Join-Path $repoRoot 'gui') -PackageRoot $stageFull

$hash = (Get-FileHash -LiteralPath $asset -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText(
    "$asset.sha256",
    "$hash *$assetName`n",
    [Text.Encoding]::ASCII
)
[ordered]@{
    schema = 1
    target = $Target
    version = $Version
    asset = $asset
    sha256 = $hash
} | ConvertTo-Json
