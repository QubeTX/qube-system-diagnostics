#!/usr/bin/env bash
# Build the universal, signed, notarized SD-300 PKG distribution.
# The managed shell channel remains preferred; this is the direct native option.
# Runs only on an ephemeral native macOS GitHub runner.

set -euo pipefail

if [[ $# -ne 6 ]]; then
    echo "usage: $0 <version> <arm64-cli-archive> <x86_64-cli-archive> <arm64-gui-archive> <x86_64-gui-archive> <output-dir>" >&2
    exit 64
fi

version=${1#v}
arm_archive=$2
x86_archive=$3
arm_gui_archive=$4
x86_gui_archive=$5
output_dir=$6

required_vars=(
    APPLE_CERTIFICATE_P12_BASE64
    APPLE_CERTIFICATE_PASSWORD
    APPLE_INSTALLER_CERTIFICATE_P12_BASE64
    APPLE_INSTALLER_CERTIFICATE_PASSWORD
    APPLE_API_KEY_P8_BASE64
    APPLE_API_KEY_ID
    APPLE_API_ISSUER_ID
    APPLE_SIGNING_IDENTITY
    APPLE_INSTALLER_SIGNING_IDENTITY
    APPLE_TEAM_ID
)
for name in "${required_vars[@]}"; do
    if [[ -z ${!name:-} ]]; then
        echo "required Apple release credential is unavailable: $name" >&2
        exit 78
    fi
done

for archive in "$arm_archive" "$x86_archive" "$arm_gui_archive" "$x86_gui_archive"; do
    if [[ ! -f $archive ]]; then
        echo "required macOS archive is missing: $archive" >&2
        exit 66
    fi
done

runner_temp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
work_dir=$(mktemp -d "${runner_temp%/}/sd300-macos-installer.XXXXXX")
keychain="${work_dir}/sd300-release.keychain-db"
keychain_password=$(openssl rand -base64 32)
credential_dir="${work_dir}/credentials"
mkdir -m 700 "$credential_dir" "$output_dir"
# Absolute from here on: later steps write archives from inside subshells that
# have already changed directory, where a relative output path cannot resolve.
output_dir=$(cd "$output_dir" && pwd)
chmod 700 "$work_dir"

original_user_keychains=()
while IFS= read -r line; do
    path=${line#*\"}
    path=${path%\"*}
    [[ -n $path ]] && original_user_keychains+=("$path")
done < <(security list-keychains -d user)

cleanup() {
    security list-keychains -d user -s "${original_user_keychains[@]}" >/dev/null 2>&1 || true
    security delete-keychain "$keychain" >/dev/null 2>&1 || true
    rm -rf "$work_dir"
}
trap cleanup EXIT INT TERM

app_p12="${credential_dir}/developer-id-application.p12"
installer_p12="${credential_dir}/developer-id-installer.p12"
api_key="${credential_dir}/AuthKey_${APPLE_API_KEY_ID}.p8"
printf '%s' "$APPLE_CERTIFICATE_P12_BASE64" | /usr/bin/base64 -D > "$app_p12"
printf '%s' "$APPLE_INSTALLER_CERTIFICATE_P12_BASE64" | /usr/bin/base64 -D > "$installer_p12"
printf '%s' "$APPLE_API_KEY_P8_BASE64" | /usr/bin/base64 -D > "$api_key"
chmod 600 "$app_p12" "$installer_p12" "$api_key"

security create-keychain -p "$keychain_password" "$keychain"
security set-keychain-settings -lut 21600 "$keychain"
security unlock-keychain -p "$keychain_password" "$keychain"
# Explicit PKCS#12 selection follows GitHub's hosted-runner import pattern.
# `-A` applies only to this disposable keychain; the partition list below
# enables non-interactive Apple tools and cleanup deletes the keychain.
security import "$app_p12" -k "$keychain" -P "$APPLE_CERTIFICATE_PASSWORD" \
    -A -f pkcs12
security import "$installer_p12" -k "$keychain" \
    -P "$APPLE_INSTALLER_CERTIFICATE_PASSWORD" -A -f pkcs12
security set-key-partition-list -S apple-tool:,apple: -s -k "$keychain_password" "$keychain" >/dev/null
security list-keychains -d user -s "$keychain" "${original_user_keychains[@]}"

application_identities=$(security find-identity -v -p codesigning "$keychain")
if ! grep -Fq "$APPLE_SIGNING_IDENTITY" <<< "$application_identities"; then
    echo "configured Developer ID Application identity was not found in the ephemeral keychain" >&2
    exit 1
fi
# Installer identities are package-signing certificates, not code-signing
# identities, so `security find-identity -p codesigning` will not list them.
# The repository variable stores the full Developer ID Installer common name;
# require that exact certificate in the isolated keychain before pkgbuild.
if ! security find-certificate -c "$APPLE_INSTALLER_SIGNING_IDENTITY" \
    "$keychain" >/dev/null; then
    echo "configured Developer ID Installer certificate was not found in the ephemeral keychain" >&2
    exit 1
fi

arm_dir="${work_dir}/arm64"
x86_dir="${work_dir}/x86_64"
mkdir "$arm_dir" "$x86_dir"
COPYFILE_DISABLE=1 tar -xJf "$arm_archive" -C "$arm_dir"
COPYFILE_DISABLE=1 tar -xJf "$x86_archive" -C "$x86_dir"
arm_binary=$(find "$arm_dir" -type f -name sd300 -perm -111 -print -quit)
x86_binary=$(find "$x86_dir" -type f -name sd300 -perm -111 -print -quit)
if [[ -z $arm_binary || -z $x86_binary ]]; then
    echo "could not locate both architecture-specific sd300 binaries" >&2
    exit 65
fi

universal="${work_dir}/sd300"
lipo -create "$arm_binary" "$x86_binary" -output "$universal"
chmod 755 "$universal"
# Xcode 16.4 requires the input file before -verify_arch. Keep this ordering
# in lockstep with every post-install validation call in the hosted workflow.
lipo "$universal" -verify_arch arm64 x86_64
codesign --force --identifier com.qubetx.sd300 --options runtime --timestamp \
    --keychain "$keychain" --sign "$APPLE_SIGNING_IDENTITY" "$universal"
codesign --verify --strict --verbose=4 "$universal"
details=$(codesign -d --verbose=4 "$universal" 2>&1)
grep -Fqx 'Identifier=com.qubetx.sd300' <<< "$details"
grep -Fqx "TeamIdentifier=${APPLE_TEAM_ID}" <<< "$details"
grep -Eq '^CodeDirectory .*flags=.*\(runtime\)' <<< "$details"
grep -Eq '^Timestamp=.+' <<< "$details"

notarize() {
    local artifact=$1
    local result
    result="${work_dir}/notary-$(basename "$artifact").json"
    xcrun notarytool submit "$artifact" \
        --key "$api_key" \
        --key-id "$APPLE_API_KEY_ID" \
        --issuer "$APPLE_API_ISSUER_ID" \
        --wait --output-format json > "$result"
    local status submission
    status=$(jq -r '.status // empty' "$result")
    submission=$(jq -r '.id // empty' "$result")
    if [[ $status != Accepted ]]; then
        if [[ -n $submission ]]; then
            xcrun notarytool log "$submission" \
                --key "$api_key" --key-id "$APPLE_API_KEY_ID" \
                --issuer "$APPLE_API_ISSUER_ID" || true
        fi
        echo "Apple notarization failed for $(basename "$artifact"): ${status:-unknown}" >&2
        exit 1
    fi
    echo "Apple notarization accepted for $(basename "$artifact") (${submission})."
}

verify_pkg_signature() {
    local artifact=$1
    local signature
    signature=$(pkgutil --check-signature "$artifact" 2>&1)
    printf '%s\n' "$signature"
    grep -Fq 'Status: signed by a developer certificate' <<< "$signature"
    grep -Fq "$APPLE_INSTALLER_SIGNING_IDENTITY" <<< "$signature"
    grep -Fq "(${APPLE_TEAM_ID})" <<< "$signature"
}

arm_gui_dir="${work_dir}/gui-arm64"
x86_gui_dir="${work_dir}/gui-x86_64"
mkdir "$arm_gui_dir" "$x86_gui_dir"
COPYFILE_DISABLE=1 tar -xzf "$arm_gui_archive" -C "$arm_gui_dir"
COPYFILE_DISABLE=1 tar -xzf "$x86_gui_archive" -C "$x86_gui_dir"
arm_gui_binary=$(find "$arm_gui_dir" -type f -name sd300-gui -perm -111 -print -quit)
x86_gui_binary=$(find "$x86_gui_dir" -type f -name sd300-gui -perm -111 -print -quit)
arm_gui_engine=$(find "$arm_gui_dir" -type f -name libsd300_engine.dylib -print -quit)
x86_gui_engine=$(find "$x86_gui_dir" -type f -name libsd300_engine.dylib -print -quit)
arm_gui_notices=$(find "$arm_gui_dir" -type d -name licenses -print -quit)
x86_gui_notices=$(find "$x86_gui_dir" -type d -name licenses -print -quit)
if [[ -z $arm_gui_binary || -z $x86_gui_binary || -z $arm_gui_engine || -z $x86_gui_engine ||
      -z $arm_gui_notices || -z $x86_gui_notices ]]; then
    echo 'could not locate both architecture-specific GUI binaries and engines' >&2
    exit 65
fi
for notice in PRODUCT-LICENSE.md IBM-PLEX-OFL-1.1.txt NATIVE-SDK-APACHE-2.0.txt; do
    [[ -s "$arm_gui_notices/$notice" && -s "$x86_gui_notices/$notice" ]] || {
        echo "required GUI notice is missing: $notice" >&2
        exit 65
    }
    cmp -s "$arm_gui_notices/$notice" "$x86_gui_notices/$notice" || {
        echo "GUI notice differs between architecture builds: $notice" >&2
        exit 65
    }
done

script_dir=$(cd "$(dirname "$0")" && pwd)
gui_project=$(cd "$script_dir/../gui" && pwd)
(cd "$gui_project" && npm ci --ignore-scripts)
app_bundle="${work_dir}/SD-300.app"
(cd "$gui_project" && npx --no-install native package \
    --target macos --output "$app_bundle" --binary "$x86_gui_binary" \
    --assets assets --signing none)
app_executable="${app_bundle}/Contents/MacOS/sd300-gui"
app_engine="${app_bundle}/Contents/MacOS/libsd300_engine.dylib"
[[ -d $app_bundle && -f $app_executable ]] || { echo 'Native SDK packager did not produce the expected app bundle' >&2; exit 1; }
app_icon="${app_bundle}/Contents/Resources/AppIcon.icns"
runtime_app_icon="${app_bundle}/Contents/Resources/assets/app-icon.png"
tray_template="${app_bundle}/Contents/Resources/assets/tray-icon-template.png"
for required_identity_asset in "$app_icon" "$runtime_app_icon" "$tray_template"; do
    [[ -s $required_identity_asset ]] || {
        echo "macOS app bundle is missing an SD-300 identity asset: $required_identity_asset" >&2
        exit 1
    }
done
grep -Fq '<string>AppIcon.icns</string>' "${app_bundle}/Contents/Info.plist" || {
    echo 'macOS app bundle does not declare the generated SD-300 application icon' >&2
    exit 1
}
app_notices="${app_bundle}/Contents/Resources/licenses"
mkdir -p "$app_notices"
for notice in PRODUCT-LICENSE.md IBM-PLEX-OFL-1.1.txt NATIVE-SDK-APACHE-2.0.txt; do
    install -m 644 "$x86_gui_notices/$notice" "$app_notices/$notice"
done
lipo -create "$arm_gui_binary" "$x86_gui_binary" -output "${app_executable}.universal"
mv "${app_executable}.universal" "$app_executable"
lipo -create "$arm_gui_engine" "$x86_gui_engine" -output "$app_engine"
chmod 755 "$app_executable" "$app_engine"
lipo "$app_executable" -verify_arch arm64 x86_64
lipo "$app_engine" -verify_arch arm64 x86_64
gui_self_test=$($app_executable --self-test --json)
jq -e --arg version "$version" '
  .success == true and .product == "SD-300" and .product_version == $version and
  .abi_version == 1 and .engine_schema_version == 1
' <<< "$gui_self_test" >/dev/null

codesign --force --identifier dev.qubetx.sd300.engine --options runtime --timestamp \
    --keychain "$keychain" --sign "$APPLE_SIGNING_IDENTITY" "$app_engine"
codesign --force --identifier dev.qubetx.sd300.gui --options runtime --timestamp \
    --keychain "$keychain" --sign "$APPLE_SIGNING_IDENTITY" "$app_executable"
codesign --force --options runtime --timestamp \
    --entitlements "$gui_project/node_modules/@native-sdk/cli/assets/native-sdk.entitlements" \
    --keychain "$keychain" --sign "$APPLE_SIGNING_IDENTITY" "$app_bundle"
codesign --verify --deep --strict --verbose=4 "$app_bundle"
app_details=$(codesign -d --verbose=4 "$app_bundle" 2>&1)
grep -Fqx 'Identifier=dev.qubetx.sd300' <<< "$app_details"
grep -Fqx "TeamIdentifier=${APPLE_TEAM_ID}" <<< "$app_details"
grep -Eq '^CodeDirectory .*flags=.*\(runtime\)' <<< "$app_details"

gui_notary_zip="${work_dir}/sd300-gui-notary.zip"
/usr/bin/ditto -c -k --keepParent "$app_bundle" "$gui_notary_zip"
notarize "$gui_notary_zip"
xcrun stapler staple "$app_bundle"
xcrun stapler validate "$app_bundle"
spctl --assess --type execute --verbose=4 "$app_bundle"

gui_dist="${work_dir}/gui-dist"
mkdir "$gui_dist"
/usr/bin/ditto "$app_bundle" "$gui_dist/SD-300.app"
node "$script_dir/write-gui-install-manifest.mjs" "$gui_dist" \
    macos-universal 'SD-300.app/Contents/MacOS/sd300-gui' \
    'SD-300.app/Contents/MacOS/libsd300_engine.dylib' "$version"
gui_archive="${output_dir}/sd300-gui-macos-universal.zip"
(cd "$gui_dist" && /usr/bin/zip -qry -X "$gui_archive" SD-300.app install-manifest.json)
gui_sha=$(shasum -a 256 "$gui_archive" | awk '{print $1}')
printf '%s *%s\n' "$gui_sha" "$(basename "$gui_archive")" > "${gui_archive}.sha256"
(cd "$output_dir" && shasum -a 256 -c "$(basename "$gui_archive").sha256")

binary_zip="${work_dir}/sd300-universal-notary.zip"
/usr/bin/ditto -c -k --keepParent "$universal" "$binary_zip"
notarize "$binary_zip"

payload="${work_dir}/payload"
install -d -m 755 "${payload}/usr/local/bin" "${payload}/Applications"
install -m 755 "$universal" "${payload}/usr/local/bin/sd300"
/usr/bin/ditto "$app_bundle" "${payload}/Applications/SD-300.app"
pkg_scripts="${work_dir}/pkg-scripts"
mkdir -m 755 "$pkg_scripts"
install -m 755 "$universal" "${pkg_scripts}/sd300-lifecycle"
cat > "${pkg_scripts}/preinstall" <<'PREINSTALL'
#!/bin/sh
# Package scripts run as root, but the GUI lifecycle socket is intentionally
# private to the user who owns the process. Use the signed, package-matched CLI
# helper as each running GUI owner so the request is authenticated and the CLI
# can prove that the process exited before Installer replaces any payload file.
set -u

gui_pids=$(/usr/bin/pgrep -x sd300-gui 2>/dev/null || true)
[ -n "$gui_pids" ] || exit 0

gui_uids=$(
    for gui_pid in $gui_pids; do
        /bin/ps -o uid= -p "$gui_pid" 2>/dev/null | /usr/bin/tr -d ' '
    done | /usr/bin/awk '/^[0-9]+$/' | /usr/bin/sort -u
)
if [ -z "$gui_uids" ]; then
    echo 'SD-300: a GUI process is running but its owner could not be resolved; the package stopped before changing files.' >&2
    exit 1
fi

script_dir=$(/usr/bin/dirname "$0")
lifecycle_helper="${script_dir}/sd300-lifecycle"
if [ ! -x "$lifecycle_helper" ]; then
    echo 'SD-300: the signed GUI lifecycle helper is unavailable; the package stopped before changing files.' >&2
    exit 1
fi

for gui_uid in $gui_uids; do
    gui_user=$(/usr/bin/id -nu "$gui_uid" 2>/dev/null || true)
    if [ -z "$gui_user" ]; then
        echo "SD-300: GUI owner ${gui_uid} has no local account; the package stopped before changing files." >&2
        exit 1
    fi
    if [ "$gui_uid" -eq 0 ]; then
        "$lifecycle_helper" stop-gui --quiet || exit 1
    else
        /usr/bin/sudo -u "$gui_user" "$lifecycle_helper" stop-gui --quiet || exit 1
    fi
done

if /usr/bin/pgrep -x sd300-gui >/dev/null 2>&1; then
    echo 'SD-300: a GUI process remained after the authenticated lifecycle request; the package stopped before changing files.' >&2
    exit 1
fi
exit 0
PREINSTALL
cat > "${pkg_scripts}/postinstall" <<'POSTINSTALL'
#!/bin/sh
# A deliberately launched PKG is the user's newest install-channel choice.
# Remove only the active console user's allowlisted Cargo/cargo-dist copy; the
# installed Rust helper validates the exact binary name and standard per-user
# cargo-dist receipt. A custom receipt root is not guessed or searched broadly.
set -u

console_user=$(/usr/bin/stat -f '%Su' /dev/console 2>/dev/null || true)
case "$console_user" in
    ''|root|loginwindow|_mbsetupuser)
        if [ -n "${SUDO_USER:-}" ] && [ "${SUDO_USER}" != root ]; then
            console_user=$SUDO_USER
        else
            # Headless native CI/MDM has no logged-in console user. Recover only
            # when exactly one normal home contains the allowlisted CLI copy or
            # its standard receipt. Zero means there is nothing to converge;
            # multiple user owners are ambiguous and must fail the package.
            candidates=''
            for home in /Users/*; do
                if [ ! -e "${home}/.cargo/bin/sd300" ] && \
                    [ ! -e "${home}/.config/sd300/sd300-receipt.json" ]; then
                    continue
                fi
                candidates="${candidates}${home}\n"
            done
            candidate_count=$(printf '%b' "$candidates" | /usr/bin/grep -c '^/Users/' || true)
            [ "$candidate_count" -ne 0 ] || exit 0
            if [ "$candidate_count" -ne 1 ]; then
                echo "SD-300: multiple per-user CLI owners exist and no console user identifies the intended one; preserving them and failing PKG takeover." >&2
                exit 1
            fi
            user_home=$(printf '%b' "$candidates" | /usr/bin/sed -n '1p')
            console_user=${user_home##*/}
        fi
        ;;
esac

if [ -z "${user_home:-}" ]; then
    user_home=$(/usr/bin/dscl . -read "/Users/${console_user}" NFSHomeDirectory 2>/dev/null \
        | /usr/bin/awk '{print $2}')
fi
if [ -z "$user_home" ] || [ ! -d "$user_home" ]; then
    echo "SD-300: could not resolve the active user's home; preserving any CLI install." >&2
    exit 0
fi

managed_binary="${user_home}/.cargo/bin/sd300"
managed_receipt="${user_home}/.config/sd300/sd300-receipt.json"
managed_app="${user_home}/Applications/SD-300.app"
managed_app_owner="${user_home}/Library/Application Support/SD-300/managed-install-owner.json"
rollback_dir=$(/usr/bin/mktemp -d "${TMPDIR:-/tmp}/sd300-pkg-takeover.XXXXXXXX") || exit 1
managed_binary_existed=0
managed_receipt_existed=0
managed_app_existed=0
managed_app_owner_existed=0
takeover_committed=0
rollback_managed() {
    if [ "$takeover_committed" -eq 0 ]; then
        if [ "$managed_binary_existed" -eq 1 ]; then
            /bin/mkdir -p "$(/usr/bin/dirname "$managed_binary")"
            /bin/cp -p "$rollback_dir/sd300" "$managed_binary" || true
        fi
        if [ "$managed_receipt_existed" -eq 1 ]; then
            /bin/mkdir -p "$(/usr/bin/dirname "$managed_receipt")"
            /bin/cp -p "$rollback_dir/sd300-receipt.json" "$managed_receipt" || true
        fi
        if [ "$managed_app_existed" -eq 1 ]; then
            /bin/mkdir -p "$user_home/Applications"
            /usr/bin/ditto "$rollback_dir/SD-300.app" "$managed_app" || true
        fi
        if [ "$managed_app_owner_existed" -eq 1 ]; then
            /bin/mkdir -p "$(/usr/bin/dirname "$managed_app_owner")"
            /bin/cp -p "$rollback_dir/managed-install-owner.json" "$managed_app_owner" || true
        fi
    fi
    /bin/rm -rf "$rollback_dir"
    takeover_committed=1
}
trap rollback_managed EXIT HUP INT TERM
if [ -f "$managed_binary" ]; then
    /bin/cp -p "$managed_binary" "$rollback_dir/sd300" || exit 1
    managed_binary_existed=1
fi
if [ -f "$managed_receipt" ]; then
    /bin/cp -p "$managed_receipt" "$rollback_dir/sd300-receipt.json" || exit 1
    managed_receipt_existed=1
fi
if [ -e "$managed_app" ] || [ -e "$managed_app_owner" ]; then
    [ -d "$managed_app" ] && [ -f "$managed_app_owner" ] || {
        echo 'SD-300: per-user GUI ownership evidence is incomplete; preserving it and failing PKG takeover.' >&2
        exit 1
    }
    /usr/bin/grep -Eq '"schema"[[:space:]]*:[[:space:]]*1' "$managed_app_owner" &&
        /usr/bin/grep -Eq '"product"[[:space:]]*:[[:space:]]*"SD-300"' "$managed_app_owner" &&
        /usr/bin/grep -Eq '"owner"[[:space:]]*:[[:space:]]*"shell-installer"' "$managed_app_owner" || {
            echo 'SD-300: per-user GUI ownership marker is ambiguous; preserving it and failing PKG takeover.' >&2
            exit 1
        }
    /usr/bin/ditto "$managed_app" "$rollback_dir/SD-300.app" || exit 1
    /bin/cp -p "$managed_app_owner" "$rollback_dir/managed-install-owner.json" || exit 1
    managed_app_existed=1
    managed_app_owner_existed=1
fi

/usr/local/bin/sd300 migrate-cleanup --quiet --strict --cargo-copy --user-profile "$user_home"
if [ -e "$managed_binary" ]; then
    echo "SD-300: the prior Cargo-path copy could not be removed; PKG takeover is incomplete." >&2
    exit 1
fi
if [ -e "$managed_receipt" ]; then
    echo "SD-300: the matching managed-installer receipt could not be removed; PKG takeover is incomplete." >&2
    exit 1
fi
/bin/rm -rf "$managed_app" || exit 1
/bin/rm -f "$managed_app_owner" || exit 1
if [ -e "$managed_app" ] || [ -e "$managed_app_owner" ]; then
    echo 'SD-300: the prior managed GUI could not be removed; PKG takeover is incomplete.' >&2
    exit 1
fi
takeover_committed=1
exit 0
POSTINSTALL
chmod 755 "${pkg_scripts}/preinstall" "${pkg_scripts}/postinstall"
pkg="${work_dir}/sd300.pkg"
pkgbuild --root "$payload" \
    --scripts "$pkg_scripts" \
    --identifier com.qubetx.sd300.pkg \
    --version "$version" \
    --install-location / \
    --sign "$APPLE_INSTALLER_SIGNING_IDENTITY" \
    --keychain "$keychain" \
    "$pkg"
verify_pkg_signature "$pkg"
notarize "$pkg"
xcrun stapler staple "$pkg"
xcrun stapler validate "$pkg"
spctl --assess --type install --verbose=4 "$pkg"

direct_pkg="${output_dir}/sd300-macos-universal.pkg"
cp "$pkg" "$direct_pkg"
verify_pkg_signature "$direct_pkg"
xcrun stapler validate "$direct_pkg"
spctl --assess --type install --verbose=4 "$direct_pkg"

pkg_sha=$(shasum -a 256 "$direct_pkg" | awk '{print $1}')
printf '%s *%s\n' "$pkg_sha" "$(basename "$direct_pkg")" > "${direct_pkg}.sha256"
(
    cd "$output_dir"
    shasum -a 256 -c "$(basename "$direct_pkg").sha256"
)

echo "Built signed, notarized, stapled universal PKG: $direct_pkg"
