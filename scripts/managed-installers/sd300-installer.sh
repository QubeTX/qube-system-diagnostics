#!/bin/sh
# SD-300 managed shell installer.
#
# The release workflow renders the immutable tag/version placeholders, keeps the
# cargo-dist generated installer as sd300-dist-installer.sh, and publishes this
# stable-name wrapper as sd300-cli-installer.sh. On macOS a deliberately launched
# fresh wrapper safely supersedes an exact, receipt-owned SD-300 PKG install.

set -u

sd300_tag='@SD300_TAG@'
sd300_version='@SD300_VERSION@'
sd300_release_base="https://github.com/QubeTX/qube-system-diagnostics/releases/download/${sd300_tag}"
sd300_recovery_url='https://github.com/QubeTX/qube-system-diagnostics/releases/latest'
sd300_temp=''
sd300_transaction_started=0
sd300_committed=0
sd300_receipt_existed=0
sd300_intended_binary_existed=0
sd300_prior_binary_existed=0
sd300_prior_binary=''
sd300_pkg_present=0
sd300_pkg_payload_removed=0

sd300_cleanup() {
    if [ "$sd300_transaction_started" -eq 1 ] && [ "$sd300_committed" -eq 0 ]; then
        sd300_restore_managed_state ||
            printf '%s\n' 'SD-300 warning: restoring the prior managed/Cargo state also failed' >&2
        if [ "$sd300_pkg_payload_removed" -eq 1 ] && [ -f "$sd300_temp/prior-pkg-sd300" ]; then
            sudo /usr/bin/ditto "$sd300_temp/prior-pkg-sd300" /usr/local/bin/sd300 >/dev/null 2>&1 ||
                printf '%s\n' 'SD-300 warning: restoring the prior PKG payload also failed' >&2
        fi
        sd300_committed=1
    fi
    if [ -n "$sd300_temp" ]; then
        rm -rf "$sd300_temp" >/dev/null 2>&1 || true
    fi
}
trap sd300_cleanup EXIT HUP INT TERM

sd300_fail() {
    printf '%s\n' "SD-300 managed install failed safely: $*" >&2
    printf '%s\n' "Download a fresh installer: ${sd300_recovery_url}" >&2
    exit 1
}

sd300_download() {
    url=$1
    output=$2
    if command -v curl >/dev/null 2>&1; then
        auth_token=${SD300_GITHUB_TOKEN:-${GITHUB_TOKEN:-${GH_TOKEN:-}}}
        if [ -n "$auth_token" ]; then
            curl --proto '=https' --tlsv1.2 -fLsS \
                -H "Authorization: Bearer ${auth_token}" "$url" -o "$output"
        else
            curl --proto '=https' --tlsv1.2 -fLsS "$url" -o "$output"
        fi
    elif command -v wget >/dev/null 2>&1; then
        auth_token=${SD300_GITHUB_TOKEN:-${GITHUB_TOKEN:-${GH_TOKEN:-}}}
        if [ -n "$auth_token" ]; then
            wget -q --header="Authorization: Bearer ${auth_token}" -O "$output" "$url"
        else
            wget -q -O "$output" "$url"
        fi
    else
        sd300_fail 'curl or wget is required'
    fi
}

