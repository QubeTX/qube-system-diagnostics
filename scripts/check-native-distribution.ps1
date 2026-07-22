param(
    [string]$GuiRoot = (Join-Path $PSScriptRoot "..\gui"),
    [string]$PackageRoot = ""
)

$ErrorActionPreference = "Stop"
$gui = (Resolve-Path -LiteralPath $GuiRoot).Path
$repo = (Resolve-Path -LiteralPath (Join-Path $gui "..")).Path
$toolchain = Get-Content -Raw -LiteralPath (Join-Path $gui "toolchain-lock.json") | ConvertFrom-Json
$zon = Join-Path $gui "build.zig.zon"
$source = Get-Content -Raw -LiteralPath $zon

$requiredVersion = $toolchain.native_sdk.version
$requiredUrl = $toolchain.native_sdk.tarball
$requiredIntegrity = $toolchain.native_sdk.npm_integrity
$requiredHash = $toolchain.native_sdk.zig_content_hash
$requiredGitHead = $toolchain.native_sdk.npm_git_head
$requiredPatchPath = $toolchain.native_sdk.renderer_patch
$requiredPatchHash = $toolchain.native_sdk.renderer_patch_sha256
$requiredMakiraHash = $toolchain.fonts.makira.sha256
if ($toolchain.schema -ne 1 -or -not $requiredVersion -or -not $requiredUrl -or
    -not $requiredIntegrity -or -not $requiredHash -or -not $requiredPatchPath -or
    $requiredPatchHash -notmatch '^[0-9a-f]{64}$' -or
    $requiredMakiraHash -notmatch '^[0-9a-f]{64}$' -or
    $toolchain.fonts.makira.source -ne 'licensed-repository-secret' -or
    $requiredGitHead -notmatch '^[0-9a-f]{40}$') {
    throw "gui/toolchain-lock.json is incomplete."
}
$rendererPatch = Join-Path $gui $requiredPatchPath
if (-not (Test-Path -LiteralPath $rendererPatch -PathType Leaf) -or
    (Get-FileHash -LiteralPath $rendererPatch -Algorithm SHA256).Hash.ToLowerInvariant() -ne $requiredPatchHash) {
    throw "The Native SDK renderer patch disagrees with gui/toolchain-lock.json."
}
if (-not $source.Contains($requiredUrl) -or -not $source.Contains($requiredHash)) {
    throw "Native SDK dependency is not pinned to the reviewed tarball and Zig content hash."
}

$packageManifest = Get-Content -Raw -LiteralPath (Join-Path $gui "package.json") | ConvertFrom-Json
$packageLock = Get-Content -Raw -LiteralPath (Join-Path $gui "package-lock.json") | ConvertFrom-Json -AsHashtable
$lockedSdk = $packageLock['packages']['node_modules/@native-sdk/cli']
if ($packageManifest.devDependencies.'@native-sdk/cli' -ne $requiredVersion -or
    $lockedSdk['version'] -ne $requiredVersion -or
    $lockedSdk['resolved'] -ne $requiredUrl -or
    $lockedSdk['integrity'] -ne $requiredIntegrity) {
    throw "npm manifest, lockfile, and reviewed Native SDK toolchain record disagree."
}
foreach ($entry in $lockedSdk['optionalDependencies'].GetEnumerator()) {
    $lockedHost = $packageLock['packages']["node_modules/$($entry.Key)"]
    if ($entry.Value -ne $requiredVersion -or $lockedHost['version'] -ne $requiredVersion -or
        $lockedHost['resolved'] -notmatch '^https://registry\.npmjs\.org/@native-sdk/' -or
        $lockedHost['integrity'] -notmatch '^sha512-[A-Za-z0-9+/]+={0,2}$') {
        throw "Native SDK optional host package is not immutable: $($entry.Key)."
    }
}

