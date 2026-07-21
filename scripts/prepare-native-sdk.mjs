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
  patchHash: "b7af0bc02554c0c29aa334e1aa4fb8cecfd8ecf867531d082933d91b05024f4e",
};

const files = new Map([
  ["build/app.zig", ["0b224d560c66f0a111c1cc333c3f81002ab25811dabb81d85d174a89ed491595", "675bd4cd6552a4084eaf57856b2f681b02424eaa783a50e05a6bf7722dc2eb2c"]],
  ["src/app_runner/root.zig", ["e085afe9f414a5ef0c21388e0bb1436bf05cb346349d6e87ca7e352c38b0c4e0", "5a3cbdbe53a4a68c93a49defb6024d20f335bda176697342a3c79163ce880340"]],
  ["src/platform/linux/gtk_host.c", ["da73fa340df0f577cc09873ae0c6d5e6d94bc7ca8024a68ad51d2df94cd93af7", "772f4e3d01366e5b31ad138cab1e6977bfc6e081c6a962cbb27336fc7bd2e14f"]],
  ["src/platform/windows/webview2_host.cpp", ["93d9843a411de4364310bbd4f87be19381c085828152b1c975249064d0c6e8a3", "ec274d378638c99fd2e823a77725a3b14487fa7ddd72834f55f01ccb247239e1"]],
  ["src/primitives/canvas/reference_memo.zig", ["ebb7d49035d993b11b30c784e362f9cb12ed625a5e6ce19a44059bb20b34d592", "ea69ec3d6f4024062f4ac8aad88b4482258c8c0f4328dae7dcc33b89621b8196"]],
  ["src/primitives/canvas/reference.zig", ["56ef9cec4f76ee6cbff8a56dc5f579d3b9ee2daa79ad4cdb1f40073c3a053ecb", "cf6068c5e7d2b9ffc4c8d28940be822348bf441646739e5b74238d433936fa8b"]],
  ["src/primitives/canvas/reference_tests.zig", ["3accd42966c9465b28859cd73a33684619926d18082a32f7c6faac8b0f3b326a", "55a3e981de470b10ff67821e978e476eecab6fd6f607cc3945b30a81a6014f60"]],
  ["src/runtime/canvas_frame.zig", ["d2eb5ff8c63a391a695a2a47bfef6c315ddafa98e7b35cd91253437eb066a0ce", "2678ff7cfb3d47c765d517d5b9c8eb1746985cb6b610e75da3bfd02c24eb0639"]],
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
