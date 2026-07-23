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
  patchHash: "346b26bb366718740b929b3a758ab653f5f53904fd70e6c3a8b270958aa28ced",
};

const files = new Map([
  ["build/app.zig", ["0b224d560c66f0a111c1cc333c3f81002ab25811dabb81d85d174a89ed491595", "26cf51a704eef3be95471b364b989808c10eed9905ffb4f993d689c7ae248e28"]],
  ["src/app_runner/root.zig", ["e085afe9f414a5ef0c21388e0bb1436bf05cb346349d6e87ca7e352c38b0c4e0", "5a3cbdbe53a4a68c93a49defb6024d20f335bda176697342a3c79163ce880340"]],
  ["src/platform/linux/gtk_host.c", ["da73fa340df0f577cc09873ae0c6d5e6d94bc7ca8024a68ad51d2df94cd93af7", "772f4e3d01366e5b31ad138cab1e6977bfc6e081c6a962cbb27336fc7bd2e14f"]],
  ["src/platform/macos/appkit_host.h", ["56e44b321b7011ef6bd2b85d97b2e9f6d1b502ceb3421032dfe15b53eb1b6d32", "6f5c6f6756667cf740d6181548141befd3a01970da4d09872de81d806cd90aa5"]],
  ["src/platform/macos/appkit_host.m", ["df07d1e67688b307752f5ff850108664d76e5c3f44335970d21fa0139f9ec7d5", "d49911a090c1d1c4383e04cbd80948e9f26ec0922078001705d82c1c4553040b"]],
  ["src/platform/macos/root.zig", ["980888b6c53acf2dac2bca908f878214dce8707a71c7c8f4365aa0ade821ffc6", "920fe57a917ba11f5bfef358458e50ec3a6ca00f92ee0a5e0672f31292830f23"]],
  ["src/platform/null_platform.zig", ["f4e4fa7f018ccc443f2477e72783f31d355d928149d0fd9687c02be7fc0babc5", "114153cf720649ef50f18bbb5de8175f31eb79e550d79cfaff7300926b32422f"]],
  ["src/platform/types.zig", ["213ee148a0206039a78a1d9260ad08cf7f972bcb8f24f95b8e6e0ba986bc33df", "14e69741e8572fc2e894b7409564901e5b7cf5cd0a738fca2bd83f3d023c53fe"]],
  ["src/platform/windows/root.zig", ["76b0c53e8f217ce177d1b4c4c5c7c3029deb26e3984706b1087785b1be404e30", "09b1119e6212d12ba366292d05fca8586c9a0ee9f6c3c1a906f3395526160721"]],
  ["src/platform/windows/webview2_host.cpp", ["93d9843a411de4364310bbd4f87be19381c085828152b1c975249064d0c6e8a3", "acb9d381ed51d307dff9aa1430e8e0e0a07f4e6c93f088185afdbce901a2ddc7"]],
  ["src/primitives/canvas/reference_memo.zig", ["ebb7d49035d993b11b30c784e362f9cb12ed625a5e6ce19a44059bb20b34d592", "ea69ec3d6f4024062f4ac8aad88b4482258c8c0f4328dae7dcc33b89621b8196"]],
  ["src/primitives/canvas/reference.zig", ["56ef9cec4f76ee6cbff8a56dc5f579d3b9ee2daa79ad4cdb1f40073c3a053ecb", "cf6068c5e7d2b9ffc4c8d28940be822348bf441646739e5b74238d433936fa8b"]],
  ["src/primitives/canvas/reference_tests.zig", ["3accd42966c9465b28859cd73a33684619926d18082a32f7c6faac8b0f3b326a", "55a3e981de470b10ff67821e978e476eecab6fd6f607cc3945b30a81a6014f60"]],
  ["src/runtime/bridge_permission_tests.zig", ["d048b23298d75c225476e2708c695c4bb4feca26c09131648d13112067cce9c1", "e083b02a70108f669077306efcd564bd6b3de37c1f2d76feb4da01015275d9c4"]],
  ["src/runtime/canvas_frame.zig", ["d2eb5ff8c63a391a695a2a47bfef6c315ddafa98e7b35cd91253437eb066a0ce", "2678ff7cfb3d47c765d517d5b9c8eb1746985cb6b610e75da3bfd02c24eb0639"]],
  ["src/runtime/canvas_frame_patch_tests.zig", ["c24d345ae4c26b073b84442bad64b2378ab7a4f424813df0b0a7d3ffcbb96d79", "c83a1327674e9e7b8b36accdaf8ed2e63ca140ec00a81c1f81347b375c6cf462"]],
  ["src/runtime/core.zig", ["47ec8939f1be3be8808360627f26a41135398a5cb52d36c8785414b9b195e193", "d897d14a9a59ef19dfa5ab720788b7af4bd322206afc09273533a19ed33f6e5b"]],
  ["src/runtime/system_services.zig", ["69ed09c968796645c03276f4fa6e7065350a63bf9926070c255d41ab8c09e46e", "0a519438bd416d01f9196e605d27d537fd1e9acd12443afa2cdd367954fd833c"]],
  ["src/runtime/ui_app.zig", ["eedba5eef9470959f75aa97574c1343466798a4e093476fff2fb5dd9ac465e26", "f1f5d5aef7eccd36af9ceec2a1e1df4d423a9975caa3742887806fa90804b301"]],
  ["src/runtime/ui_app_tests.zig", ["eaf4c33dfca9858e9070faedb3809be5ba330893bb71fad7c6fdc416b29570af", "7c27ef3b85fb927c4b7abe7e6ac93f95a73f7afeebb42aaf2fa00206009472d7"]],
  ["src/runtime/validation.zig", ["96790d675894fca8b1af1233ef81161433932d2ac00777f61809ff82a7bdef36", "41cf8fb540a20f1084551c93543f1f0d480d54d716a28145b3a9d021b5cb0a28"]],
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
