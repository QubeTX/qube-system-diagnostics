#!/usr/bin/env bash

set -euo pipefail

archive=${1:-}
version=${2:-}
[[ -f $archive && -n $version ]] || {
  echo "usage: $0 <sd300-gui-linux-*.tar.xz> <version>" >&2
  exit 64
}
archive_name=$(basename "$archive")
case "$archive_name" in
  sd300-gui-linux-gnu-x86_64.tar.xz) expected_target=linux-gnu-x86_64; expected_arch=x86_64; expected_machine='Advanced Micro Devices X86-64'; expected_interpreter='ld-linux-x86-64.so.2' ;;
  sd300-gui-linux-gnu-arm64.tar.xz) expected_target=linux-gnu-arm64; expected_arch=aarch64; expected_machine='AArch64'; expected_interpreter='ld-linux-aarch64.so.1' ;;
  sd300-gui-linux-musl-x86_64.tar.xz) expected_target=linux-musl-x86_64; expected_arch=x86_64; expected_machine='Advanced Micro Devices X86-64'; expected_interpreter='ld-musl-x86_64.so.1' ;;
  *) echo "unsupported Linux GUI archive name: $archive_name" >&2; exit 64 ;;
esac
for command in dbus-run-session jq ldd python3 readelf tar timeout Xvfb; do
  command -v "$command" >/dev/null || { echo "$command is required" >&2; exit 1; }
done

work=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/sd300-linux-validate.XXXXXXXX")
xvfb_pid=''
cleanup() {
  if [[ -n $xvfb_pid ]]; then kill "$xvfb_pid" >/dev/null 2>&1 || true; fi
  rm -rf "$work"
}
trap cleanup EXIT INT TERM

# Reject links and special files before extraction so an archive cannot write
# through an owned-looking path to somewhere outside the disposable root.
python3 - "$archive" <<'PY'
import pathlib
import sys
import tarfile

archive = pathlib.Path(sys.argv[1])
with tarfile.open(archive, "r:xz") as package:
    for member in package.getmembers():
        path = pathlib.PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts:
            raise SystemExit(f"GUI archive contains an unsafe path: {member.name}")
        if not path.parts or path.parts[0] != "sd300":
            raise SystemExit(f"GUI archive contains a file outside its owned root: {member.name}")
        if not (member.isdir() or member.isfile()):
            raise SystemExit(f"GUI archive contains a link or special file: {member.name}")
PY

members=$(tar -tJf "$archive")
grep -Eq '(^/|(^|/)\.\.(/|$))' <<< "$members" && {
  echo 'GUI archive contains an unsafe path' >&2
  exit 1
}
grep -Eqv '^sd300(/|$)' <<< "$members" && {
  echo 'GUI archive contains files outside its owned root' >&2
  exit 1
}
tar -xJf "$archive" -C "$work"
root="$work/sd300"
entry="$root/bin/sd300-gui"
runtime="$root/lib/runtime"
[[ -x $entry && -x $root/libexec/sd300-gui && -f $root/libexec/libsd300_engine.so ]]
for notice in PRODUCT-LICENSE.md IBM-PLEX-OFL-1.1.txt NATIVE-SDK-APACHE-2.0.txt; do
  [[ -s "$root/share/licenses/sd300/$notice" ]] || {
    echo "GUI package is missing required notice: $notice" >&2
    exit 1
  }
done
[[ -s "$root/THIRD_PARTY_NOTICES.txt" ]] || {
  echo 'GUI package is missing Linux third-party notices' >&2
  exit 1
}
[[ -d "$root/share/licenses/runtime" ]] || {
  echo 'GUI package is missing its private-runtime license evidence' >&2
  exit 1
}

python3 - "$root" "$expected_target" "$version" <<'PY'
import hashlib
import json
import os
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1]).resolve()
target = sys.argv[2]
version = sys.argv[3]

def safe_path(value, field):
    if not isinstance(value, str) or not value or "\\" in value:
        raise SystemExit(f"{field} is not a portable relative path: {value!r}")
    path = pathlib.PurePosixPath(value)
    if (path.is_absolute() or "." in path.parts or ".." in path.parts
            or path.as_posix() != value):
        raise SystemExit(f"{field} is not a safe canonical path: {value!r}")
    return path

