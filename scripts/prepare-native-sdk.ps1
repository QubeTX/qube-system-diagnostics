[CmdletBinding()]
param(
    [string]$GuiRoot = (Join-Path $PSScriptRoot "..\gui")
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$gui = (Resolve-Path -LiteralPath $GuiRoot).Path
$packageLockPath = Join-Path $gui "package-lock.json"
$sdkRoot = Join-Path $gui "node_modules\@native-sdk\cli"
$patchPath = Join-Path $gui "patches\native-sdk-0.5.4-software-render.patch"

$requiredVersion = "0.5.4"
$requiredTarball = "https://registry.npmjs.org/@native-sdk/cli/-/cli-0.5.4.tgz"
$requiredIntegrity = "sha512-8ixE8TjN2zQ+9rnnpjOnmHDeloyvKBc9CKXVUdYxge63fSKn6AH3rodRcdE6EYQiAIDYzQiJSr8AKT1qdFcABA=="
$requiredZigHash = "native_sdk-0.1.0-hzDzQo8l5gCK6W8hPyRC4voBqyQU8bhy6ktUDXKIqWlb"
$requiredPatchHash = "557dacc4780967b73a5e12d02a805f5cba53c8e3687e9d5eacffb5547633d7df"

if (-not (Test-Path -LiteralPath $sdkRoot -PathType Container)) {
    throw "The project-local Native SDK is missing. Run npm ci in '$gui' first."
}
if (-not (Test-Path -LiteralPath $patchPath -PathType Leaf)) {
    throw "The reviewed Native SDK renderer patch is missing at '$patchPath'."
}
$actualPatchHash = (Get-FileHash -LiteralPath $patchPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualPatchHash -ne $requiredPatchHash) {
    throw "The Native SDK renderer patch does not match the reviewed toolchain record."
}

$lock = Get-Content -Raw -LiteralPath $packageLockPath | ConvertFrom-Json -AsHashtable
$locked = $lock['packages']['node_modules/@native-sdk/cli']
if ($null -eq $locked -or
    $locked['version'] -ne $requiredVersion -or
    $locked['resolved'] -ne $requiredTarball -or
    $locked['integrity'] -ne $requiredIntegrity) {
    throw "package-lock.json does not resolve the reviewed Native SDK 0.5.4 bytes."
}

$sdkPackage = Get-Content -Raw -LiteralPath (Join-Path $sdkRoot "package.json") | ConvertFrom-Json
if ($sdkPackage.version -ne $requiredVersion) {
    throw "Expected project-local Native SDK $requiredVersion; found '$($sdkPackage.version)'."
}

$files = [ordered]@{
    "build/app.zig" = @{
        Pristine = "0b224d560c66f0a111c1cc333c3f81002ab25811dabb81d85d174a89ed491595"
        Patched = "675bd4cd6552a4084eaf57856b2f681b02424eaa783a50e05a6bf7722dc2eb2c"
    }
    "src/app_runner/root.zig" = @{
        Pristine = "e085afe9f414a5ef0c21388e0bb1436bf05cb346349d6e87ca7e352c38b0c4e0"
        Patched = "5a3cbdbe53a4a68c93a49defb6024d20f335bda176697342a3c79163ce880340"
    }
    "src/platform/linux/gtk_host.c" = @{
        Pristine = "da73fa340df0f577cc09873ae0c6d5e6d94bc7ca8024a68ad51d2df94cd93af7"
        Patched = "772f4e3d01366e5b31ad138cab1e6977bfc6e081c6a962cbb27336fc7bd2e14f"
    }
    "src/platform/windows/webview2_host.cpp" = @{
        Pristine = "93d9843a411de4364310bbd4f87be19381c085828152b1c975249064d0c6e8a3"
        Patched = "843ff81775a7ae2aefb27f212b1c193779571a232ed72232d9f97e51606d902a"
    }
    "src/primitives/canvas/reference_memo.zig" = @{
        Pristine = "ebb7d49035d993b11b30c784e362f9cb12ed625a5e6ce19a44059bb20b34d592"
        Patched = "ea69ec3d6f4024062f4ac8aad88b4482258c8c0f4328dae7dcc33b89621b8196"
    }
    "src/primitives/canvas/reference.zig" = @{
        Pristine = "56ef9cec4f76ee6cbff8a56dc5f579d3b9ee2daa79ad4cdb1f40073c3a053ecb"
        Patched = "cf6068c5e7d2b9ffc4c8d28940be822348bf441646739e5b74238d433936fa8b"
    }
    "src/primitives/canvas/reference_tests.zig" = @{
        Pristine = "3accd42966c9465b28859cd73a33684619926d18082a32f7c6faac8b0f3b326a"
        Patched = "55a3e981de470b10ff67821e978e476eecab6fd6f607cc3945b30a81a6014f60"
    }
    "src/runtime/canvas_frame.zig" = @{
        Pristine = "d2eb5ff8c63a391a695a2a47bfef6c315ddafa98e7b35cd91253437eb066a0ce"
        Patched = "2678ff7cfb3d47c765d517d5b9c8eb1746985cb6b610e75da3bfd02c24eb0639"
    }
    "src/runtime/canvas_frame_patch_tests.zig" = @{
        Pristine = "c24d345ae4c26b073b84442bad64b2378ab7a4f424813df0b0a7d3ffcbb96d79"
        Patched = "c83a1327674e9e7b8b36accdaf8ed2e63ca140ec00a81c1f81347b375c6cf462"
    }
}

$state = $null
foreach ($entry in $files.GetEnumerator()) {
    $relative = $entry.Key.Replace('/', [IO.Path]::DirectorySeparatorChar)
    $path = Join-Path $sdkRoot $relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Native SDK source file is missing: '$($entry.Key)'."
    }
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    $fileState = if ($hash -eq $entry.Value.Pristine) {
        "pristine"
    }
    elseif ($hash -eq $entry.Value.Patched) {
        "patched"
    }
    else {
        throw "Native SDK source '$($entry.Key)' has unreviewed bytes ($hash)."
    }
    if ($null -eq $state) {
        $state = $fileState
    }
    elseif ($state -ne $fileState) {
        throw "Native SDK renderer sources are in a mixed pristine/patched state. Run npm ci and retry."
    }
}

