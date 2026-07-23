#!/usr/bin/env node

import { cpSync, mkdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, relative, resolve, sep } from "node:path";
import { arch, platform } from "node:os";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptRoot = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptRoot, "..");
const guiRoot = resolve(repoRoot, "gui");
const sdkRoot = resolve(guiRoot, "node_modules", "@native-sdk", "cli");
const stageBase = resolve(repoRoot, "target", "native-gui-test-stage");
const stageRoot = resolve(stageBase, `${platform()}-${arch()}`);
const appStage = resolve(stageRoot, "app");
const sdkStage = resolve(stageRoot, "sdk");

function fail(message) {
  throw new Error(message);
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, env: process.env, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function copyRequired(source, destination) {
  if (!statSync(source, { throwIfNoEntry: false })) fail(`Required native GUI test input is missing: ${source}`);
  cpSync(source, destination, { recursive: true, force: true });
}

// The checked-in build.zig.zon intentionally retains the immutable npm URL
// and Zig content hash. SD-300 carries a reviewed downstream renderer patch,
// so app tests must use the same generated, repository-relative dependency
// graph as release builds instead of mutating Zig's content-addressed cache.
run(process.execPath, [resolve(scriptRoot, "prepare-makira-font.mjs"), guiRoot], repoRoot);
run(process.execPath, [resolve(scriptRoot, "prepare-native-sdk.mjs"), guiRoot], repoRoot);

const stageRelative = relative(stageBase, stageRoot);
if (!stageRelative || stageRelative === ".." || stageRelative.startsWith(`..${sep}`) || resolve(stageBase, stageRelative) !== stageRoot) {
  fail(`Refusing to prepare a native GUI test stage outside ${stageBase}: ${stageRoot}`);
}
rmSync(stageRoot, { recursive: true, force: true });
mkdirSync(appStage, { recursive: true });

for (const name of ["build.zig", "app.zon", "README.md"]) {
  copyRequired(resolve(guiRoot, name), resolve(appStage, name));
}
for (const name of ["src", "assets", "platform", "tools"]) {
  copyRequired(resolve(guiRoot, name), resolve(appStage, name));
}
copyRequired(sdkRoot, sdkStage);

// The test step does not install the engine, but keeping any already-built
// host engine beside the staged app also exercises the production layout.
for (const name of ["sd300_engine.dll", "libsd300_engine.dylib", "libsd300_engine.so"]) {
  const source = resolve(guiRoot, name);
  if (statSync(source, { throwIfNoEntry: false })?.isFile()) cpSync(source, resolve(appStage, name), { force: true });
}

writeFileSync(resolve(appStage, "build.zig.zon"), `.{
    .name = .gui,
    .fingerprint = 0xd4ff50f85a707070,
    .version = "3.0.0",
    .minimum_zig_version = "0.16.0",
    .dependencies = .{
        .native_sdk = .{ .path = "../sdk" },
    },
    .paths = .{ "build.zig", "build.zig.zon", "src", "assets", "platform", "tools", "app.zon", "README.md" },
}
`, "utf8");

// Invoke the project-local CLI dispatcher directly. This is the same
// `native test` implementation exposed by npx, without Windows cmd.exe
// quoting or any chance of resolving a globally installed package.
run(process.execPath, [resolve(sdkRoot, "bin", "native.js"), "test", appStage, "--yes", ...process.argv.slice(2)], guiRoot);