if ($toolchain.zig.version -ne '0.16.0') {
    throw "The reviewed Zig toolchain must be 0.16.0."
}
$rustToolchain = Get-Content -Raw -LiteralPath (Join-Path $repo 'rust-toolchain.toml')
if (-not $rustToolchain.Contains("channel = `"$($toolchain.rust.channel)`"")) {
    throw "rust-toolchain.toml disagrees with gui/toolchain-lock.json."
}
foreach ($platform in @('x86_64-windows', 'x86_64-macos', 'aarch64-macos', 'x86_64-linux', 'aarch64-linux')) {
    $archive = $toolchain.zig.archives.$platform
    if ($null -eq $archive -or $archive.url -notmatch '/0\.16\.0/' -or
        $archive.sha256 -notmatch '^[0-9a-f]{64}$') {
        throw "Zig archive pin is incomplete for $platform."
    }
}
$rustAction = $toolchain.rust.github_action
if ($rustAction -notmatch '^dtolnay/rust-toolchain@[0-9a-f]{40}$') {
    throw 'The Rust setup action must be recorded at an immutable commit.'
}
foreach ($relativePath in @(
    '.github\workflows\windows-installers.yml',
    '.github\workflows\macos-installer.yml',
    '.github\workflows\linux-native-gui.yml'
)) {
    $workflow = Get-Content -Raw -LiteralPath (Join-Path $repo $relativePath)
    if (-not $workflow.Contains("uses: $rustAction") -or
        -not $workflow.Contains("toolchain: `"$($toolchain.rust.channel)`"")) {
        throw "$relativePath disagrees with the reviewed Rust toolchain record."
    }
}
$muslBuilder = Get-Content -Raw -LiteralPath (Join-Path $repo 'scripts\build-native-gui-musl-container.sh')
$muslZig = $toolchain.zig.archives.'x86_64-linux'
if (-not $muslBuilder.Contains("--default-toolchain $($toolchain.rust.channel)") -or
    -not $muslBuilder.Contains("== $($toolchain.rust.channel).0") -or
    -not $muslBuilder.Contains($muslZig.url) -or
    -not $muslBuilder.Contains($muslZig.sha256)) {
    throw 'The Alpine musl builder disagrees with the reviewed Rust/Zig toolchain record.'
}
foreach ($relativePath in @('scripts\build-native-gui.sh', 'scripts\build-native-gui.ps1')) {
    $builder = Get-Content -Raw -LiteralPath (Join-Path $repo $relativePath)
    if (-not $builder.Contains("$($toolchain.rust.channel).0") -or
        -not $builder.Contains($toolchain.zig.version)) {
        throw "$relativePath does not enforce the reviewed Rust/Zig versions."
    }
}
foreach ($relativePath in @('scripts\prepare-native-sdk.mjs', 'scripts\prepare-native-sdk.ps1')) {
    $prepare = Get-Content -Raw -LiteralPath (Join-Path $repo $relativePath)
    foreach ($value in @($requiredVersion, $requiredUrl, $requiredIntegrity, $requiredHash, $requiredPatchHash)) {
        if (-not $prepare.Contains($value)) {
            throw "$relativePath disagrees with gui/toolchain-lock.json."
        }
    }
}

$makiraPrepare = Get-Content -Raw -LiteralPath (Join-Path $repo 'scripts\prepare-makira-font.mjs')
foreach ($marker in @(
    'SD300_MAKIRA_FONT_BROTLI_BASE64_PART_1',
    'SD300_MAKIRA_FONT_BROTLI_BASE64_PART_2',
    'brotliDecompressSync'
)) {
    if (-not $makiraPrepare.Contains($marker)) {
        throw "Licensed Makira preparer is missing $marker."
    }
}
$gitignore = Get-Content -LiteralPath (Join-Path $repo '.gitignore')
if ($gitignore -notcontains 'gui/src/fonts/Makira-Regular.ttf') {
    throw 'The commercial Makira source font must remain excluded from the public repository.'
}

$forbiddenSource = @(
    '(?im)\.path\s*=',
    '(?i)AppData[\\/](Local|Roaming)',
    '(?i)node_modules[\\/]@native-sdk[\\/]cli'
)
foreach ($pattern in $forbiddenSource) {
    if ($source -match $pattern) {
        throw "Developer-local Native SDK dependency detected in build.zig.zon: $pattern"
    }
}

if ($PackageRoot) {
    $package = (Resolve-Path -LiteralPath $PackageRoot).Path
    $forbiddenBytes = '(?i)([A-Z]:[\\/]Users[\\/]|[\\/]Users[\\/][^\\/]+[\\/]|[\\/]home[\\/][^\\/]+[\\/]|AppData[\\/](Local|Roaming)|node_modules[\\/]@native-sdk[\\/]cli)'
    $matches = Get-ChildItem -LiteralPath $package -Recurse -File | Select-String -Pattern $forbiddenBytes -List -ErrorAction SilentlyContinue
    if ($matches) {
        $paths = ($matches | ForEach-Object Path) -join [Environment]::NewLine
        throw "Developer-local paths leaked into packaged assets:$([Environment]::NewLine)$paths"
    }

    $notices = [ordered]@{
        'PRODUCT-LICENSE.md' = (Join-Path $repo 'LICENSE.md')
        'IBM-PLEX-OFL-1.1.txt' = (Join-Path $gui 'assets\fonts\IBM-PLEX-LICENSE.txt')
        'NATIVE-SDK-APACHE-2.0.txt' = (Join-Path $gui 'node_modules\@native-sdk\cli\LICENSE')
    }
    $noticeRoot = if (Test-Path -LiteralPath (Join-Path $package 'bin\sd300-gui.exe') -PathType Leaf) {
        Join-Path $package 'bin\licenses'
    }
    elseif (Test-Path -LiteralPath (Join-Path $package 'sd300-gui.exe') -PathType Leaf) {
        Join-Path $package 'licenses'
    }
    else {
        throw "Packaged GUI root has neither the build-tree nor archive-tree executable layout: $package"
    }
    foreach ($entry in $notices.GetEnumerator()) {
        $packaged = Join-Path $noticeRoot $entry.Key
        if (-not (Test-Path -LiteralPath $packaged -PathType Leaf)) {
            throw "Packaged GUI is missing required notice: $($entry.Key)."
        }
        $sourceHash = (Get-FileHash -LiteralPath $entry.Value -Algorithm SHA256).Hash
        $packageHash = (Get-FileHash -LiteralPath $packaged -Algorithm SHA256).Hash
        if ($sourceHash -ne $packageHash) {
            throw "Packaged GUI notice does not match its reviewed source: $($entry.Key)."
        }
    }
}

Write-Output "Native SDK distribution pins and path-leak checks passed."