if ($state -eq "pristine") {
    # The patch paths are rooted at the SDK package. --directory makes that
    # relationship explicit even though node_modules is ignored beneath the
    # SD-300 worktree; disabling autocrlf preserves the reviewed npm bytes.
    $repoRoot = (& git -C $gui rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or -not $repoRoot) {
        throw "Could not resolve the SD-300 worktree for Native SDK patching."
    }
    $sdkRelative = [IO.Path]::GetRelativePath($repoRoot, $sdkRoot).Replace('\', '/')
    if ($sdkRelative.StartsWith("../", [StringComparison]::Ordinal) -or [IO.Path]::IsPathRooted($sdkRelative)) {
        throw "Project-local Native SDK resolved outside the SD-300 worktree: '$sdkRoot'."
    }
    $directoryArg = "--directory=$sdkRelative"
    & git -c core.autocrlf=false -C $repoRoot apply $directoryArg --check --whitespace=nowarn $patchPath
    if ($LASTEXITCODE -ne 0) {
        throw "The reviewed Native SDK renderer patch no longer applies cleanly."
    }
    & git -c core.autocrlf=false -C $repoRoot apply $directoryArg --whitespace=nowarn $patchPath
    if ($LASTEXITCODE -ne 0) {
        throw "Applying the reviewed Native SDK renderer patch failed."
    }
}

foreach ($entry in $files.GetEnumerator()) {
    $relative = $entry.Key.Replace('/', [IO.Path]::DirectorySeparatorChar)
    $hash = (Get-FileHash -LiteralPath (Join-Path $sdkRoot $relative) -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -ne $entry.Value.Patched) {
        throw "Patched Native SDK verification failed for '$($entry.Key)'."
    }
}

[ordered]@{
    schema = 1
    native_sdk_cli = $requiredVersion
    tarball = $requiredTarball
    npm_integrity = $requiredIntegrity
    zig_content_hash = $requiredZigHash
    renderer_patch_sha256 = $requiredPatchHash
    source_state = "patched"
} | ConvertTo-Json
