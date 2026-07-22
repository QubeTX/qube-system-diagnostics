[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet(
        "windows-x86_64",
        "macos-x86_64",
        "macos-arm64",
        "linux-gnu-x86_64",
        "linux-gnu-arm64",
        "linux-musl-x86_64"
    )]
    [string]$Target,

    [switch]$SkipNpmCi,
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$targets = @{
    "windows-x86_64" = @{
        HostOS = "windows"
        Zig = "x86_64-windows"
        Rust = "x86_64-pc-windows-msvc"
        Engine = "sd300_engine.dll"
    }
    "macos-x86_64" = @{
        HostOS = "macos"
        Zig = "x86_64-macos"
        Rust = "x86_64-apple-darwin"
        Engine = "libsd300_engine.dylib"
    }
    "macos-arm64" = @{
        HostOS = "macos"
        Zig = "aarch64-macos"
        Rust = "aarch64-apple-darwin"
        Engine = "libsd300_engine.dylib"
    }
    "linux-gnu-x86_64" = @{
        HostOS = "linux"
        Zig = "x86_64-linux-gnu"
        Rust = "x86_64-unknown-linux-gnu"
        Engine = "libsd300_engine.so"
    }
    "linux-gnu-arm64" = @{
        HostOS = "linux"
        Zig = "aarch64-linux-gnu"
        Rust = "aarch64-unknown-linux-gnu"
        Engine = "libsd300_engine.so"
    }
    "linux-musl-x86_64" = @{
        HostOS = "linux"
        Zig = "x86_64-linux-musl"
        Rust = "x86_64-unknown-linux-musl"
        Engine = "libsd300_engine.so"
    }
}

$contract = $targets[$Target]
$hostOS = if ($IsWindows) { "windows" } elseif ($IsMacOS) { "macos" } elseif ($IsLinux) { "linux" } else { "unknown" }
if ($hostOS -ne $contract.HostOS) {
    throw "Target '$Target' requires a $($contract.HostOS) host; current host is '$hostOS'. SD-300 release lanes build and test on their native OS."
}

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$guiRoot = Join-Path $repoRoot "gui"
$engineRoot = Join-Path $repoRoot "gui-engine"

& node (Join-Path $PSScriptRoot "prepare-makira-font.mjs") $guiRoot
if ($LASTEXITCODE -ne 0) {
    throw "Preparing the licensed Makira build input failed."
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)] [string]$FilePath,
        [Parameter(ValueFromRemainingArguments = $true)] [string[]]$Arguments
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed ($LASTEXITCODE): $FilePath $($Arguments -join ' ')"
    }
}

function Invoke-StagedZigBuild {
    param(
        [Parameter(Mandatory = $true)] [string]$WorkingDirectory,
        [Parameter(Mandatory = $true)] [string[]]$Arguments
    )

    Push-Location $WorkingDirectory
    try {
        $firstOutput = @(& zig @Arguments 2>&1)
        $firstExit = $LASTEXITCODE
        $firstOutput | ForEach-Object { Write-Host $_ }
        if ($firstExit -eq 0) {
            return
        }

        # Zig 0.16.0 can report a first-use local dependency-cache miss after
        # materializing a path dependency, then complete from that same
        # partially populated cache on the immediate retry. Retry only this
        # exact disposable-stage condition; every compiler/linker/test error
        # still fails closed without a second attempt.
        $transcript = ($firstOutput | ForEach-Object { $_.ToString() }) -join "`n"
        $cacheMiss = 'failed to check cache:\s+.*\.zig-cache[\\/]+o[\\/]+[0-9a-f]+[\\/]+dependencies\.zig.*\s+file_hash FileNotFound'
        if ($transcript -notmatch $cacheMiss) {
            throw "Command failed ($firstExit): zig $($Arguments -join ' ')"
        }

        Write-Warning "Zig 0.16.0 reported its known first-use staged dependency-cache miss; retrying exactly once."
        & zig @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "Command failed after the one permitted cache retry ($LASTEXITCODE): zig $($Arguments -join ' ')"
        }
    }
    finally {
        Pop-Location
    }
}

$zigVersion = (& zig version).Trim()
if ($LASTEXITCODE -ne 0 -or $zigVersion -ne "0.16.0") {
    throw "SD-300 Native SDK builds require Zig 0.16.0; found '$zigVersion'."
}

