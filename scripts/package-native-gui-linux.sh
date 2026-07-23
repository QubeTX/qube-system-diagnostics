#!/usr/bin/env bash

set -euo pipefail

target=${1:-}
output_dir=${2:-}
version=${3:-3.1.1}
case "$target" in
  linux-gnu-x86_64|linux-gnu-arm64|linux-musl-x86_64) ;;
  *) echo "unsupported Linux GUI package target: $target" >&2; exit 64 ;;
esac
[[ -n $output_dir ]] || { echo 'output directory is required' >&2; exit 64; }

script_root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH='' cd -- "$script_root/.." && pwd)
SOURCE_DATE_EPOCH=$(git -c safe.directory="$repo_root" -C "$repo_root" show -s --format=%ct HEAD)
export SOURCE_DATE_EPOCH
stage="$repo_root/target/native-gui-stage/$target/app/zig-out/bin"
binary="$stage/sd300-gui"
engine="$stage/libsd300_engine.so"
[[ -x $binary && -f $engine ]] || { echo "native GUI build is incomplete under $stage" >&2; exit 1; }
for command in patchelf node tar xz; do command -v "$command" >/dev/null || { echo "$command is required" >&2; exit 1; }; done
if command -v lddtree >/dev/null; then
  lddtree_command=lddtree
elif command -v lddtreepax >/dev/null; then
  # Alpine 3.20 splits the dependency-tree reader from pax-utils and exposes
  # it as `lddtreepax`; Ubuntu's pax-utils keeps the `lddtree` command.
  lddtree_command=lddtreepax
else
  echo 'lddtree or lddtreepax is required' >&2
  exit 1
fi

work=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/sd300-linux-package.XXXXXXXX")
trap 'rm -rf "$work"' EXIT INT TERM
root="$work/sd300"
runtime="$root/lib/runtime"
declare -A runtime_sources=()
mkdir -p "$root/bin" "$root/libexec/assets" "$runtime/lib" \
  "$runtime/share/glib-2.0/schemas" "$root/share/applications" \
  "$root/share/icons/hicolor/256x256/apps" "$root/share/licenses/sd300"
install -m 755 "$binary" "$root/libexec/sd300-gui"
install -m 755 "$engine" "$root/libexec/libsd300_engine.so"
install -m 644 "$repo_root/gui/assets/icon.png" "$root/libexec/assets/icon.png"
install -m 644 "$repo_root/gui/assets/icon.png" "$root/share/icons/hicolor/256x256/apps/sd300.png"
for notice in PRODUCT-LICENSE.md IBM-PLEX-OFL-1.1.txt NATIVE-SDK-APACHE-2.0.txt; do
  install -m 644 "$stage/licenses/$notice" "$root/share/licenses/sd300/$notice"
done

cat > "$root/bin/sd300-gui" <<'LAUNCHER'
#!/bin/sh
set -eu
sd300_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
sd300_runtime="$sd300_root/lib/runtime"
export LD_LIBRARY_PATH="$sd300_runtime/lib"
export GSETTINGS_SCHEMA_DIR="$sd300_runtime/share/glib-2.0/schemas"
export GIO_EXTRA_MODULES="$sd300_runtime/lib/gio/modules"
export GDK_PIXBUF_MODULE_FILE="$sd300_runtime/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
export XDG_DATA_DIRS="$sd300_root/share:$sd300_runtime/share:/usr/local/share:/usr/share"
exec "$sd300_root/libexec/sd300-gui" "$@"
LAUNCHER
chmod 755 "$root/bin/sd300-gui"
cat > "$root/share/applications/sd300.desktop" <<'DESKTOP'
[Desktop Entry]
# SD-300 managed desktop entry
Type=Application
Name=SD-300
Comment=Native system diagnostics and live performance monitoring
Exec=@SD300_GUI@
Icon=sd300
Terminal=false
Categories=System;Monitor;
StartupNotify=true
DESKTOP

system_allow='^(linux-vdso.*|ld-linux.*|ld-musl.*|libc\.so(\..*)?|libc\.musl-[^.]+\.so(\..*)?|libm\.so(\..*)?|libdl\.so(\..*)?|libpthread\.so(\..*)?|librt\.so(\..*)?)$'
record_runtime_source() {
  destination=$1
  source=$2
  relative=${destination#"$root"/}
  runtime_sources["$relative"]=$(readlink -f "$source")
}
copy_library() {
  source=$1
  base=$(basename "$source")
  [[ $base =~ $system_allow ]] && return 0
  case "$base" in libGL.so*|libEGL.so*|libGLX.so*|libOpenGL.so*|libdrm.so*) return 0 ;; esac
  destination="$runtime/lib/$base"
  if [[ -e $destination ]]; then
    cmp -s "$source" "$destination" || { echo "runtime library basename collision: $base" >&2; exit 1; }
  else
    cp -L "$source" "$destination"
    chmod 755 "$destination"
  fi
  record_runtime_source "$destination" "$source"
}
copy_closure() {
  object=$1
  dependency_list=$("$lddtree_command" -l "$object") || {
    echo "dependency-tree inspection failed for $object" >&2
    exit 1
  }
  while IFS= read -r dependency; do
    [[ -z $dependency ]] && continue
    [[ -f $dependency ]] || {
      echo "dependency-tree inspection returned an unresolved path for $object: $dependency" >&2
      exit 1
    }
    [[ $dependency == "$object" ]] && continue
    copy_library "$dependency"
  done <<< "$dependency_list"
}
copy_closure "$root/libexec/sd300-gui"
copy_closure "$root/libexec/libsd300_engine.so"