sd300_stage_release_asset() {
    name=$1
    output=$2
    if [ "${GITHUB_ACTIONS:-}" = true ] && [ -n "${SD300_CI_RELEASE_ASSET_DIR:-}" ]; then
        case "$SD300_CI_RELEASE_ASSET_DIR" in
            /*) ;;
            *) sd300_fail 'GitHub Actions candidate asset directory must be absolute' ;;
        esac
        [ -f "${SD300_CI_RELEASE_ASSET_DIR%/}/${name}" ] ||
            sd300_fail "GitHub Actions candidate asset is missing: ${name}"
        cp "${SD300_CI_RELEASE_ASSET_DIR%/}/${name}" "$output" ||
            sd300_fail "could not stage GitHub Actions candidate asset: ${name}"
    else
        sd300_download "${sd300_release_base}/${name}" "$output" ||
            sd300_fail "could not download immutable release asset: ${name}"
    fi
}

sd300_verify_sha256() {
    asset=$1
    sidecar=$2
    expected=$(awk '{ for (i = 1; i <= NF; i++) if ($i ~ /^[0-9a-fA-F]{64}$/) { print tolower($i); exit } }' "$sidecar")
    [ -n "$expected" ] || sd300_fail 'managed installer SHA-256 sidecar is invalid'
    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$asset" | awk '{print tolower($1)}')
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$asset" | awk '{print tolower($1)}')
    else
        sd300_fail 'sha256sum or shasum is required to verify the managed installer'
    fi
    [ "$actual" = "$expected" ] || sd300_fail 'managed installer SHA-256 verification failed'
}

sd300_install_prefix() {
    if [ -n "${SD300_INSTALL_DIR:-}" ]; then
        printf '%s\n' "$SD300_INSTALL_DIR"
    elif [ -n "${CARGO_DIST_FORCE_INSTALL_DIR:-}" ]; then
        printf '%s\n' "$CARGO_DIST_FORCE_INSTALL_DIR"
    elif [ -n "${CARGO_HOME:-}" ]; then
        printf '%s\n' "$CARGO_HOME"
    elif [ -n "${HOME:-}" ]; then
        printf '%s\n' "$HOME/.cargo"
    else
        sd300_fail 'HOME or CARGO_HOME is required to verify the managed install'
    fi
}

sd300_receipt_path() {
    if [ -n "${XDG_CONFIG_HOME:-}" ]; then
        printf '%s\n' "${XDG_CONFIG_HOME%/}/sd300/sd300-receipt.json"
    elif [ -n "${HOME:-}" ]; then
        printf '%s\n' "${HOME%/}/.config/sd300/sd300-receipt.json"
    else
        sd300_fail 'HOME or XDG_CONFIG_HOME is required to verify the managed receipt'
    fi
}

sd300_receipt_prefix() {
    receipt=$1
    prefix=$(sed -n 's/.*"install_prefix"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$receipt")
    [ -n "$prefix" ] || return 1
    [ "$(printf '%s\n' "$prefix" | wc -l | tr -d ' ')" = 1 ] || return 1
    case "$prefix" in
        /*) ;;
        *) return 1 ;;
    esac
    case "$prefix" in
        *\\*) return 1 ;;
    esac
    printf '%s\n' "$prefix"
}

sd300_receipt_version() {
    receipt=$1
    version=$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$receipt" | tail -n 1)
    [ -n "$version" ] || return 1
    printf '%s\n' "$version"
}

sd300_receipt_is_exact_app() {
    receipt=$1
    grep -Eq '"source"[[:space:]]*:[[:space:]]*"cargo-dist"' "$receipt" || return 1
    grep -Eq '"app_name"[[:space:]]*:[[:space:]]*"sd300"' "$receipt" || return 1
    sd300_receipt_prefix "$receipt" >/dev/null 2>&1 || return 1
}

sd300_save_managed_state() {
    sd300_intended_prefix=$(sd300_install_prefix) || sd300_fail 'could not resolve the managed install prefix'
    case "$sd300_intended_prefix" in
        /*) ;;
        *) sd300_fail 'the managed install prefix must be an absolute path' ;;
    esac
    sd300_intended_binary="${sd300_intended_prefix%/}/bin/sd300"
    sd300_receipt=$(sd300_receipt_path) || sd300_fail 'could not resolve the managed receipt path'

    if [ -f "$sd300_receipt" ]; then
        sd300_receipt_is_exact_app "$sd300_receipt" ||
            sd300_fail 'the existing SD-300 managed receipt is ambiguous; preserving it'
        sd300_receipt_existed=1
        cp -p "$sd300_receipt" "$sd300_temp/prior-receipt.json" ||
            sd300_fail 'could not back up the existing managed receipt'
        sd300_prior_prefix=$(sd300_receipt_prefix "$sd300_receipt") ||
            sd300_fail 'could not read the existing managed install prefix'
        sd300_prior_binary="${sd300_prior_prefix%/}/bin/sd300"
    fi

    if [ -f "$sd300_intended_binary" ]; then
        sd300_intended_binary_existed=1
        cp -p "$sd300_intended_binary" "$sd300_temp/prior-intended-sd300" ||
            sd300_fail 'could not back up the existing managed/Cargo binary'
    fi
    if [ -n "$sd300_prior_binary" ] && [ "$sd300_prior_binary" != "$sd300_intended_binary" ] &&
        [ -f "$sd300_prior_binary" ]; then
        sd300_prior_binary_existed=1
        cp -p "$sd300_prior_binary" "$sd300_temp/prior-receipt-sd300" ||
            sd300_fail 'could not back up the receipt-owned managed binary'
    fi
}

sd300_restore_one_binary() {
    path=$1
    existed=$2
    backup=$3
    if [ "$existed" -eq 1 ]; then
        mkdir -p "$(dirname "$path")" && cp -p "$backup" "$path"
    else
        rm -f "$path"
    fi
}

sd300_restore_managed_state() {
    sd300_restore_one_binary "$sd300_intended_binary" "$sd300_intended_binary_existed" \
        "$sd300_temp/prior-intended-sd300" || return 1
    if [ -n "$sd300_prior_binary" ] && [ "$sd300_prior_binary" != "$sd300_intended_binary" ]; then
        sd300_restore_one_binary "$sd300_prior_binary" "$sd300_prior_binary_existed" \
            "$sd300_temp/prior-receipt-sd300" || return 1
    fi
    if [ "$sd300_receipt_existed" -eq 1 ]; then
        mkdir -p "$(dirname "$sd300_receipt")" &&
            cp -p "$sd300_temp/prior-receipt.json" "$sd300_receipt"
    else
        rm -f "$sd300_receipt"
    fi
}

sd300_verify_receipt() {
    [ -f "$sd300_receipt" ] || sd300_fail "managed installer receipt is missing: $sd300_receipt"
    sd300_receipt_is_exact_app "$sd300_receipt" ||
        sd300_fail 'managed installer receipt does not identify SD-300 cargo-dist ownership'
    receipt_prefix=$(sd300_receipt_prefix "$sd300_receipt") ||
        sd300_fail 'managed installer receipt has no exact install prefix'
    [ "$receipt_prefix" = "$sd300_intended_prefix" ] ||
        sd300_fail 'managed installer receipt does not identify the requested install prefix'
    receipt_version=$(sd300_receipt_version "$sd300_receipt") ||
        sd300_fail 'managed installer receipt has no exact version'
    [ "$receipt_version" = "$sd300_version" ] ||
        sd300_fail "managed installer receipt does not identify ${sd300_version}"
}

sd300_verify_binary() {
    binary=$sd300_intended_binary
    [ -x "$binary" ] || sd300_fail "managed SD-300 binary is missing: $binary"
    reported=$($binary --version 2>/dev/null) || sd300_fail 'managed SD-300 binary did not run'
    [ "$reported" = "sd300 ${sd300_version}" ] \
        || sd300_fail "managed SD-300 binary did not report ${sd300_version}"
    printf '%s\n' "$binary"
}

sd300_pkg_is_exact_product() {
    package_info=$(pkgutil --pkg-info com.qubetx.sd300.pkg 2>/dev/null) || return 1
    payload_files=$(pkgutil --files com.qubetx.sd300.pkg 2>/dev/null) || return 1
    file_info=$(pkgutil --file-info /usr/local/bin/sd300 2>/dev/null) || return 1
    signature=$(codesign -d --verbose=4 /usr/local/bin/sd300 2>&1) || return 1
    codesign --verify --strict /usr/local/bin/sd300 >/dev/null 2>&1 || return 1

    printf '%s\n' "$package_info" | grep -Eq '^package-id:[[:space:]]*com\.qubetx\.sd300\.pkg$' || return 1
    printf '%s\n' "$package_info" | grep -Eq '^volume:[[:space:]]*/$' || return 1
    printf '%s\n' "$payload_files" | sed 's#^\./##; s#^/##' | grep -Fxq 'usr/local/bin/sd300' || return 1
    printf '%s\n' "$file_info" | grep -Eq '^(pkgid|package-id):[[:space:]]*com\.qubetx\.sd300\.pkg$' || return 1
    printf '%s\n' "$file_info" | grep -Eq '^path:[[:space:]]*/usr/local/bin/sd300$' || return 1
    printf '%s\n' "$signature" | grep -Fxq 'Identifier=com.qubetx.sd300' || return 1
    printf '%s\n' "$signature" | grep -Fxq 'TeamIdentifier=M9D5379H93' || return 1
    printf '%s\n' "$signature" | grep -Eq '^Authority=Developer ID Application:' || return 1
}

