#!/usr/bin/env node

import { createHash } from "node:crypto";
import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { brotliDecompressSync } from "node:zlib";
import { fileURLToPath } from "node:url";

const scriptRoot = dirname(fileURLToPath(import.meta.url));
const guiRoot = resolve(process.argv[2] ?? resolve(scriptRoot, "..", "gui"));
const lock = JSON.parse(readFileSync(resolve(guiRoot, "toolchain-lock.json"), "utf8"));
const faces = [
  ["makira", "Makira-Regular.ttf", ["SD300_MAKIRA_FONT_BROTLI_BASE64_PART_1", "SD300_MAKIRA_FONT_BROTLI_BASE64_PART_2"]],
  ["gail_rock", "Gail-Rock-Regular.ttf", ["SD300_GAIL_ROCK_FONT_BROTLI_BASE64"]],
];
for (const [key, name, secrets] of faces) {
  const target = resolve(guiRoot, "src", "fonts", name);
  const expected = lock.fonts?.[key]?.sha256;
  if (!/^[0-9a-f]{64}$/.test(expected ?? "")) {
    throw new Error(`toolchain-lock.json is missing the reviewed ${name} SHA-256.`);
  }
  let bytes;
  if (existsSync(target)) {
    bytes = readFileSync(target);
  } else {
    const parts = secrets.map((name) => process.env[name] ?? "");
    if (parts.some((part) => !part)) {
      throw new Error(`${name} is absent. Supply ${secrets.join(" and ")} from the licensed build inputs.`);
    }
    bytes = brotliDecompressSync(Buffer.from(parts.join("").replace(/\s+/g, ""), "base64"), { maxOutputLength: 2 * 1024 * 1024 });
  }
  const actual = createHash("sha256").update(bytes).digest("hex");
  if (actual !== expected) throw new Error(`${name} build input SHA-256 mismatch: expected ${expected}, got ${actual}.`);
  if (!existsSync(target)) {
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, bytes, { mode: 0o600 });
  }
  try { chmodSync(target, 0o600); } catch { /* Windows permissions are inherited. */ }
  process.stdout.write(`Prepared licensed ${name} build input (${actual}).\n`);
}