$rustInfo = (& rustc -vV) -join "`n"
if ($LASTEXITCODE -ne 0) {
    throw "rustc is required to build the SD-300 GUI engine."
}
$rustVersion = ((& rustc --version) -split '\s+')[1]
if ($LASTEXITCODE -ne 0 -or $rustVersion -ne "1.95.0") {
    throw "SD-300 Native SDK builds require Rust 1.95.0; found '$rustVersion'."
}
$rustHost = ([regex]::Match($rustInfo, '(?m)^host:\s*(\S+)\s*$')).Groups[1].Value
if ($rustHost -ne $contract.Rust) {
    throw "Target '$Target' requires native Rust host '$($contract.Rust)'; found '$rustHost'."
}

Push-Location $guiRoot
try {
    if (-not $SkipNpmCi) {
        Invoke-Checked npm ci --ignore-scripts
    }
    # Strict mode also verifies the generated model contract. A clean checkout
    # intentionally has not emitted it yet, so perform structural validation
    # here and strict validation after the test/build graph refreshes it.
    Invoke-Checked npx --no-install native check .
}
finally {
    Pop-Location
}

$prepareSdk = Join-Path $PSScriptRoot "prepare-native-sdk.ps1"
$patchReceipt = & $prepareSdk -GuiRoot $guiRoot
if ($LASTEXITCODE -ne 0) {
    throw "Preparing the reviewed Native SDK renderer dependency failed."
}

$cargoArgs = @(
    "build",
    "--manifest-path", (Join-Path $engineRoot "Cargo.toml"),
    "--release",
    "--locked",
    "--target", $contract.Rust
)
$buildProfileRoot = if ($env:USERPROFILE) { $env:USERPROFILE } elseif ($env:HOME) { $env:HOME } else { "" }
$previousRustFlags = Get-Item -LiteralPath "Env:RUSTFLAGS" -ErrorAction SilentlyContinue
$previousRustFlagsValue = if ($null -ne $previousRustFlags) { $previousRustFlags.Value } else { "" }
$remapFlags = @("--remap-path-prefix=$repoRoot=/src/sd300")
if ($buildProfileRoot) {
    $remapFlags += "--remap-path-prefix=$buildProfileRoot=/build-user"
}
$env:RUSTFLAGS = (($previousRustFlagsValue, ($remapFlags -join " ")) -join " ").Trim()
try {
    Invoke-Checked cargo @cargoArgs
}
finally {
    if ($null -ne $previousRustFlags) {
        $env:RUSTFLAGS = $previousRustFlags.Value
    }
    else {
        Remove-Item -LiteralPath "Env:RUSTFLAGS" -ErrorAction SilentlyContinue
    }
}

$engineArtifact = Join-Path $engineRoot "target\$($contract.Rust)\release\$($contract.Engine)"
if (-not (Test-Path -LiteralPath $engineArtifact -PathType Leaf)) {
    throw "Rust engine artifact was not produced at '$engineArtifact'."
}

