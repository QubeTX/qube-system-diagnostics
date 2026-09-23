import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Cargo emits these scalar fields in a fixed format. This deliberately reads
// only package identities, not arbitrary TOML or dependency feature resolution.
function packages(text) {
  if (!/^version = [34]\s*$/m.test(text)) throw new Error("Unsupported Cargo.lock format");
  const blocks = text.split(/^\[\[package\]\]\s*$/m).slice(1);
  if (!blocks.length) throw new Error("Cargo.lock has no packages");
  return blocks.map(block => {
    const field = name => block.match(new RegExp(`^${name} = "([^"\\r\\n]+)"\\s*$`, "m"))?.[1];
    const record = Object.fromEntries(["name", "version", "source", "checksum"].map(n => [n, field(n)]));
    if (!record.name || !record.version) throw new Error("Malformed Cargo package identity");
    return record;
  });
}

export function compareSharedLocks(rootText, engineText) {
  const root = packages(rootText);
  const sharedNames = new Set(root.filter(p => p.source).map(p => p.name));
  const failures = [];
  for (const p of packages(engineText)) {
    if (!sharedNames.has(p.name)) continue;
    const same = root.find(r => r.name === p.name && r.version === p.version && r.source === p.source);
    if (!same || same.checksum !== p.checksum) failures.push(`${p.name} ${p.version}: engine differs from root lock`);
  }
  return failures;
}

export function checkSharedLocks(repoRoot) {
  const failures = compareSharedLocks(
    readFileSync(resolve(repoRoot, "Cargo.lock"), "utf8"),
    readFileSync(resolve(repoRoot, "gui-engine/Cargo.lock"), "utf8"),
  );
  if (failures.length) throw new Error(`Shared Rust dependency drift:\n${failures.join("\n")}\nReconcile both locks before qualifying the composite product.`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  checkSharedLocks(resolve(process.argv[2] ?? fileURLToPath(new URL("..", import.meta.url))));
  console.log("Shared Rust dependency versions, sources and checksums match");
}