sd300_prepare_macos_pkg() {
    [ "$(uname -s)" = Darwin ] || return 0
    command -v pkgutil >/dev/null 2>&1 || sd300_fail 'pkgutil is unavailable on macOS'
    if ! pkgutil --pkg-info com.qubetx.sd300.pkg >/dev/null 2>&1; then
        return 0
    fi
    [ -e /usr/local/bin/sd300 ] || sd300_fail 'the SD-300 PKG receipt exists but its payload is missing'
    sd300_pkg_is_exact_product || sd300_fail 'the PKG receipt/payload/signature evidence conflicts; preserving it'
    /usr/bin/ditto /usr/local/bin/sd300 "$sd300_temp/prior-pkg-sd300" ||
        sd300_fail 'could not back up the receipt-owned PKG payload'
    sd300_pkg_present=1
}

sd300_take_over_macos_pkg() {
    [ "$sd300_pkg_present" -eq 1 ] || return 0
    sd300_pkg_is_exact_product || sd300_fail 'the PKG receipt/payload/signature evidence conflicts; preserving it'

    printf '%s\n' 'Switching SD-300 ownership from macos-pkg to shell-installer...'
    sudo -v || sd300_fail 'administrator authorization was cancelled; the existing PKG was preserved'
    sudo rm -f /usr/local/bin/sd300 || sd300_fail 'could not remove the receipt-owned PKG payload'
    sd300_pkg_payload_removed=1
    sudo pkgutil --forget com.qubetx.sd300.pkg >/dev/null \
        || sd300_fail 'could not forget the SD-300 PKG receipt'
    if [ -e /usr/local/bin/sd300 ] || pkgutil --pkg-info com.qubetx.sd300.pkg >/dev/null 2>&1; then
        sd300_fail 'PKG takeover did not converge; the managed shell install remains available'
    fi
}

