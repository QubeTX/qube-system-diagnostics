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
sd300_gui_root=''
sd300_gui_root_existed=0
sd300_gui_desktop=''
sd300_gui_desktop_existed=0
sd300_gui_target=''
sd300_gui_archive_name=''
sd300_gui_stage=''
sd300_gui_owner=''
sd300_gui_install_started=0

sd300_cleanup() {
    if [ "$sd300_transaction_started" -eq 1 ] && [ "$sd300_committed" -eq 0 ]; then
        sd300_restore_managed_state ||
            printf '%s\n' 'SD-300 warning: restoring the prior managed/Cargo state also failed' >&2
        if [ "$sd300_pkg_payload_removed" -eq 1 ] && [ -f "$sd300_temp/prior-pkg-sd300" ]; then
            sudo /usr/bin/ditto "$sd300_temp/prior-pkg-sd300" /usr/local/bin/sd300 >/dev/null 2>&1 ||
                printf '%s\n' 'SD-300 warning: restoring the prior PKG payload also failed' >&2
            if [ -d "$sd300_temp/prior-pkg-app" ]; then
                sudo /usr/bin/ditto "$sd300_temp/prior-pkg-app" /Applications/SD-300.app >/dev/null 2>&1 ||
                    printf '%s\n' 'SD-300 warning: restoring the prior PKG application also failed' >&2
            fi
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

sd300_file_sha256() {
    file=$1
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file" | awk '{print tolower($1)}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file" | awk '{print tolower($1)}'
    else
        sd300_fail 'sha256sum or shasum is required to verify the GUI manifest'
    fi
}

# The managed shell installer deliberately has no jq/Python/Node dependency.
# The manifest is emitted by write-gui-install-manifest.mjs in one reviewed,
# stable shape, so extract only its exact scalar and file-record lines with
# POSIX tools and fail closed on any shape or inventory drift.
sd300_verify_gui_manifest() {
    payload_root=$1
    manifest="$payload_root/install-manifest.json"
    expected_entrypoint=$2
    expected_engine=$3
    [ -f "$manifest" ] || sd300_fail 'GUI archive has no install-manifest.json'
    [ "$(grep -Ec '^[[:space:]]{2}"schema": 1,$' "$manifest")" -eq 1 ] ||
        sd300_fail 'GUI archive manifest schema is incompatible'
    grep -Fqx '  "product": "SD-300",' "$manifest" ||
        sd300_fail 'GUI archive manifest product is incompatible'
    grep -Fqx "  \"version\": \"${sd300_version}\"," "$manifest" ||
        sd300_fail 'GUI archive manifest version is incompatible'
    grep -Fqx "  \"target\": \"${sd300_gui_target}\"," "$manifest" ||
        sd300_fail 'GUI archive manifest target is incompatible'
    grep -Fqx "  \"entrypoint\": \"${expected_entrypoint}\"," "$manifest" ||
        sd300_fail 'GUI archive manifest entrypoint is incompatible'
    grep -Fqx "  \"engine\": \"${expected_engine}\"," "$manifest" ||
        sd300_fail 'GUI archive manifest engine is incompatible'

    manifest_paths="$sd300_temp/gui-manifest-paths"
    manifest_hashes="$sd300_temp/gui-manifest-hashes"
    manifest_inventory="$sd300_temp/gui-manifest-inventory"
    actual_inventory="$sd300_temp/gui-actual-inventory"
    sed -n 's/^[[:space:]]*"path": "\([^"]*\)",[[:space:]]*$/\1/p' "$manifest" > "$manifest_paths"
    sed -n 's/^[[:space:]]*"sha256": "\([0-9a-f][0-9a-f]*\)"[[:space:]]*$/\1/p' "$manifest" > "$manifest_hashes"
    [ -s "$manifest_paths" ] || sd300_fail 'GUI archive manifest has no file inventory'
    [ "$(wc -l < "$manifest_paths" | tr -d ' ')" = "$(wc -l < "$manifest_hashes" | tr -d ' ')" ] ||
        sd300_fail 'GUI archive manifest file records are incomplete'
    if grep -Ev '^[A-Za-z0-9._/@+-]+$' "$manifest_paths" >/dev/null; then
        sd300_fail 'GUI archive manifest contains a non-portable path'
    fi
    if grep -E '(^/|(^|/)\.\.(/|$)|(^|/)[.](/|$))' "$manifest_paths" >/dev/null; then
        sd300_fail 'GUI archive manifest contains an unsafe path'
    fi
    if [ "$(LC_ALL=C sort "$manifest_paths" | uniq | wc -l | tr -d ' ')" != "$(wc -l < "$manifest_paths" | tr -d ' ')" ]; then
        sd300_fail 'GUI archive manifest repeats a file path'
    fi
    paste "$manifest_hashes" "$manifest_paths" > "$manifest_inventory"
    while IFS="$(printf '\t')" read -r expected relative; do
        [ "${#expected}" -eq 64 ] || sd300_fail "GUI archive manifest has an invalid SHA-256 for ${relative}"
        path="$payload_root/$relative"
        [ -f "$path" ] && [ ! -L "$path" ] ||
            sd300_fail "GUI archive is missing a declared regular file: ${relative}"
        actual=$(sd300_file_sha256 "$path")
        [ "$actual" = "$expected" ] ||
            sd300_fail "GUI archive file failed manifest verification: ${relative}"
    done < "$manifest_inventory"

    if find "$payload_root" -type l -print -quit | grep -q .; then
        sd300_fail 'GUI archive contains a symbolic link'
    fi
    (cd "$payload_root" && find . -type f -print | sed 's#^\./##' |
        grep -Fvx 'install-manifest.json' | LC_ALL=C sort) > "$actual_inventory"
    LC_ALL=C sort "$manifest_paths" > "${manifest_paths}.sorted"
    cmp -s "${manifest_paths}.sorted" "$actual_inventory" ||
        sd300_fail 'GUI archive manifest does not exactly cover the extracted payload'
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
    fi || return 1
    sd300_restore_gui_state
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

sd300_resolve_gui_target() {
    os=$(uname -s)
    arch=$(uname -m)
    case "$os:$arch" in
        Darwin:x86_64|Darwin:arm64)
            sd300_gui_target='macos-universal'
            sd300_gui_archive_name='sd300-gui-macos-universal.zip'
            [ -n "${HOME:-}" ] || sd300_fail 'HOME is required to install the macOS application'
            sd300_gui_root="${HOME%/}/Applications/SD-300.app"
            sd300_gui_owner="${HOME%/}/Library/Application Support/SD-300/managed-install-owner.json"
            ;;
        Linux:x86_64|Linux:amd64)
            libc=gnu
            if ldd --version 2>&1 | grep -qi musl; then libc=musl; fi
            sd300_gui_target="linux-${libc}-x86_64"
            sd300_gui_archive_name="sd300-gui-${sd300_gui_target}.tar.xz"
            data_home=${XDG_DATA_HOME:-${HOME:-}/.local/share}
            [ -n "$data_home" ] || sd300_fail 'HOME or XDG_DATA_HOME is required to install the Linux application'
            sd300_gui_root="${data_home%/}/sd300"
            sd300_gui_desktop="${data_home%/}/applications/sd300.desktop"
            sd300_gui_owner="$sd300_gui_root/.sd300-managed-owner.json"
            ;;
        Linux:aarch64|Linux:arm64)
            if ldd --version 2>&1 | grep -qi musl; then
                sd300_fail 'Linux musl ARM64 is not an existing SD-300 release target'
            fi
            sd300_gui_target='linux-gnu-arm64'
            sd300_gui_archive_name='sd300-gui-linux-gnu-arm64.tar.xz'
            data_home=${XDG_DATA_HOME:-${HOME:-}/.local/share}
            [ -n "$data_home" ] || sd300_fail 'HOME or XDG_DATA_HOME is required to install the Linux application'
            sd300_gui_root="${data_home%/}/sd300"
            sd300_gui_desktop="${data_home%/}/applications/sd300.desktop"
            sd300_gui_owner="$sd300_gui_root/.sd300-managed-owner.json"
            ;;
        *) sd300_fail "the native GUI has no qualified package for ${os} ${arch}" ;;
    esac
    case "$sd300_gui_root" in /*) ;; *) sd300_fail 'the GUI install root must be absolute' ;; esac
}

sd300_gui_marker() {
    [ -n "$sd300_gui_owner" ] || return 1
    printf '%s\n' "$sd300_gui_owner"
}

sd300_gui_root_is_managed() {
    marker=$(sd300_gui_marker) || return 1
    [ -f "$marker" ] || return 1
    grep -Eq '"schema"[[:space:]]*:[[:space:]]*1' "$marker" || return 1
    grep -Eq '"product"[[:space:]]*:[[:space:]]*"SD-300"' "$marker" || return 1
    grep -Eq '"owner"[[:space:]]*:[[:space:]]*"shell-installer"' "$marker" || return 1
}

sd300_save_gui_state() {
    if [ -f "$sd300_gui_owner" ] && [ ! -e "$sd300_gui_root" ]; then
        sd300_fail "GUI ownership marker exists without its managed payload: $sd300_gui_owner"
    fi
    if [ -e "$sd300_gui_root" ]; then
        [ -d "$sd300_gui_root" ] || sd300_fail "GUI destination is not a directory: $sd300_gui_root"
        sd300_gui_root_is_managed || sd300_fail "GUI destination exists without managed-shell ownership: $sd300_gui_root"
        sd300_gui_root_existed=1
        if [ "$(uname -s)" = Darwin ]; then
            /usr/bin/ditto "$sd300_gui_root" "$sd300_temp/prior-gui-root" || sd300_fail 'could not back up the managed macOS app'
        else
            cp -a "$sd300_gui_root" "$sd300_temp/prior-gui-root" || sd300_fail 'could not back up the managed Linux app'
        fi
        cp -p "$sd300_gui_owner" "$sd300_temp/prior-gui-owner" || sd300_fail 'could not back up the GUI ownership marker'
    fi
    if [ -n "$sd300_gui_desktop" ] && [ -f "$sd300_gui_desktop" ]; then
        grep -Fq '# SD-300 managed desktop entry' "$sd300_gui_desktop" ||
            sd300_fail "desktop entry exists without SD-300 ownership: $sd300_gui_desktop"
        sd300_gui_desktop_existed=1
        cp -p "$sd300_gui_desktop" "$sd300_temp/prior-gui-desktop"
    fi
}

sd300_gui_is_running() {
    current_uid=$(id -u)
    if command -v pgrep >/dev/null 2>&1; then
        pgrep -u "$current_uid" -x sd300-gui >/dev/null 2>&1
        return
    fi
    ps -ax -o uid= -o comm= 2>/dev/null | awk -v uid="$current_uid" '
        $1 == uid {
            name = $2
            sub(/^.*\//, "", name)
            if (name == "sd300-gui") found = 1
        }
        END { exit(found ? 0 : 1) }
    '
}

sd300_try_stop_gui_with() {
    lifecycle_cli=$1
    [ -x "$lifecycle_cli" ] || return 1
    "$lifecycle_cli" stop-gui --quiet
}

sd300_stop_owned_gui() {
    owned_gui_present=0
    if [ "$sd300_gui_root_existed" -eq 1 ]; then
        owned_gui_present=1
    fi
    if [ "$sd300_pkg_present" -eq 1 ] && [ -d /Applications/SD-300.app ]; then
        owned_gui_present=1
    fi
    [ "$owned_gui_present" -eq 1 ] || return 0
    sd300_gui_is_running || return 0

    if [ -n "$sd300_prior_binary" ] && sd300_try_stop_gui_with "$sd300_prior_binary"; then
        return 0
    fi
    if [ -n "${sd300_intended_binary:-}" ] && [ "$sd300_intended_binary" != "$sd300_prior_binary" ] &&
        sd300_try_stop_gui_with "$sd300_intended_binary"; then
        return 0
    fi
    if [ "$sd300_pkg_present" -eq 1 ] && [ /usr/local/bin/sd300 != "$sd300_prior_binary" ] &&
        [ /usr/local/bin/sd300 != "${sd300_intended_binary:-}" ] &&
        sd300_try_stop_gui_with /usr/local/bin/sd300; then
        return 0
    fi
    sd300_fail 'an owned SD-300 GUI is running but no proven CLI could stop it through the authenticated lifecycle endpoint'
}

sd300_restore_gui_state() {
    if [ -e "$sd300_gui_root" ]; then
        if [ "$sd300_gui_install_started" -ne 1 ]; then
            sd300_gui_root_is_managed || return 1
        fi
        rm -rf "$sd300_gui_root" || return 1
    fi
    if [ "$sd300_gui_root_existed" -eq 1 ]; then
        mkdir -p "$(dirname "$sd300_gui_root")" || return 1
        if [ "$(uname -s)" = Darwin ]; then
            /usr/bin/ditto "$sd300_temp/prior-gui-root" "$sd300_gui_root" || return 1
        else
            cp -a "$sd300_temp/prior-gui-root" "$sd300_gui_root" || return 1
        fi
        mkdir -p "$(dirname "$sd300_gui_owner")" &&
            cp -p "$sd300_temp/prior-gui-owner" "$sd300_gui_owner" || return 1
    elif [ -f "$sd300_gui_owner" ]; then
        sd300_gui_root_is_managed || return 1
        rm -f "$sd300_gui_owner" || return 1
    fi
    if [ -n "$sd300_gui_desktop" ]; then
        if [ "$sd300_gui_desktop_existed" -eq 1 ]; then
            mkdir -p "$(dirname "$sd300_gui_desktop")" &&
                cp -p "$sd300_temp/prior-gui-desktop" "$sd300_gui_desktop" || return 1
        elif [ -f "$sd300_gui_desktop" ] && grep -Fq '# SD-300 managed desktop entry' "$sd300_gui_desktop"; then
            rm -f "$sd300_gui_desktop" || return 1
        fi
    fi
}

sd300_stage_gui_payload() {
    gui_archive="$sd300_temp/$sd300_gui_archive_name"
    gui_sidecar="${gui_archive}.sha256"
    sd300_stage_release_asset "$sd300_gui_archive_name" "$gui_archive"
    sd300_stage_release_asset "${sd300_gui_archive_name}.sha256" "$gui_sidecar"
    sd300_verify_sha256 "$gui_archive" "$gui_sidecar"
    sd300_gui_stage="$sd300_temp/gui-payload"
    mkdir -p "$sd300_gui_stage"
    case "$sd300_gui_target" in
        macos-*)
            command -v unzip >/dev/null 2>&1 || sd300_fail 'unzip is required to install the macOS app'
            unzip -Z1 "$gui_archive" | grep -Eq '(^/|(^|/)\.\.(/|$))' && sd300_fail 'GUI archive contains an unsafe path'
            unzip -q "$gui_archive" -d "$sd300_gui_stage" || sd300_fail 'could not extract the macOS GUI archive'
            [ -d "$sd300_gui_stage/SD-300.app" ] || sd300_fail 'macOS GUI archive has no SD-300.app'
            sd300_verify_gui_manifest "$sd300_gui_stage" \
                'SD-300.app/Contents/MacOS/sd300-gui' \
                'SD-300.app/Contents/MacOS/libsd300_engine.dylib'
            codesign --verify --deep --strict "$sd300_gui_stage/SD-300.app" >/dev/null 2>&1 || sd300_fail 'macOS GUI code signature is invalid'
            gui_binary="$sd300_gui_stage/SD-300.app/Contents/MacOS/sd300-gui"
            ;;
        linux-*)
            members=$(tar -tJf "$gui_archive") || sd300_fail 'could not inspect the Linux GUI archive'
            printf '%s\n' "$members" | grep -Eq '(^/|(^|/)\.\.(/|$))' && sd300_fail 'GUI archive contains an unsafe path'
            printf '%s\n' "$members" | grep -Eqv '^sd300(/|$)' && sd300_fail 'GUI archive contains files outside its owned root'
            tar -xJf "$gui_archive" -C "$sd300_gui_stage" || sd300_fail 'could not extract the Linux GUI archive'
            [ -d "$sd300_gui_stage/sd300" ] || sd300_fail 'Linux GUI archive has no sd300 root'
            sd300_verify_gui_manifest "$sd300_gui_stage/sd300" \
                'bin/sd300-gui' 'libexec/libsd300_engine.so'
            gui_binary="$sd300_gui_stage/sd300/bin/sd300-gui"
            ;;
    esac
    [ -x "$gui_binary" ] || sd300_fail 'GUI archive entrypoint is missing or not executable'
    gui_result=$($gui_binary --self-test --json 2>/dev/null) || sd300_fail 'staged GUI self-test failed'
    printf '%s\n' "$gui_result" | grep -Eq '"success"[[:space:]]*:[[:space:]]*true' || sd300_fail 'staged GUI did not report success'
    printf '%s\n' "$gui_result" | grep -Eq "\"product_version\"[[:space:]]*:[[:space:]]*\"${sd300_version}\"" || sd300_fail 'staged GUI version is incompatible'
    printf '%s\n' "$gui_result" | grep -Eq '"abi_version"[[:space:]]*:[[:space:]]*1' || sd300_fail 'staged GUI ABI is incompatible'
}

sd300_install_gui_payload() {
    sd300_gui_install_started=1
    if [ -e "$sd300_gui_root" ]; then
        sd300_gui_root_is_managed || sd300_fail "refusing to replace an unowned GUI root: $sd300_gui_root"
        rm -rf "$sd300_gui_root" || sd300_fail 'could not remove the prior managed GUI payload'
    fi
    mkdir -p "$(dirname "$sd300_gui_root")" || sd300_fail 'could not create the GUI parent directory'
    case "$sd300_gui_target" in
        macos-*)
            /usr/bin/ditto "$sd300_gui_stage/SD-300.app" "$sd300_gui_root" || sd300_fail 'could not install SD-300.app'
            marker="$sd300_gui_owner"
            gui_binary="$sd300_gui_root/Contents/MacOS/sd300-gui"
            ;;
        linux-*)
            mv "$sd300_gui_stage/sd300" "$sd300_gui_root" || sd300_fail 'could not install the Linux GUI payload'
            marker="$sd300_gui_owner"
            gui_binary="$sd300_gui_root/bin/sd300-gui"
            runtime="$sd300_gui_root/lib/runtime"
            cache="$runtime/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
            if [ -f "$cache" ]; then
                case "$runtime" in *'#'*|*'@'*) sd300_fail 'GUI runtime path contains an unsupported cache delimiter' ;; esac
                sed "s#@SD300_RUNTIME@#${runtime}#g" "$cache" > "${cache}.new" || sd300_fail 'could not configure the private GTK loader cache'
                mv "${cache}.new" "$cache"
            fi
            desktop_source="$sd300_gui_root/share/applications/sd300.desktop"
            mkdir -p "$(dirname "$sd300_gui_desktop")"
            sed "s#@SD300_GUI@#${gui_binary}#g" "$desktop_source" > "$sd300_gui_desktop" || sd300_fail 'could not install the Linux desktop entry'
            chmod 644 "$sd300_gui_desktop"
            ;;
    esac
    mkdir -p "$(dirname "$marker")" || sd300_fail 'could not create the GUI ownership directory'
    printf '%s\n' "{\"schema\":1,\"product\":\"SD-300\",\"version\":\"${sd300_version}\",\"owner\":\"shell-installer\"}" > "$marker" || sd300_fail 'could not write the GUI ownership marker'
    chmod 600 "$marker" || sd300_fail 'could not protect the GUI ownership marker'
    result=$($gui_binary --self-test --json 2>/dev/null) || sd300_fail 'installed GUI self-test failed'
    printf '%s\n' "$result" | grep -Eq '"success"[[:space:]]*:[[:space:]]*true' || sd300_fail 'installed GUI did not report success'
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
    if printf '%s\n' "$payload_files" | sed 's#^\./##; s#^/##' | grep -Fxq 'Applications/SD-300.app/Contents/MacOS/sd300-gui'; then
        [ -d /Applications/SD-300.app ] || return 1
        codesign --verify --deep --strict /Applications/SD-300.app >/dev/null 2>&1 || return 1
    elif [ -e /Applications/SD-300.app ]; then
        return 1
    fi
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
    if [ -d /Applications/SD-300.app ]; then
        /usr/bin/ditto /Applications/SD-300.app "$sd300_temp/prior-pkg-app" ||
            sd300_fail 'could not back up the receipt-owned PKG application'
    fi
    sd300_pkg_present=1
}

sd300_take_over_macos_pkg() {
    [ "$sd300_pkg_present" -eq 1 ] || return 0
    sd300_pkg_is_exact_product || sd300_fail 'the PKG receipt/payload/signature evidence conflicts; preserving it'

    printf '%s\n' 'Switching SD-300 ownership from macos-pkg to shell-installer...'
    sudo -v || sd300_fail 'administrator authorization was cancelled; the existing PKG was preserved'
    sudo rm -f /usr/local/bin/sd300 || sd300_fail 'could not remove the receipt-owned PKG payload'
    sd300_pkg_payload_removed=1
    sudo rm -rf /Applications/SD-300.app || sd300_fail 'could not remove the receipt-owned PKG application'
    sudo pkgutil --forget com.qubetx.sd300.pkg >/dev/null \
        || sd300_fail 'could not forget the SD-300 PKG receipt'
    if [ -e /usr/local/bin/sd300 ] || [ -e /Applications/SD-300.app ] || pkgutil --pkg-info com.qubetx.sd300.pkg >/dev/null 2>&1; then
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
sd300_resolve_gui_target
sd300_prepare_macos_pkg
sd300_save_managed_state
sd300_save_gui_state
sd300_stop_owned_gui
sd300_stage_gui_payload
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
sd300_install_gui_payload
if [ -n "$sd300_prior_binary" ] && [ "$sd300_prior_binary" != "$managed_binary" ]; then
    rm -f "$sd300_prior_binary" || sd300_fail 'could not remove the prior managed install path'
fi
sd300_verify_receipt
sd300_verify_binary >/dev/null || sd300_fail 'final managed SD-300 verification did not complete'
sd300_take_over_macos_pkg
sd300_committed=1
printf '%s\n' "SD-300 ${sd300_version} is installed through the managed shell channel: ${managed_binary}"
