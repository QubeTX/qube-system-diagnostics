#!/usr/bin/env bash

set -euo pipefail

version=${1:-}
output_dir=${2:-}
[[ -n $version && -n $output_dir ]] || {
  echo "usage: $0 <version> <output-dir>" >&2
  exit 64
}
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || {
  echo 'the Alpine musl qualification build requires native Linux x86_64' >&2
  exit 1
}
grep -Fqx '3.20' /etc/alpine-release || {
  echo "the musl package must be built in Alpine 3.20; found $(cat /etc/alpine-release)" >&2
  exit 1
}

apk add --no-cache \
  bash build-base ca-certificates curl findutils git gtk4.0-dev nodejs npm \
  pax-utils patchelf pkgconf xz

zig_archive=zig-x86_64-linux-0.16.0.tar.xz
zig_url=https://ziglang.org/download/0.16.0/zig-x86_64-linux-0.16.0.tar.xz
zig_sha=70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00
tool_root=${RUNNER_TEMP:-/tmp}/sd300-toolchains
mkdir -p "$tool_root"
curl --proto '=https' --tlsv1.2 -fsSLo "$tool_root/$zig_archive" \
  "$zig_url"
printf '%s  %s\n' "$zig_sha" "$tool_root/$zig_archive" | sha256sum -c -
tar -xJf "$tool_root/$zig_archive" -C "$tool_root"
export PATH="$tool_root/zig-x86_64-linux-0.16.0:$PATH"

curl --proto '=https' --tlsv1.2 -fsS https://sh.rustup.rs | \
  sh -s -- -y --profile minimal --default-toolchain 1.95
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
[[ $(rustc --version | awk '{print $2}') == 1.95.0 ]] || {
  echo "Rust 1.95.0 is required for the musl release lane" >&2
  exit 1
}
[[ $(rustc -vV | awk '/^host:/ {print $2}') == x86_64-unknown-linux-musl ]]

script_root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH='' cd -- "$script_root/.." && pwd)
npm_cache=${RUNNER_TEMP:-/tmp}/sd300-native-npm-cache
rm -rf "$repo_root/gui/node_modules" "$npm_cache"
npm --prefix "$repo_root/gui" ci --ignore-scripts --cache "$npm_cache"
node "$script_root/prepare-native-sdk.mjs" "$repo_root/gui"
node "$script_root/check-native-distribution.mjs" "$repo_root/gui"
rm -rf "$repo_root/gui/node_modules"
npm --prefix "$repo_root/gui" ci --ignore-scripts --offline --cache "$npm_cache"
node "$script_root/prepare-native-sdk.mjs" "$repo_root/gui"
node "$script_root/check-native-distribution.mjs" "$repo_root/gui"
SD300_SKIP_NPM_CI=1 bash "$script_root/build-native-gui.sh" linux-musl-x86_64
bash "$script_root/package-native-gui-linux.sh" \
  linux-musl-x86_64 "$output_dir" "$version"