def regular_files():
    files = {}
    for directory, directories, names in os.walk(root, followlinks=False):
        directory_path = pathlib.Path(directory)
        for name in [*directories, *names]:
            candidate = directory_path / name
            if candidate.is_symlink():
                raise SystemExit(f"package contains a symbolic link: {candidate.relative_to(root)}")
        for name in names:
            candidate = directory_path / name
            if not candidate.is_file():
                raise SystemExit(f"package contains a non-regular file: {candidate.relative_to(root)}")
            files[candidate.relative_to(root).as_posix()] = candidate
    return files

files = regular_files()
manifest_path = root / "install-manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if not (
    manifest.get("schema") == 1
    and manifest.get("product") == "SD-300"
    and manifest.get("version") == version
    and manifest.get("target") == target
    and manifest.get("entrypoint") == "bin/sd300-gui"
    and manifest.get("engine") == "libexec/libsd300_engine.so"
):
    raise SystemExit("GUI install manifest identity does not match the archive")

records = manifest.get("files")
if not isinstance(records, list) or not records:
    raise SystemExit("GUI install manifest has no file records")
declared = {}
for record in records:
    if not isinstance(record, dict):
        raise SystemExit("GUI install manifest contains a non-object file record")
    name = safe_path(record.get("path"), "install manifest path").as_posix()
    if name in declared:
        raise SystemExit(f"GUI install manifest repeats a file: {name}")
    if not isinstance(record.get("size"), int) or record["size"] < 0:
        raise SystemExit(f"GUI install manifest has an invalid size for {name}")
    digest = record.get("sha256")
    if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
        raise SystemExit(f"GUI install manifest has an invalid SHA-256 for {name}")
    declared[name] = record

actual_names = set(files) - {"install-manifest.json"}
if set(declared) != actual_names:
    missing = sorted(actual_names - set(declared))
    extra = sorted(set(declared) - actual_names)
    raise SystemExit(f"GUI install manifest inventory mismatch; missing={missing}, extra={extra}")
for name, record in declared.items():
    payload = files[name].read_bytes()
    if len(payload) != record["size"] or hashlib.sha256(payload).hexdigest() != record["sha256"]:
        raise SystemExit(f"GUI install manifest digest mismatch: {name}")

spdx_name = "runtime-components.spdx.json"
spdx = json.loads(files[spdx_name].read_text(encoding="utf-8"))
if not (
    spdx.get("spdxVersion") == "SPDX-2.3"
    and spdx.get("dataLicense") == "CC0-1.0"
    and spdx.get("SPDXID") == "SPDXRef-DOCUMENT"
    and spdx.get("name") == f"SD-300-{target}-{version}"
    and spdx.get("documentNamespace") == f"https://github.com/QubeTX/qube-system-diagnostics/spdx/{version}/{target}"
):
    raise SystemExit("Linux private-runtime SPDX identity does not match the archive")
spdx_records = spdx.get("files")
if not isinstance(spdx_records, list) or not spdx_records:
    raise SystemExit("Linux private-runtime SPDX has no file inventory")
spdx_files = {}
for record in spdx_records:
    name = safe_path(record.get("fileName"), "SPDX fileName").as_posix()
    if name in spdx_files:
        raise SystemExit(f"Linux private-runtime SPDX repeats a file: {name}")
    checksums = record.get("checksums")
    if not isinstance(checksums, list):
        raise SystemExit(f"Linux private-runtime SPDX has no checksum for {name}")
    sha256 = next((item.get("checksumValue") for item in checksums
                   if isinstance(item, dict) and item.get("algorithm") == "SHA256"), None)
    if not isinstance(sha256, str) or re.fullmatch(r"[0-9a-f]{64}", sha256) is None:
        raise SystemExit(f"Linux private-runtime SPDX has an invalid SHA-256 for {name}")
    spdx_files[name] = sha256

expected_spdx_files = set(files) - {"install-manifest.json", spdx_name}
if set(spdx_files) != expected_spdx_files:
    raise SystemExit("Linux private-runtime SPDX inventory does not cover the packaged payload")
for name, digest in spdx_files.items():
    if hashlib.sha256(files[name].read_bytes()).hexdigest() != digest:
        raise SystemExit(f"Linux private-runtime SPDX digest mismatch: {name}")

