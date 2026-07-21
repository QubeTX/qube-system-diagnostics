#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { relative, resolve, sep } from "node:path";

const [rootArg, target, entrypoint, engine, version] = process.argv.slice(2);
if (!rootArg || !target || !entrypoint || !engine || !version) {
  throw new Error("usage: write-gui-install-manifest.mjs <root> <target> <entrypoint> <engine> <version>");
}
const root = resolve(rootArg);
const manifestPath = resolve(root, "install-manifest.json");

function* files(path) {
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const child = resolve(path, entry.name);
    if (entry.isDirectory()) yield* files(child);
    else if (entry.isFile() && child !== manifestPath) yield child;
  }
}

const records = [...files(root)].map((path) => {
  const bytes = readFileSync(path);
  return {
    path: relative(root, path).split(sep).join("/"),
    size: statSync(path).size,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}).sort((left, right) => left.path.localeCompare(right.path));

writeFileSync(manifestPath, `${JSON.stringify({
  schema: 1,
  product: "SD-300",
  version,
  target,
  entrypoint,
  engine,
  files: records,
}, null, 2)}\n`, { encoding: "utf8", mode: 0o644 });
