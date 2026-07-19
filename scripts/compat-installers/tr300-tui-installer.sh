#!/bin/sh
# Compatibility router for immutable SD-300 1.4.x Unix updaters.
# Fresh installs use sd300-cli-installer.sh.
set -eu

sd300_tag='@SD300_TAG@'
sd300_version='@SD300_VERSION@'
sd300_release_base="https://github.com/QubeTX/qube-system-diagnostics/releases/download/${sd300_tag}"
sd300_temp=''

sd300_cleanup() {
    [ -z "$sd300_temp" ] || rm -rf "$sd300_temp"
}
trap sd300_cleanup EXIT HUP INT TERM

sd300_download() {
    url=$1
    output=$2
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -fLsS "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget --https-only -q "$url" -O "$output"
    else
        echo 'SD-300 requires curl or wget for a verified update.' >&2
        exit 1
    fi
}

sd300_stage() {
    name=$1
    sd300_download "${sd300_release_base}/${name}" "${sd300_temp}/${name}"
    sd300_download "${sd300_release_base}/${name}.sha256" "${sd300_temp}/${name}.sha256"
    expected=$(awk 'NR == 1 { print $1 }' "${sd300_temp}/${name}.sha256")
    case "$expected" in
        *[!0-9a-fA-F]*|'') echo "Invalid checksum sidecar for ${name}." >&2; exit 1 ;;
    esac
    [ "${#expected}" -eq 64 ] || { echo "Invalid checksum length for ${name}." >&2; exit 1; }
    if command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "${sd300_temp}/${name}" | awk '{print $1}')
    else
        actual=$(sha256sum "${sd300_temp}/${name}" | awk '{print $1}')
    fi
    [ "$actual" = "$expected" ] || { echo "Checksum mismatch for ${name}." >&2; exit 1; }
}

if [ "${SD300_COMPAT_INSTALLER_TEST_ONLY:-}" = 1 ]; then
    # The exit is the executed-script fallback when return is unavailable.
    # shellcheck disable=SC2317
    return 0 2>/dev/null || exit 0
fi

sd300_temp=$(mktemp -d "${TMPDIR:-/tmp}/sd300-compat.XXXXXXXX")

if [ "$(uname -s)" = Darwin ] && command -v pkgutil >/dev/null 2>&1 && \
    pkgutil --pkg-info com.qubetx.sd300.pkg >/dev/null 2>&1; then
    pkgutil --files com.qubetx.sd300.pkg | sed 's#^\./##; s#^/##' | grep -Fxq 'usr/local/bin/sd300' || {
        echo 'SD-300 PKG receipt does not own the expected payload; preserving it.' >&2
        exit 1
    }
    pkgutil --file-info /usr/local/bin/sd300 | grep -Eq '^(pkgid|package-id):[[:space:]]*com\.qubetx\.sd300\.pkg$' || {
        echo 'SD-300 PKG payload ownership is ambiguous; preserving it.' >&2
        exit 1
    }
    sd300_stage 'sd300-macos-universal.pkg'
    echo 'Updating SD-300 through its proven macOS PKG channel...'
    sudo installer -pkg "${sd300_temp}/sd300-macos-universal.pkg" -target /
    pkgutil --pkg-info com.qubetx.sd300.pkg | grep -Eq "^version:[[:space:]]*${sd300_version}$"
    [ "$(/usr/local/bin/sd300 --version)" = "sd300 ${sd300_version}" ]
    exit 0
fi

sd300_stage 'sd300-cli-installer.sh'
exec sh "${sd300_temp}/sd300-cli-installer.sh" "$@"