packages = spdx.get("packages")
if not isinstance(packages, list) or len(packages) < 2:
    raise SystemExit("Linux private-runtime SPDX has no dependency package inventory")
package_ids = {}
product_id = "SPDXRef-Package-SD300-Linux-Runtime"
for package in packages:
    if not isinstance(package, dict):
        raise SystemExit("Linux private-runtime SPDX contains a non-object package")
    package_id = package.get("SPDXID")
    if not isinstance(package_id, str) or not package_id.startswith("SPDXRef-"):
        raise SystemExit("Linux private-runtime SPDX package has an invalid SPDXID")
    if package_id in package_ids:
        raise SystemExit(f"Linux private-runtime SPDX repeats package {package_id}")
    package_ids[package_id] = package
if product_id not in package_ids:
    raise SystemExit("Linux private-runtime SPDX is missing the aggregate runtime package")

runtime_package_ids = set(package_ids) - {product_id}
for package_id in runtime_package_ids:
    package = package_ids[package_id]
    if not all(isinstance(package.get(field), str) and package[field]
               for field in ("name", "versionInfo", "supplier", "licenseComments")):
        raise SystemExit(f"runtime dependency package metadata is incomplete: {package_id}")
    if package.get("licenseConcluded") != "NOASSERTION" or package.get("licenseDeclared") != "NOASSERTION":
        raise SystemExit(f"runtime dependency makes an unaudited SPDX license assertion: {package_id}")
    attributions = package.get("attributionTexts")
    prefix = "Bundled license evidence: "
    if not isinstance(attributions, list) or len(attributions) != 1 \
            or not isinstance(attributions[0], str) or not attributions[0].startswith(prefix):
        raise SystemExit(f"runtime dependency has no machine-readable license evidence path: {package_id}")
    evidence = safe_path(attributions[0][len(prefix):], "license evidence path").as_posix()
    evidence_path = root / evidence
    if not evidence_path.is_dir() or not any(item.is_file() and item.stat().st_size > 0
                                             for item in evidence_path.rglob("*")):
        raise SystemExit(f"runtime dependency license evidence is absent or empty: {evidence}")

relationships = spdx.get("relationships")
if not isinstance(relationships, list) or not relationships:
    raise SystemExit("Linux private-runtime SPDX has no relationships")
edges = set()
for relationship in relationships:
    if not isinstance(relationship, dict):
        raise SystemExit("Linux private-runtime SPDX contains a non-object relationship")
    edge = (relationship.get("spdxElementId"), relationship.get("relationshipType"),
            relationship.get("relatedSpdxElement"))
    if not all(isinstance(value, str) and value for value in edge):
        raise SystemExit("Linux private-runtime SPDX has an invalid relationship")
    edges.add(edge)
if ("SPDXRef-DOCUMENT", "DESCRIBES", product_id) not in edges:
    raise SystemExit("Linux private-runtime SPDX does not describe its aggregate package")
for record in spdx_records:
    if (product_id, "CONTAINS", record["SPDXID"]) not in edges:
        raise SystemExit(f"aggregate runtime package does not contain {record['fileName']}")
for package_id in runtime_package_ids:
    if (product_id, "CONTAINS", package_id) not in edges:
        raise SystemExit(f"aggregate runtime package does not contain {package_id}")

file_ids_by_name = {record["fileName"]: record["SPDXID"] for record in spdx_records}
owned_runtime_files = {
    related for source, relationship, related in edges
    if source in runtime_package_ids and relationship == "CONTAINS"
}
for name, file_id in file_ids_by_name.items():
    package_owned = (
        (name.startswith("lib/runtime/lib/") and re.search(r"\\.so(?:\\.|$)", name))
        or (name.startswith("lib/runtime/share/glib-2.0/schemas/") and name.endswith(".xml"))
    )
    if package_owned and file_id not in owned_runtime_files:
        raise SystemExit(f"runtime dependency file is not attributed to a distro package: {name}")
PY

cache="$runtime/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
if [[ -f $cache ]]; then
  sed "s#@SD300_RUNTIME@#${runtime}#g" "$cache" > "$cache.configured"
  mv "$cache.configured" "$cache"
fi
desktop="$root/share/applications/sd300.desktop"
grep -Fq 'Exec=@SD300_GUI@' "$desktop"
sed "s#@SD300_GUI@#${entry}#g" "$desktop" > "$desktop.configured"
mv "$desktop.configured" "$desktop"
grep -Fqx "Exec=$entry" "$desktop"
grep -Fqx 'Terminal=false' "$desktop"

