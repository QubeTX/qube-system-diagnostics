#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFileSync, statSync } from "node:fs";
import { dirname, relative, resolve, sep } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptRoot = dirname(fileURLToPath(import.meta.url));
const guiRoot = resolve(process.argv[2] ?? resolve(scriptRoot, "..", "gui"));
const repoRoot = resolve(guiRoot, "..");
const sdkRoot = resolve(guiRoot, "node_modules", "@native-sdk", "cli");
const patchPath = resolve(guiRoot, "patches", "native-sdk-0.5.4-software-render.patch");

const required = {
  version: "0.5.4",
  tarball: "https://registry.npmjs.org/@native-sdk/cli/-/cli-0.5.4.tgz",
  integrity: "sha512-8ixE8TjN2zQ+9rnnpjOnmHDeloyvKBc9CKXVUdYxge63fSKn6AH3rodRcdE6EYQiAIDYzQiJSr8AKT1qdFcABA==",
  zigHash: "native_sdk-0.1.0-hzDzQo8l5gCK6W8hPyRC4voBqyQU8bhy6ktUDXKIqWlb",
  patchHash: "32ca4fc9a6bf2d2a0933d5bf2b9db5d4b3c3736f27ffcbe4938f76998a350fc1",
};

const files = new Map([
  ["src/app_runner/root.zig", ["e085afe9f414a5ef0c21388e0bb1436bf05cb346349d6e87ca7e352c38b0c4e0", "5a3cbdbe53a4a68c93a49defb6024d20f335bda176697342a3c79163ce880340"]],
  ["src/platform/windows/webview2_host.cpp", ["93d9843a411de4364310bbd4f87be19381c085828152b1c975249064d0c6e8a3", "7410ad7d8e2f6ddd97d78614967473f67934c7eed7fb1202af0e7a21ef9cbe2a"]],
  ["src/primitives/canvas/reference_memo.zig", ["ebb7d49035d993b11b30c784e362f9cb12ed625a5e6ce19a44059bb20b34d592", "7a496d84accbbf780e5dbbdc2b12b6e3abc7977d4397f673dffcddf3be018c34"]],
  ["src/primitives/canvas/reference.zig", ["56ef9cec4f76ee6cbff8a56dc5f579d3b9ee2daa79ad4cdb1f40073c3a053ecb", "a4c297c8e04114213adb1c042bb70833b9e1c7cbdd39755fd9383b4bbe743cbe"]],
  ["src/primitives/canvas/reference_tests.zig", ["3accd42966c9465b28859cd73a33684619926d18082a32f7c6faac8b0f3b326a", "a0479ceb2f5f88ed15835fde5f6613d7409b8c4422e808301964f9dc1e2dee87"]],
  ["src/runtime/canvas_frame.zig", ["d2eb5ff8c63a391a695a2a47bfef6c315ddafa98e7b35cd91253437eb066a0ce", "69b6354af8e15ced6738b9dae78b503ba558278bc04daa8176c36317152f1392"]],
  ["src/runtime/canvas_frame_patch_tests.zig", ["c24d345ae4c26b073b84442bad64b2378ab7a4f424813df0b0a7d3ffcbb96d79", "c83a1327674e9e7b8b36accdaf8ed2e63ca140ec00a81c1f81347b375c6cf462"]],
]);

function fail(message) {
  throw new Error(message);
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function json(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

if (!statSync(sdkRoot, { throwIfNoEntry: false })?.isDirectory()) {
  fail(`Project-local Native SDK is missing under ${guiRoot}; run npm ci first.`);
}
if (!statSync(patchPath, { throwIfNoEntry: false })?.isFile()) {
  fail(`Reviewed Native SDK patch is missing: ${patchPath}`);
}
if (sha256(patchPath) !== required.patchHash) {
  fail("The Native SDK renderer patch does not match the reviewed toolchain record.");
}
const lock = json(resolve(guiRoot, "package-lock.json"));
const locked = lock.packages?.["node_modules/@native-sdk/cli"];
if (locked?.version !== required.version || locked?.resolved !== required.tarball || locked?.integrity !== required.integrity) {
  fail("package-lock.json does not resolve the reviewed Native SDK 0.5.4 bytes.");
}
if (json(resolve(sdkRoot, "package.json")).version !== required.version) {
  fail("The project-local Native SDK version does not match the reviewed 0.5.4 package.");
}

let state;
for (const [name, [pristine, patched]] of files) {
  const path = resolve(sdkRoot, ...name.split("/"));
  const hash = sha256(path);
  const fileState = hash === pristine ? "pristine" : hash === patched ? "patched" : undefined;
  if (!fileState) fail(`Native SDK source ${name} has unreviewed bytes (${hash}).`);
  if (state && state !== fileState) fail("Native SDK sources are in a mixed pristine/patched state; rerun npm ci.");
  state = fileState;
}

if (state === "pristine") {
  const sdkRelative = relative(repoRoot, sdkRoot).split(sep).join("/");
  if (sdkRelative.startsWith("../") || sdkRelative === "..") fail("Project-local Native SDK resolved outside the repository.");
  const common = ["-c", "core.autocrlf=false", "apply", `--directory=${sdkRelative}`, "--whitespace=nowarn"];
  const check = spawnSync("git", [...common, "--check", patchPath], { cwd: repoRoot, stdio: "inherit" });
  if (check.status !== 0) fail("The reviewed Native SDK renderer patch no longer applies cleanly.");
  const apply = spawnSync("git", [...common, patchPath], { cwd: repoRoot, stdio: "inherit" });
  if (apply.status !== 0) fail("Applying the reviewed Native SDK renderer patch failed.");
}

for (const [name, [, patched]] of files) {
  if (sha256(resolve(sdkRoot, ...name.split("/"))) !== patched) {
    fail(`Patched Native SDK verification failed for ${name}.`);
  }
}

process.stdout.write(`${JSON.stringify({
  schema: 1,
  native_sdk_cli: required.version,
  tarball: required.tarball,
  npm_integrity: required.integrity,
  zig_content_hash: required.zigHash,
  renderer_patch_sha256: required.patchHash,
  source_state: "patched",
})}\n`);