if [ "${SD300_MANAGED_INSTALLER_TEST_ONLY:-}" = 1 ]; then
    # The exit is the executed-script fallback when return is unavailable.
    # shellcheck disable=SC2317
    return 0 2>/dev/null || exit 0
fi

sd300_temp=$(mktemp -d "${TMPDIR:-/tmp}/sd300-managed-install.XXXXXXXX") \
    || sd300_fail 'could not create a private staging directory'
sd300_prepare_macos_pkg
sd300_save_managed_state
dist_installer="$sd300_temp/sd300-dist-installer.sh"
dist_sidecar="${dist_installer}.sha256"
sd300_stage_release_asset 'sd300-dist-installer.sh' "$dist_installer"
sd300_stage_release_asset 'sd300-dist-installer.sh.sha256' "$dist_sidecar"
sd300_verify_sha256 "$dist_installer" "$dist_sidecar"
chmod 700 "$dist_installer" || sd300_fail 'could not protect the managed installer'

sd300_transaction_started=1
sh "$dist_installer" "$@" || sd300_fail 'cargo-dist installation did not complete'
sd300_verify_receipt
managed_binary=$(sd300_verify_binary) || sd300_fail 'managed SD-300 verification did not complete'
sd300_take_over_macos_pkg
if [ -n "$sd300_prior_binary" ] && [ "$sd300_prior_binary" != "$managed_binary" ]; then
    rm -f "$sd300_prior_binary" || sd300_fail 'could not remove the prior managed install path'
fi
sd300_verify_receipt
sd300_verify_binary >/dev/null || sd300_fail 'final managed SD-300 verification did not complete'
sd300_committed=1
printf '%s\n' "SD-300 ${sd300_version} is installed through the managed shell channel: ${managed_binary}"
