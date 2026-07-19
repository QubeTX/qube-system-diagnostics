#!/usr/bin/env bash
# Prove the Developer ID Installer certificate and private key on native macOS.

set -euo pipefail

required_vars=(
    APPLE_INSTALLER_CERTIFICATE_P12_BASE64
    APPLE_INSTALLER_CERTIFICATE_PASSWORD
    APPLE_INSTALLER_SIGNING_IDENTITY
    APPLE_TEAM_ID
)
for name in "${required_vars[@]}"; do
    if [[ -z ${!name:-} ]]; then
        echo "required Apple Installer credential is unavailable: $name" >&2
        exit 78
    fi
done

runner_temp=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
work_dir=$(mktemp -d "${runner_temp%/}/sd300-installer-preflight.XXXXXX")
keychain="${work_dir}/sd300-installer-preflight.keychain-db"
keychain_password=$(openssl rand -base64 32)
p12="${work_dir}/developer-id-installer.p12"

original_user_keychains=()
while IFS= read -r line; do
    path=${line#*\"}
    path=${path%\"*}
    [[ -n $path ]] && original_user_keychains+=("$path")
done < <(security list-keychains -d user)

cleanup() {
    security list-keychains -d user -s "${original_user_keychains[@]}" \
        >/dev/null 2>&1 || true
    security delete-keychain "$keychain" >/dev/null 2>&1 || true
    rm -rf "$work_dir"
}
trap cleanup EXIT INT TERM

chmod 700 "$work_dir"
printf '%s' "$APPLE_INSTALLER_CERTIFICATE_P12_BASE64" | \
    /usr/bin/base64 -D > "$p12"
chmod 600 "$p12"

security create-keychain -p "$keychain_password" "$keychain"
security set-keychain-settings -lut 21600 "$keychain"
security unlock-keychain -p "$keychain_password" "$keychain"
# GitHub's documented hosted-runner pattern explicitly selects PKCS#12 and
# grants access inside this disposable keychain. The partition list below
# restricts non-interactive Apple tooling, and cleanup deletes the keychain.
security import "$p12" -k "$keychain" \
    -P "$APPLE_INSTALLER_CERTIFICATE_PASSWORD" \
    -A -f pkcs12
security set-key-partition-list -S apple-tool:,apple: -s \
    -k "$keychain_password" "$keychain" >/dev/null
security list-keychains -d user -s "$keychain" "${original_user_keychains[@]}"

leaf_pem="${work_dir}/installer-leaf.pem"
security find-certificate -c "$APPLE_INSTALLER_SIGNING_IDENTITY" \
    -p "$keychain" > "$leaf_pem"
subject=$(openssl x509 -in "$leaf_pem" -noout -subject -nameopt RFC2253)
issuer=$(openssl x509 -in "$leaf_pem" -noout -issuer -nameopt RFC2253)
details=$(openssl x509 -in "$leaf_pem" -noout -text)

grep -Fq "CN=${APPLE_INSTALLER_SIGNING_IDENTITY}" <<< "$subject"
grep -Eq "(^|,)OU=${APPLE_TEAM_ID}(,|$)" <<< "$subject"
grep -Eq '(^|,)OU=G2(,|$)' <<< "$issuer"
grep -Fq 'Public-Key: (2048 bit)' <<< "$details"
grep -Fq '1.2.840.113635.100.4.13' <<< "$details"
openssl x509 -in "$leaf_pem" -checkend 86400 -noout

# A signed disposable package proves that the imported certificate has its
# matching private key and is usable by Apple's package-signing toolchain.
payload="${work_dir}/payload"
mkdir -p "${payload}/usr/local/share/sd300"
printf '%s\n' 'SD-300 Apple Installer credential preflight' > \
    "${payload}/usr/local/share/sd300/preflight.txt"
pkg="${work_dir}/installer-credential-preflight.pkg"
pkgbuild --root "$payload" \
    --identifier com.qubetx.sd300.installer-credential-preflight \
    --version 0 \
    --install-location / \
    --sign "$APPLE_INSTALLER_SIGNING_IDENTITY" \
    --keychain "$keychain" \
    "$pkg"

signature=$(pkgutil --check-signature "$pkg")
grep -Fq "Developer ID Installer: " <<< "$signature"
grep -Fq "$APPLE_TEAM_ID" <<< "$signature"

echo "Developer ID Installer credential imported and signed a verified PKG."
