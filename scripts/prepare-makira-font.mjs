#!/usr/bin/env node

import { createHash } from "node:crypto";
import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { brotliDecompressSync } from "node:zlib";
import { fileURLToPath } from "node:url";

const scriptRoot = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptRoot, "..");
const guiRoot = resolve(process.argv[2] ?? resolve(repoRoot, "gui"));
const lock = JSON.parse(readFileSync(resolve(guiRoot, "toolchain-lock.json"), "utf8"));
const target = resolve(guiRoot, "src", "fonts", "Makira-Regular.ttf");
const expected = lock.fonts?.makira?.sha256;

if (!/^[0-9a-f]{64}$/.test(expected ?? "")) {
  throw new Error("gui/toolchain-lock.json is missing the reviewed Makira SHA-256.");
}

const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
let bytes;

if (existsSync(target)) {
  bytes = readFileSync(target);
} else {
  const part1 = process.env.SD300_MAKIRA_FONT_BROTLI_BASE64_PART_1 ?? "";
  const part2 = process.env.SD300_MAKIRA_FONT_BROTLI_BASE64_PART_2 ?? "";
  if (!part1 || !part2) {
    throw new Error(
      "Makira-Regular.ttf is absent. Supply both SD300_MAKIRA_FONT_BROTLI_BASE64_PART_1 and " +
      "SD300_MAKIRA_FONT_BROTLI_BASE64_PART_2 from the licensed build secret.",
    );
  }
  const encoded = `${part1}${part2}`.replace(/\s+/g, "");
  bytes = brotliDecompressSync(Buffer.from(encoded, "base64"));
}

const actual = digest(bytes);
if (actual !== expected) {
  throw new Error(`Makira build input SHA-256 mismatch: expected ${expected}, got ${actual}.`);
}
if (!existsSync(target)) {
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, bytes, { mode: 0o600 });
}
try {
  chmodSync(target, 0o600);
} catch {
  // Windows does not expose POSIX file modes. The repository secret remains
  // protected by Actions and the file is ignored by Git on every host.
}

process.stdout.write(`Prepared licensed Makira build input (${actual}).\n`);
