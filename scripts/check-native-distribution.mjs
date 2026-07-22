#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptRoot = dirname(fileURLToPath(import.meta.url));
const guiRoot = resolve(process.argv[2] ?? resolve(scriptRoot, "..", "gui"));
const packageRoot = process.argv[3] ? resolve(process.argv[3]) : undefined;
const repoRoot = resolve(guiRoot, "..");
const toolchain = JSON.parse(readFileSync(resolve(guiRoot, "toolchain-lock.json"), "utf8"));
const zon = readFileSync(resolve(guiRoot, "build.zig.zon"), "utf8");
const requiredVersion = toolchain.native_sdk?.version;
const requiredUrl = toolchain.native_sdk?.tarball;
const requiredIntegrity = toolchain.native_sdk?.npm_integrity;
const requiredHash = toolchain.native_sdk?.zig_content_hash;
const requiredGitHead = toolchain.native_sdk?.npm_git_head;
const requiredPatchPath = toolchain.native_sdk?.renderer_patch;
const requiredPatchHash = toolchain.native_sdk?.renderer_patch_sha256;
const requiredMakiraHash = toolchain.fonts?.makira?.sha256;
if (toolchain.schema !== 1 || !requiredVersion || !requiredUrl || !requiredIntegrity ||
    !requiredHash || !requiredPatchPath || !/^[0-9a-f]{64}$/.test(requiredPatchHash ?? "") ||
    !/^[0-9a-f]{64}$/.test(requiredMakiraHash ?? "") ||
    toolchain.fonts?.makira?.source !== "licensed-repository-secret" ||
    !/^[0-9a-f]{40}$/.test(requiredGitHead ?? "")) {
  throw new Error("gui/toolchain-lock.json is incomplete.");
}
const patchPath = resolve(guiRoot, requiredPatchPath);
if (!statSync(patchPath, { throwIfNoEntry: false })?.isFile() ||
    createHash("sha256").update(readFileSync(patchPath)).digest("hex") !== requiredPatchHash) {
  throw new Error("The Native SDK renderer patch disagrees with gui/toolchain-lock.json.");
}
if (!zon.includes(requiredUrl) || !zon.includes(requiredHash)) throw new Error("Native SDK dependency pins are incomplete.");
if (/\.path\s*=|AppData[\\/](?:Local|Roaming)|node_modules[\\/]@native-sdk[\\/]cli/i.test(zon)) {
  throw new Error("Developer-local Native SDK dependency detected in build.zig.zon.");
}

const packageManifest = JSON.parse(readFileSync(resolve(guiRoot, "package.json"), "utf8"));
const packageLock = JSON.parse(readFileSync(resolve(guiRoot, "package-lock.json"), "utf8"));
const lockedSdk = packageLock.packages?.["node_modules/@native-sdk/cli"];
if (packageManifest.devDependencies?.["@native-sdk/cli"] !== requiredVersion ||
    lockedSdk?.version !== requiredVersion || lockedSdk?.resolved !== requiredUrl ||
    lockedSdk?.integrity !== requiredIntegrity) {
  throw new Error("npm manifest, lockfile, and reviewed Native SDK toolchain record disagree.");
}
for (const [name, version] of Object.entries(lockedSdk?.optionalDependencies ?? {})) {
  const locked = packageLock.packages?.[`node_modules/${name}`];
  if (version !== requiredVersion || locked?.version !== requiredVersion ||
      !locked?.resolved?.startsWith("https://registry.npmjs.org/@native-sdk/") ||
      !/^sha512-[A-Za-z0-9+/]+={0,2}$/.test(locked?.integrity ?? "")) {
    throw new Error(`Native SDK optional host package is not immutable: ${name}.`);
  }
}