for forbidden in 'libc.so*' 'ld-linux*' 'ld-musl*' 'libGL.so*' 'libEGL.so*' 'libdrm.so*'; do
  if find "$runtime/lib" -type f -name "$forbidden" -print -quit | grep -q .; then
    echo "private runtime contains forbidden host/system component: $forbidden" >&2
    exit 1
  fi
done
for object in "$root/libexec/sd300-gui" "$root/libexec/libsd300_engine.so"; do
  if [[ -n ${HOME:-} ]] && readelf -d "$object" | grep -E '(RPATH|RUNPATH)' | grep -Fq "$HOME"; then
    echo "RPATH leaks the build user home: $object" >&2
    exit 1
  fi
  dependency_report=$(LD_LIBRARY_PATH="$runtime/lib" ldd "$object" 2>&1)
  printf '%s\n' "$dependency_report"
  ! grep -Fq 'not found' <<< "$dependency_report"
done
gui_dependencies=$(LD_LIBRARY_PATH="$runtime/lib" ldd "$root/libexec/sd300-gui" 2>&1)
grep -E 'libgtk-4\.so' <<< "$gui_dependencies" | grep -Fq "$runtime"
grep -Eq "Machine:[[:space:]]+${expected_machine}$" < <(readelf -h "$root/libexec/sd300-gui")
grep -Eq "Machine:[[:space:]]+${expected_machine}$" < <(readelf -h "$root/libexec/libsd300_engine.so")
grep -E 'Requesting program interpreter:' < <(readelf -l "$root/libexec/sd300-gui") | grep -Fq "$expected_interpreter"

self_test=$($entry --self-test --json)
jq -e --arg version "$version" --arg arch "$expected_arch" '
  .success == true and .product == "SD-300" and
  .product_version == $version and .abi_version == 1 and
  .engine_schema_version == 1 and .target_os == "linux" and
  .target_arch == $arch
' <<< "$self_test" >/dev/null

Xvfb :99 -screen 0 1600x1000x24 -nolisten tcp > "$work/xvfb.log" 2>&1 &
xvfb_pid=$!
for attempt in {1..30}; do
  [[ -S /tmp/.X11-unix/X99 ]] && break
  kill -0 "$xvfb_pid" 2>/dev/null || { cat "$work/xvfb.log" >&2; exit 1; }
  [[ $attempt -lt 30 ]] || { echo 'Xvfb did not become ready' >&2; exit 1; }
  sleep 1
done

mkdir -m 700 "$work/runtime-dir"
DISPLAY=:99 XDG_RUNTIME_DIR="$work/runtime-dir" dbus-run-session -- \
  bash -s -- "$entry" "$work/runtime-dir/sd300/gui.sock" "$work/gui.log" <<'INNER'
set -euo pipefail
entry=$1
socket=$2
log=$3
"$entry" >"$log" 2>&1 &
gui_pid=$!
cleanup_inner() { kill "$gui_pid" >/dev/null 2>&1 || true; }
trap cleanup_inner EXIT INT TERM
for attempt in {1..30}; do
  [[ -S $socket ]] && break
  kill -0 "$gui_pid" 2>/dev/null || { cat "$log" >&2; exit 1; }
  [[ $attempt -lt 30 ]] || { echo 'GUI lifecycle socket did not become ready' >&2; exit 1; }
  sleep 1
done

# A second app launch must focus the existing instance and exit, not create a
# second collector/renderer process.
timeout 10 "$entry"
kill -0 "$gui_pid"

python3 - "$socket" <<'PY'
import socket, sys
client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
client.settimeout(5)
client.connect(sys.argv[1])
client.sendall(b"quit\n")
client.close()
PY
for attempt in {1..30}; do
  if ! kill -0 "$gui_pid" 2>/dev/null; then break; fi
  [[ $attempt -lt 30 ]] || { echo 'GUI did not exit through its lifecycle socket' >&2; exit 1; }
  sleep 1
done
wait "$gui_pid"
trap - EXIT INT TERM
INNER

echo "Linux native GUI package passed blank-host self-test, visible launch, single-instance focus, and graceful quit: $archive"
