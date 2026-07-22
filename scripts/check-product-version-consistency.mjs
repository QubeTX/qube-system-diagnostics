#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const semverPattern = String.raw`[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?`;

function usage() {
  return [
    "Usage: node scripts/check-product-version-consistency.mjs [options]",
    "",
    "Options:",
    "  --expected-target <version>  Require the root product version to equal this release target.",
    "  --repo-root <path>           Check another source tree (primarily for tests/tooling).",
    "  --help                       Show this help.",
    "",
    "Without --expected-target, the [package] version in the root Cargo.toml is authoritative.",
  ].join("\n");
}

function parseArguments(argv) {
  const options = { expectedTarget: undefined, repoRoot: undefined };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--help" || argument === "-h") {
      process.stdout.write(`${usage()}\n`);
      process.exit(0);
    }
    if (argument === "--expected-target" || argument === "--repo-root") {
      const value = argv[index + 1];
      if (!value || value.startsWith("--")) {
        throw new Error(`${argument} requires a value.\n\n${usage()}`);
      }
      if (argument === "--expected-target") options.expectedTarget = value;
      else options.repoRoot = value;
      index += 1;
      continue;
    }
    if (argument.startsWith("--expected-target=")) {
      options.expectedTarget = argument.slice("--expected-target=".length);
      continue;
    }
    if (argument.startsWith("--repo-root=")) {
      options.repoRoot = argument.slice("--repo-root=".length);
      continue;
    }
    throw new Error(`Unknown argument: ${argument}\n\n${usage()}`);
  }
  return options;
}

function readSource(repoRoot, relativePath) {
  try {
    return readFileSync(resolve(repoRoot, relativePath), "utf8");
  } catch (error) {
    throw new Error(`Could not read ${relativePath}: ${error.message}`);
  }
}

function requiredMatch(text, expression, label) {
  const match = text.match(expression);
  if (!match) throw new Error(`Could not resolve ${label}.`);
  return match[1];
}

function packageManifestVersion(repoRoot, relativePath) {
  const text = readSource(repoRoot, relativePath);
  const packageSection = requiredMatch(
    text,
    /^\[package\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/m,
    `${relativePath} [package] section`,
  );
  return requiredMatch(
    packageSection,
    new RegExp(`^version\\s*=\\s*"(${semverPattern})"\\s*$`, "m"),
    `${relativePath} [package].version`,
  );
}

function cargoLockPackageVersion(repoRoot, relativePath, packageName) {
  const text = readSource(repoRoot, relativePath);
  const blocks = text.split(/^\[\[package\]\]\s*$/m).slice(1);
  const block = blocks.find((candidate) => {
    const name = candidate.match(/^name\s*=\s*"([^"]+)"\s*$/m)?.[1];
    return name === packageName;
  });
  if (!block) throw new Error(`Could not resolve package ${packageName} in ${relativePath}.`);
  return requiredMatch(
    block,
    new RegExp(`^version\\s*=\\s*"(${semverPattern})"\\s*$`, "m"),
    `${relativePath} package ${packageName} version`,
  );
}

function jsonVersion(repoRoot, relativePath, selector = (value) => value.version) {
  const value = JSON.parse(readSource(repoRoot, relativePath));
  const version = selector(value);
  if (typeof version !== "string" || !new RegExp(`^${semverPattern}$`).test(version)) {
    throw new Error(`Could not resolve a semantic version from ${relativePath}.`);
  }
  return version;
}

function zonVersion(repoRoot, relativePath) {
  return requiredMatch(
    readSource(repoRoot, relativePath),
    new RegExp(`^\\s*\\.version\\s*=\\s*"(${semverPattern})",?\\s*$`, "m"),
    `${relativePath} .version`,
  );
}

function zigExpectedProductVersion(repoRoot) {
  return requiredMatch(
    readSource(repoRoot, "gui/src/engine.zig"),
    new RegExp(`^pub const expected_product_version\\s*=\\s*"(${semverPattern})";\\s*$`, "m"),
    "gui/src/engine.zig expected_product_version",
  );
}

function engineMetadataTestVersion(repoRoot) {
  return requiredMatch(
    readSource(repoRoot, "gui-engine/src/lib.rs"),
    new RegExp(
      `assert_eq!\\(metadata\\["product_version"\\],\\s*"(${semverPattern})"\\);`,
    ),
    "gui-engine/src/lib.rs metadata product-version assertion",
  );
}

function nativeMarkupVersions(repoRoot) {
  const text = readSource(repoRoot, "gui/src/app.native");
  // Visible version labels bind to {productVersionLabel}, which derives from
  // the checked gui/src/engine.zig expected_product_version at comptime, so
  // zero literals is the healthy state. Any literal that does appear must
  // still match the release.
  return [...text.matchAll(new RegExp(`\\bv(${semverPattern})\\b`, "g"))].map(
    (match) => match[1],
  );
}