# Build from a deterministic staging graph. The checked-in manifest keeps the
# reviewed npm tarball URL and Zig hash, while this generated graph points only
# at the project-local, lockfile-verified copy that prepare-native-sdk.ps1
# patched and hash-checked. No profile/global npm path enters the graph.
$stageBase = Join-Path $repoRoot "target\native-gui-stage"
$stageRoot = Join-Path $stageBase $Target
$appStage = Join-Path $stageRoot "app"
$sdkStage = Join-Path $stageRoot "sdk"
$stageBaseFull = [IO.Path]::GetFullPath($stageBase)
$stageRootFull = [IO.Path]::GetFullPath($stageRoot)
if (-not $stageRootFull.StartsWith($stageBaseFull + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to prepare a native GUI stage outside '$stageBaseFull': '$stageRootFull'."
}
if (Test-Path -LiteralPath $stageRootFull) {
    Remove-Item -LiteralPath $stageRootFull -Recurse -Force
}
New-Item -ItemType Directory -Path $appStage -Force | Out-Null

foreach ($file in @("build.zig", "app.zon", "README.md")) {
    Copy-Item -LiteralPath (Join-Path $guiRoot $file) -Destination (Join-Path $appStage $file) -Force
}
foreach ($directory in @("src", "assets", "platform")) {
    Copy-Item -LiteralPath (Join-Path $guiRoot $directory) -Destination (Join-Path $appStage $directory) -Recurse -Force
}
Copy-Item -LiteralPath (Join-Path $guiRoot "node_modules\@native-sdk\cli") -Destination $sdkStage -Recurse -Force
Copy-Item -LiteralPath $engineArtifact -Destination (Join-Path $appStage $contract.Engine) -Force

$stageZon = @'
.{
    .name = .gui,
    .fingerprint = 0xd4ff50f85a707070,
    .version = "3.0.0",
    .minimum_zig_version = "0.16.0",
    .dependencies = .{
        .native_sdk = .{ .path = "../sdk" },
    },
    .paths = .{ "build.zig", "build.zig.zon", "src", "assets", "platform", "app.zon", "README.md" },
}
'@
Set-Content -LiteralPath (Join-Path $appStage "build.zig.zon") -Value $stageZon -Encoding utf8NoBOM

Push-Location $guiRoot
try {
    if (-not $SkipTests) {
        Invoke-Checked npx --no-install native check . --strict
    }
}
finally {
    Pop-Location
}

$zigBuildArguments = @(
    "build",
    "-Dtarget=$($contract.Zig)",
    "-Dcpu=baseline",
    "-Doptimize=ReleaseFast",
    # Native SDK defaults to high-frequency runtime event tracing. Release
    # products retain panic capture and explicit self-tests but compile that
    # development telemetry out of the sink filter.
    "-Dtrace=off"
)
Invoke-StagedZigBuild -WorkingDirectory $appStage -Arguments $zigBuildArguments

if (-not $SkipTests) {
    # Drive the release-shaped staged graph through the public Native SDK
    # command. The checked-in graph stays URL/hash pinned; this generated
    # graph points only at the project-local, verified downstream patch.
    Push-Location $guiRoot
    try {
        $testArgs = @(
            "--no-install", "native", "test", $appStage, "--yes",
            "-Dtarget=$($contract.Zig)",
            "-Dcpu=baseline",
            "-Doptimize=ReleaseFast",
            "-Dtrace=off"
        )
        Invoke-Checked npx @testArgs
    }
    finally {
        Pop-Location
    }

    # `native test` emits the exact staged model contract. Re-run strict check
    # after that generation step so a stale preflight contract can never leave
    # a distribution build with only the structural fallback validated.
    Push-Location $guiRoot
    try {
        Invoke-Checked npx --no-install native check $appStage --strict
    }
    finally {
        Pop-Location
    }
}

$zigOutput = (Resolve-Path -LiteralPath (Join-Path $appStage "zig-out")).Path
if (-not $zigOutput.StartsWith($stageRootFull, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to clean debug symbols outside the native GUI stage: '$zigOutput'."
}
$notices = Join-Path $zigOutput "bin\licenses"
New-Item -ItemType Directory -Path $notices -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $repoRoot "LICENSE.md") -Destination (Join-Path $notices "PRODUCT-LICENSE.md") -Force
Copy-Item -LiteralPath (Join-Path $guiRoot "assets\fonts\IBM-PLEX-LICENSE.txt") -Destination (Join-Path $notices "IBM-PLEX-OFL-1.1.txt") -Force
Copy-Item -LiteralPath (Join-Path $sdkStage "LICENSE") -Destination (Join-Path $notices "NATIVE-SDK-APACHE-2.0.txt") -Force
Get-ChildItem -LiteralPath $zigOutput -Recurse -File -Filter "*.pdb" |
    ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force }

$distributionCheck = Join-Path $PSScriptRoot "check-native-distribution.ps1"
& $distributionCheck -GuiRoot $guiRoot -PackageRoot $zigOutput
if ($LASTEXITCODE -ne 0) {
    throw "Native distribution check failed."
}

$receipt = [ordered]@{
    schema = 1
    target = $Target
    zig_target = $contract.Zig
    zig_cpu = "baseline"
    zig_optimize = "ReleaseFast"
    native_sdk_trace = "off"
    zig_version = $zigVersion
    rust_version = $rustVersion
    rust_target = $contract.Rust
    rust_host = $rustHost
    native_sdk_cli = "0.5.4"
    native_sdk_patch = ($patchReceipt | ConvertFrom-Json).renderer_patch_sha256
    engine_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $engineArtifact).Hash.ToLowerInvariant()
    package_root = $zigOutput
}
$receipt | ConvertTo-Json | Write-Output
