#!/usr/bin/env bash

set -euo pipefail

target=${1:-}
skip_tests=${SD300_SKIP_NATIVE_TESTS:-0}
skip_npm_ci=${SD300_SKIP_NPM_CI:-0}
case "$target" in
  macos-x86_64) host_os=Darwin; zig_target=x86_64-macos; rust_target=x86_64-apple-darwin; engine=libsd300_engine.dylib ;;
  macos-arm64) host_os=Darwin; zig_target=aarch64-macos; rust_target=aarch64-apple-darwin; engine=libsd300_engine.dylib ;;
  linux-gnu-x86_64) host_os=Linux; zig_target=x86_64-linux-gnu; rust_target=x86_64-unknown-linux-gnu; engine=libsd300_engine.so ;;
  linux-gnu-arm64) host_os=Linux; zig_target=aarch64-linux-gnu; rust_target=aarch64-unknown-linux-gnu; engine=libsd300_engine.so ;;
  linux-musl-x86_64) host_os=Linux; zig_target=x86_64-linux-musl; rust_target=x86_64-unknown-linux-musl; engine=libsd300_engine.so ;;
  *) echo "unsupported SD-300 GUI target: $target" >&2; exit 64 ;;
esac

[[ $(uname -s) == "$host_os" ]] || { echo "$target requires a $host_os host" >&2; exit 1; }
[[ $(zig version) == 0.16.0 ]] || { echo 'Zig 0.16.0 is required' >&2; exit 1; }
rust_version=$(rustc --version | awk '{print $2}')
[[ $rust_version == 1.95.0 ]] || { echo "Rust 1.95.0 is required; found $rust_version" >&2; exit 1; }
rust_host=$(rustc -vV | awk '/^host:/ {print $2}')
[[ $rust_host == "$rust_target" ]] || { echo "$target requires native Rust host $rust_target; found $rust_host" >&2; exit 1; }

script_root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH='' cd -- "$script_root/.." && pwd)
gui_root="$repo_root/gui"
engine_root="$repo_root/gui-engine"

node "$script_root/prepare-makira-font.mjs" "$gui_root"
if [[ $skip_npm_ci != 1 ]]; then
  (cd "$gui_root" && npm ci --ignore-scripts)
fi
(cd "$gui_root" && npx --no-install native check .)
patch_receipt=$(node "$script_root/prepare-native-sdk.mjs" "$gui_root")

profile_root=${HOME:-}
remap="--remap-path-prefix=$repo_root=/src/sd300"
if [[ -n $profile_root ]]; then remap="$remap --remap-path-prefix=$profile_root=/build-user"; fi
rust_flags="${RUSTFLAGS:-} $remap"
if [[ $target == linux-musl-x86_64 ]]; then
  # The stock musl target defaults to a static C runtime, which makes rustc
  # reject cdylib output even though the target explicitly supports dynamic
  # linking. Alpine supplies the matching dynamic musl runtime; disabling only
  # crt-static keeps the same C ABI/shared-engine architecture as GNU Linux.
  rust_flags="$rust_flags -C target-feature=-crt-static"
fi
RUSTFLAGS="$rust_flags" cargo build \
  --manifest-path "$engine_root/Cargo.toml" --release --locked --target "$rust_target"
engine_artifact="$engine_root/target/$rust_target/release/$engine"
[[ -f $engine_artifact ]] || { echo "engine missing: $engine_artifact" >&2; exit 1; }
if [[ $host_os == Darwin ]]; then
  # Rust cdylibs can retain their build output as LC_ID_DYLIB. Give the
  # companion a stable bundle-relative identity before package signing so no
  # runner or developer checkout path reaches the shipped Mach-O image.
  install_name_tool -id "@rpath/$engine" "$engine_artifact"
fi