function stagedZonVersion(repoRoot, relativePath) {
  const text = readSource(repoRoot, relativePath);
  const stageBlock = requiredMatch(
    text,
    /(?:\$stageZon\s*=\s*@'|cat\s+>\s+"\$app_stage\/build\.zig\.zon"\s+<<'ZON')([\s\S]*?)(?:'@|\nZON)/,
    `${relativePath} staged build.zig.zon template`,
  );
  return requiredMatch(
    stageBlock,
    new RegExp(`^\\s*\\.version\\s*=\\s*"(${semverPattern})",?\\s*$`, "m"),
    `${relativePath} staged build.zig.zon .version`,
  );
}

function linuxPackageDefaultVersion(repoRoot) {
  return requiredMatch(
    readSource(repoRoot, "scripts/package-native-gui-linux.sh"),
    new RegExp(`^version=\\$\\{3:-(${semverPattern})\\}\\s*$`, "m"),
    "scripts/package-native-gui-linux.sh default product version",
  );
}

function collectVersionSurfaces(repoRoot) {
  const rootVersion = packageManifestVersion(repoRoot, "Cargo.toml");
  const guiEngineVersion = packageManifestVersion(repoRoot, "gui-engine/Cargo.toml");
  const guiPackageVersion = jsonVersion(repoRoot, "gui/package.json");
  const markupVersions = nativeMarkupVersions(repoRoot);

  return {
    rootVersion,
    surfaces: [
      ["Cargo.toml [package]", rootVersion],
      ["Cargo.lock tr300-tui package", cargoLockPackageVersion(repoRoot, "Cargo.lock", "tr300-tui")],
      ["gui-engine/Cargo.toml [package]", guiEngineVersion],
      [
        "gui-engine/Cargo.lock sd300-engine package",
        cargoLockPackageVersion(repoRoot, "gui-engine/Cargo.lock", "sd300-engine"),
      ],
      [
        "gui-engine/Cargo.lock tr300-tui path dependency",
        cargoLockPackageVersion(repoRoot, "gui-engine/Cargo.lock", "tr300-tui"),
      ],
      ["gui/package.json", guiPackageVersion],
      ["gui/package-lock.json top-level", jsonVersion(repoRoot, "gui/package-lock.json")],
      [
        "gui/package-lock.json root package",
        jsonVersion(repoRoot, "gui/package-lock.json", (value) => value.packages?.[""]?.version),
      ],
      ["gui/build.zig.zon", zonVersion(repoRoot, "gui/build.zig.zon")],
      ["gui/app.zon", zonVersion(repoRoot, "gui/app.zon")],
      ["gui/platform/linux/app.zon", zonVersion(repoRoot, "gui/platform/linux/app.zon")],
      ["gui-engine/src/lib.rs metadata test", engineMetadataTestVersion(repoRoot)],
      ["gui/src/engine.zig expected_product_version", zigExpectedProductVersion(repoRoot)],
      ...markupVersions.map((version, index) => [
        `gui/src/app.native visible version ${index + 1}`,
        version,
      ]),
      [
        "scripts/build-native-gui.ps1 staged build.zig.zon",
        stagedZonVersion(repoRoot, "scripts/build-native-gui.ps1"),
      ],
      [
        "scripts/build-native-gui.sh staged build.zig.zon",
        stagedZonVersion(repoRoot, "scripts/build-native-gui.sh"),
      ],
      ["scripts/package-native-gui-linux.sh default", linuxPackageDefaultVersion(repoRoot)],
    ],
  };
}

function main() {
  const options = parseArguments(process.argv.slice(2));
  if (
    options.expectedTarget &&
    !new RegExp(`^${semverPattern}$`).test(options.expectedTarget)
  ) {
    throw new Error(`--expected-target is not a semantic version: ${options.expectedTarget}`);
  }
  const scriptRoot = dirname(fileURLToPath(import.meta.url));
  const repoRoot = resolve(options.repoRoot ?? resolve(scriptRoot, ".."));
  const { rootVersion, surfaces } = collectVersionSurfaces(repoRoot);
  const failures = [];

  if (options.expectedTarget && options.expectedTarget !== rootVersion) {
    failures.push([
      "--expected-target",
      options.expectedTarget,
      `authoritative Cargo.toml is ${rootVersion}`,
    ]);
  }

  for (const [label, version] of surfaces) {
    if (version !== rootVersion) failures.push([label, version, `expected ${rootVersion}`]);
  }

  process.stdout.write(`SD-300 authoritative product version: ${rootVersion}\n`);
  if (options.expectedTarget) {
    process.stdout.write(`Requested release target: ${options.expectedTarget}\n`);
  }

  if (failures.length > 0) {
    process.stderr.write("Product version consistency check failed:\n");
    for (const [label, actual, expectation] of failures) {
      process.stderr.write(`  - ${label}: ${actual} (${expectation})\n`);
    }
    process.stderr.write(
      "Coordinate one source version bump across every listed surface before release qualification.\n",
    );
    process.exitCode = 1;
    return;
  }

  process.stdout.write(`All ${surfaces.length} product version surfaces match ${rootVersion}.\n`);
}

try {
  main();
} catch (error) {
  process.stderr.write(`Product version consistency check could not run: ${error.message}\n`);
  process.exitCode = 2;
}