for (const [platform, archive] of Object.entries(toolchain.zig?.archives ?? {})) {
  if (!archive?.url?.includes(`/0.16.0/`) || !/^[0-9a-f]{64}$/.test(archive?.sha256 ?? "")) {
    throw new Error(`Zig archive pin is incomplete for ${platform}.`);
  }
}
for (const platform of ["x86_64-windows", "x86_64-macos", "aarch64-macos", "x86_64-linux", "aarch64-linux"]) {
  if (!toolchain.zig?.archives?.[platform]) throw new Error(`Zig archive pin is missing for ${platform}.`);
}
if (toolchain.zig?.version !== "0.16.0") throw new Error("The reviewed Zig toolchain must be 0.16.0.");
if (!readFileSync(resolve(repoRoot, "rust-toolchain.toml"), "utf8").includes(`channel = "${toolchain.rust?.channel}"`)) {
  throw new Error("rust-toolchain.toml disagrees with gui/toolchain-lock.json.");
}
const rustAction = toolchain.rust?.github_action;
if (!/^dtolnay\/rust-toolchain@[0-9a-f]{40}$/.test(rustAction ?? "")) {
  throw new Error("The Rust setup action must be recorded at an immutable commit.");
}
for (const relativePath of [
  ".github/workflows/windows-installers.yml",
  ".github/workflows/macos-installer.yml",
  ".github/workflows/linux-native-gui.yml",
]) {
  const source = readFileSync(resolve(repoRoot, relativePath), "utf8");
  if (!source.includes(`uses: ${rustAction}`) || !source.includes(`toolchain: "${toolchain.rust.channel}"`)) {
    throw new Error(`${relativePath} disagrees with the reviewed Rust toolchain record.`);
  }
}
const muslBuilder = readFileSync(resolve(repoRoot, "scripts/build-native-gui-musl-container.sh"), "utf8");
const muslZig = toolchain.zig?.archives?.["x86_64-linux"];
if (!muslBuilder.includes(`--default-toolchain ${toolchain.rust.channel}`) ||
    !muslBuilder.includes(`== ${toolchain.rust.channel}.0`) ||
    !muslBuilder.includes(muslZig?.url ?? "missing-zig-url") ||
    !muslBuilder.includes(muslZig?.sha256 ?? "missing-zig-hash")) {
  throw new Error("The Alpine musl builder disagrees with the reviewed Rust toolchain record.");
}
for (const relativePath of ["scripts/build-native-gui.sh", "scripts/build-native-gui.ps1"]) {
  const source = readFileSync(resolve(repoRoot, relativePath), "utf8");
  if (!source.includes(`${toolchain.rust.channel}.0`) || !source.includes(toolchain.zig.version)) {
    throw new Error(`${relativePath} does not enforce the reviewed Rust/Zig versions.`);
  }
}

for (const relativePath of ["scripts/prepare-native-sdk.mjs", "scripts/prepare-native-sdk.ps1"]) {
  const source = readFileSync(resolve(repoRoot, relativePath), "utf8");
  for (const value of [requiredVersion, requiredUrl, requiredIntegrity, requiredHash, requiredPatchHash]) {
    if (!source.includes(value)) throw new Error(`${relativePath} disagrees with gui/toolchain-lock.json.`);
  }
}

const makiraPrepare = readFileSync(resolve(repoRoot, "scripts/prepare-makira-font.mjs"), "utf8");
const gitignore = readFileSync(resolve(repoRoot, ".gitignore"), "utf8");
for (const marker of [
  "SD300_MAKIRA_FONT_BROTLI_BASE64_PART_1",
  "SD300_MAKIRA_FONT_BROTLI_BASE64_PART_2",
  "brotliDecompressSync",
]) {
  if (!makiraPrepare.includes(marker)) throw new Error(`Licensed Makira preparer is missing ${marker}.`);
}
if (!gitignore.split(/\r?\n/).includes("gui/src/fonts/Makira-Regular.ttf")) {
  throw new Error("The commercial Makira source font must remain excluded from the public repository.");
}

function* files(root) {
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = resolve(root, entry.name);
    if (entry.isDirectory()) yield* files(path);
    else if (entry.isFile()) yield path;
  }
}

if (packageRoot) {
  const forbidden = /(?:[A-Z]:[\\/]Users[\\/]|[\\/]Users[\\/][^\\/]+[\\/]|[\\/]home[\\/][^\\/]+[\\/]|AppData[\\/](?:Local|Roaming)|node_modules[\\/]@native-sdk[\\/]cli)/i;
  const leaks = [];
  for (const path of files(packageRoot)) {
    if (forbidden.test(readFileSync(path).toString("latin1"))) leaks.push(path);
  }
  if (leaks.length) throw new Error(`Developer-local paths leaked into packaged assets:\n${leaks.join("\n")}`);

  const notices = new Map([
    ["PRODUCT-LICENSE.md", resolve(repoRoot, "LICENSE.md")],
    ["IBM-PLEX-OFL-1.1.txt", resolve(guiRoot, "assets", "fonts", "IBM-PLEX-LICENSE.txt")],
    ["NATIVE-SDK-APACHE-2.0.txt", resolve(guiRoot, "node_modules", "@native-sdk", "cli", "LICENSE")],
  ]);
  const noticeRoot = resolve(packageRoot, "bin", "licenses");
  for (const [name, source] of notices) {
    const packaged = resolve(noticeRoot, name);
    if (!statSync(packaged, { throwIfNoEntry: false })?.isFile()) {
      throw new Error(`Packaged GUI is missing required notice: ${name}.`);
    }
    const digest = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
    if (digest(packaged) !== digest(source)) {
      throw new Error(`Packaged GUI notice does not match its reviewed source: ${name}.`);
    }
  }
}
process.stdout.write("Native SDK distribution pins and path-leak checks passed.\n");