stage_base="$repo_root/target/native-gui-stage"
stage_root="$stage_base/$target"
app_stage="$stage_root/app"
sdk_stage="$stage_root/sdk"
case "$stage_root" in
  "$stage_base"/*) ;;
  *) echo "refusing to prepare a native GUI stage outside $stage_base" >&2; exit 1 ;;
esac
rm -rf "$stage_root"
mkdir -p "$app_stage"
cp "$gui_root/build.zig" "$gui_root/app.zon" "$gui_root/README.md" "$app_stage/"
cp -R "$gui_root/src" "$gui_root/assets" "$gui_root/platform" "$app_stage/"
cp -R "$gui_root/node_modules/@native-sdk/cli" "$sdk_stage"
cp "$engine_artifact" "$app_stage/$engine"
cat > "$app_stage/build.zig.zon" <<'ZON'
.{
    .name = .gui,
    .fingerprint = 0xd4ff50f85a707070,
    .version = "3.0.0",
    .minimum_zig_version = "0.16.0",
    .dependencies = .{ .native_sdk = .{ .path = "../sdk" } },
    .paths = .{ "build.zig", "build.zig.zon", "src", "assets", "platform", "app.zon", "README.md" },
}
ZON

if [[ $skip_tests != 1 ]]; then
  (cd "$gui_root" && npx --no-install native check . --strict)
fi
# Every supported non-Windows lane runs on its exact target architecture and
# libc. Let Zig resolve that native ABI so its C compiler uses one coherent set
# of host GTK/libc headers. The Rust-host equality check above still fails
# closed if a workflow schedules the wrong architecture or libc.
zig_build_target=native
zig_build_args=(-Dtarget="$zig_build_target" -Dcpu=baseline -Doptimize=ReleaseFast)
if [[ $host_os == Linux ]]; then
  gtk_lib_dir=$(pkg-config --variable=libdir gtk4)
  [[ -n $gtk_lib_dir && -d $gtk_lib_dir ]] || {
    echo "pkg-config did not resolve a usable GTK4 library directory: $gtk_lib_dir" >&2
    exit 1
  }
  zig_build_args+=("-Dsystem-lib-dir=$gtk_lib_dir")
elif [[ $host_os == Darwin ]]; then
  macos_sdk=$(xcrun --sdk macosx --show-sdk-path)
  [[ -n $macos_sdk && -d $macos_sdk ]] || {
    echo "xcrun did not resolve a usable macOS SDK: $macos_sdk" >&2
    exit 1
  }
  macos_include_dir="$macos_sdk/usr/include"
  [[ -d $macos_include_dir ]] || {
    echo "macOS SDK system include directory is missing: $macos_include_dir" >&2
    exit 1
  }
  zig_build_args+=("-Dsystem-include-dir=$macos_include_dir")

  # Xcode SDK layouts have carried libDER either under usr/include or as a
  # private/nested framework. Zig explicit-target builds need the latter's
  # parent supplied directly; never fall back to a host-global header path.
  if [[ ! -f $macos_include_dir/libDER/DERItem.h ]]; then
    libder_framework=$(find "$macos_sdk/System/Library" -type d -name libDER.framework -print -quit)
    [[ -n $libder_framework && -d $libder_framework/Headers ]] || {
      echo "macOS SDK does not contain a usable libDER header or framework: $macos_sdk" >&2
      exit 1
    }
    zig_build_args+=("-Dsystem-framework-dir=$(dirname "$libder_framework")")
  fi
fi
zig_log="$stage_root/zig-build-first-attempt.log"
set +e
(cd "$app_stage" && zig build "${zig_build_args[@]}") 2>&1 | tee "$zig_log"
zig_status=${PIPESTATUS[0]}
set -e
if [[ $zig_status -ne 0 ]]; then
  cache_miss='failed to check cache:[[:space:]]+.*\.zig-cache[/\\]+o[/\\]+[0-9a-f]+[/\\]+dependencies\.zig.*[[:space:]]+file_hash FileNotFound'
  if ! grep -Eq "$cache_miss" "$zig_log"; then
    exit "$zig_status"
  fi
  echo 'Zig 0.16.0 reported its known first-use staged dependency-cache miss; retrying exactly once.' >&2
  (cd "$app_stage" && zig build "${zig_build_args[@]}")
fi
rm -f "$zig_log"
if [[ $skip_tests != 1 ]]; then
  # Exercise the release-shaped staged graph through the public Native SDK
  # command while keeping the checked-in dependency URL/hash pinned.
  (cd "$gui_root" && npx --no-install native test "$app_stage" --yes "${zig_build_args[@]}")
fi
notices="$app_stage/zig-out/bin/licenses"
mkdir -p "$notices"
install -m 644 "$repo_root/LICENSE.md" "$notices/PRODUCT-LICENSE.md"
install -m 644 "$gui_root/assets/fonts/IBM-PLEX-LICENSE.txt" "$notices/IBM-PLEX-OFL-1.1.txt"
install -m 644 "$sdk_stage/LICENSE" "$notices/NATIVE-SDK-APACHE-2.0.txt"
find "$app_stage/zig-out" -type f -name '*.pdb' -delete 2>/dev/null || true
find "$app_stage/zig-out" -type d -name '*.dSYM' -prune -exec rm -rf {} + 2>/dev/null || true
node "$script_root/check-native-distribution.mjs" "$gui_root" "$app_stage/zig-out"

engine_sha=$(sha256sum "$engine_artifact" | awk '{print $1}')
node -e 'const r=JSON.parse(process.argv[1]); console.log(JSON.stringify({schema:1,target:process.argv[2],zig_target:process.argv[3],zig_build_target:process.argv[4],zig_cpu:"baseline",zig_optimize:"ReleaseFast",zig_version:"0.16.0",rust_version:process.argv[5],rust_target:process.argv[6],rust_host:process.argv[6],native_sdk_cli:"0.5.4",native_sdk_patch:r.renderer_patch_sha256,engine_sha256:process.argv[7],package_root:process.argv[8]}))' \
  "$patch_receipt" "$target" "$zig_target" "$zig_build_target" "$rust_version" "$rust_target" "$engine_sha" "$app_stage/zig-out"