while IFS= read -r bundled_library; do
  if [[ $(basename "$bundled_library") =~ $system_allow ]]; then
    echo "private runtime closure contains forbidden system library: $(basename "$bundled_library")" >&2
    exit 1
  fi
done < <(find "$runtime/lib" -type f)

# SD-300 renders its own canvas, embeds its fonts, and ships only PNG assets.
# GTK/GdkPixbuf provide PNG support in the linked core libraries on both pinned
# baselines. Do not sweep ambient GIO, image-loader, or print-backend modules
# from the runner: doing so makes the archive host-dependent and can pull in
# unrelated networking/CUPS closures. The empty owned directories/cache make
# this intentional module set explicit and are exercised by the blank-host GUI
# launch below.
mkdir -p "$runtime/lib/gio/modules" "$runtime/lib/gdk-pixbuf-2.0/2.10.0"

gtk_schema_names=(
  org.gtk.Settings.ColorChooser.gschema.xml
  org.gtk.Settings.Debug.gschema.xml
  org.gtk.Settings.EmojiChooser.gschema.xml
  org.gtk.Settings.FileChooser.gschema.xml
)
for schema_root in /usr/share/glib-2.0/schemas /usr/local/share/glib-2.0/schemas; do
  [[ -d $schema_root ]] || continue
  for schema_name in "${gtk_schema_names[@]}"; do
    schema="$schema_root/$schema_name"
    [[ -f $schema ]] || continue
    destination="$runtime/share/glib-2.0/schemas/$(basename "$schema")"
    if [[ -e $destination ]]; then
      cmp -s "$schema" "$destination" || {
        echo "schema basename collision: $(basename "$schema")" >&2
        exit 1
      }
    else
      cp -L "$schema" "$destination"
      chmod 644 "$destination"
    fi
    record_runtime_source "$destination" "$schema"
  done
done
glib-compile-schemas "$runtime/share/glib-2.0/schemas"

loader_dir="$runtime/lib/gdk-pixbuf-2.0/2.10.0/loaders"
if [[ -d $loader_dir ]] && command -v gdk-pixbuf-query-loaders >/dev/null; then
  GDK_PIXBUF_MODULEDIR="$loader_dir" gdk-pixbuf-query-loaders > "$runtime/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
  sed -i "s#${runtime//\#/\\#}#@SD300_RUNTIME@#g" "$runtime/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
else
  mkdir -p "$(dirname "$runtime/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache")"
  : > "$runtime/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
fi
if [[ -d $runtime/lib/gio/modules ]] && command -v gio-querymodules >/dev/null; then
  gio-querymodules "$runtime/lib/gio/modules"
fi

patchelf --set-rpath '$ORIGIN/../lib/runtime/lib' "$root/libexec/sd300-gui"
patchelf --set-rpath '$ORIGIN/../lib/runtime/lib' "$root/libexec/libsd300_engine.so"
while IFS= read -r -d '' object; do patchelf --set-rpath '$ORIGIN' "$object"; done < <(find "$runtime/lib" -maxdepth 1 -type f -name '*.so*' -print0)

cat > "$root/THIRD_PARTY_NOTICES.txt" <<'NOTICES'
SD-300 Linux private runtime

This package contains a target-specific GTK4 user-space runtime closure. The
corresponding package-to-file SPDX inventory is included as
runtime-components.spdx.json. Package-specific copyright notices and the
pinned distribution's common license texts are bundled under
share/licenses/runtime. System libc, ELF loaders, kernel interfaces, and host
GPU drivers are excluded.
NOTICES
origins="$work/runtime-origins.tsv"
for destination in "${!runtime_sources[@]}"; do
  printf '%s\t%s\n' "$destination" "${runtime_sources[$destination]}"
done | LC_ALL=C sort > "$origins"
node "$script_root/generate-linux-runtime-sbom.mjs" \
  "$root" "$target" "$version" "$origins"
node "$script_root/write-gui-install-manifest.mjs" "$root" "$target" "bin/sd300-gui" "libexec/libsd300_engine.so" "$version"

mkdir -p "$output_dir"
archive="$output_dir/sd300-gui-$target.tar.xz"
tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner -C "$work" -cJf "$archive" sd300
(cd "$output_dir" && sha256sum "$(basename "$archive")" > "$(basename "$archive").sha256")
echo "$archive"
