#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const [rootArgument, target, version, originsArgument] = process.argv.slice(2);
if (!rootArgument || !target || !version || !originsArgument) {
  console.error(
    "usage: generate-linux-runtime-sbom.mjs <package-root> <target> <version> <origins.tsv>",
  );
  process.exit(64);
}

const root = fs.realpathSync(rootArgument);
const originsPath = fs.realpathSync(originsArgument);
const licenseRoot = path.join(root, "share", "licenses", "runtime");
fs.mkdirSync(licenseRoot, { recursive: true, mode: 0o755 });

function run(command, args) {
  return execFileSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function safeSegment(value) {
  return value.replace(/[^A-Za-z0-9._+-]/g, "_");
}

function copyTree(source, destination) {
  if (!fs.existsSync(source)) return false;
  fs.mkdirSync(path.dirname(destination), { recursive: true, mode: 0o755 });
  fs.cpSync(source, destination, {
    recursive: true,
    dereference: true,
    force: true,
  });
  return true;
}

function parseOrigins() {
  const records = [];
  for (const line of fs.readFileSync(originsPath, "utf8").split(/\r?\n/)) {
    if (!line) continue;
    const separator = line.indexOf("\t");
    if (separator < 1) throw new Error(`invalid runtime origin record: ${line}`);
    const destination = line.slice(0, separator);
    const source = line.slice(separator + 1);
    if (
      !destination.startsWith("lib/runtime/") ||
      destination.includes("..") ||
      path.isAbsolute(destination) ||
      !path.isAbsolute(source)
    ) {
      throw new Error(`unsafe runtime origin record: ${line}`);
    }
    const packaged = path.resolve(root, destination);
    if (!packaged.startsWith(`${root}${path.sep}`) || !fs.statSync(packaged).isFile()) {
      throw new Error(`runtime origin destination is not a packaged file: ${destination}`);
    }
    records.push({ destination: destination.replaceAll("\\", "/"), source });
  }
  if (records.length === 0) throw new Error("runtime origin inventory is empty");
  return records;
}

function parseApkDatabase() {
  const database = fs.readFileSync("/lib/apk/db/installed", "utf8");
  const packages = new Map();
  for (const paragraph of database.split(/\n\n+/)) {
    const fields = new Map();
    for (const line of paragraph.split("\n")) {
      if (line.length >= 3 && line[1] === ":") fields.set(line[0], line.slice(2));
    }
    const name = fields.get("P");
    const packageVersion = fields.get("V");
    if (!name || !packageVersion) continue;
    packages.set(`${name}-${packageVersion}`, {
      name,
      version: packageVersion,
      architecture: fields.get("A") || "NOASSERTION",
      homepage: fields.get("U") || "NOASSERTION",
      distributorLicense: fields.get("L") || "NOASSERTION",
      origin: fields.get("o") || name,
      supplier: "Organization: Alpine Linux",
      manager: "apk",
      installedMetadata: `${paragraph.trim()}\n`,
    });
  }
  return packages;
}

function resolvePackages(origins) {
  const isDebian = fs.existsSync("/usr/bin/dpkg-query");
  const isAlpine = fs.existsSync("/sbin/apk") || fs.existsSync("/usr/sbin/apk");
  if (isDebian === isAlpine) {
    throw new Error("expected exactly one supported Linux package manager (dpkg or apk)");
  }

  const packages = new Map();
  const apkPackages = isAlpine ? parseApkDatabase() : null;
  if (isDebian) {
    const common = "/usr/share/common-licenses";
    if (!copyTree(common, path.join(licenseRoot, "debian-common"))) {
      throw new Error(`Debian common-license directory is missing: ${common}`);
    }
  }

  for (const origin of origins) {
    let metadata;
    let key;
    if (isDebian) {
      const ownerLine = run("dpkg-query", ["-S", origin.source])
        .split("\n")
        .find((line) => line.endsWith(`: ${origin.source}`));
      if (!ownerLine) throw new Error(`no Debian package owns ${origin.source}`);
      const owner = ownerLine.slice(0, ownerLine.length - origin.source.length - 2);
      const fields = run("dpkg-query", [
        "-W",
        "-f=${binary:Package}\t${Version}\t${Architecture}\t${Homepage}",
        owner,
      ]).split("\t");
      // `run()` trims command output. Debian permits Homepage to be absent,
      // which removes the trailing tab and yields three fields rather than
      // four. Package identity, version, and architecture remain mandatory;
      // an absent optional Homepage is recorded honestly as NOASSERTION.
      if (
        fields.length < 3 ||
        fields.length > 4 ||
        fields.slice(0, 3).some((field) => !field)
      ) {
        throw new Error(`incomplete Debian metadata for ${owner}`);
      }
      const [name, packageVersion, architecture, homepage = ""] = fields;
      key = `deb:${name}=${packageVersion}:${architecture}`;
      metadata = {
        name,
        version: packageVersion,
        architecture,
        homepage: homepage || "NOASSERTION",
        distributorLicense: "See bundled Debian copyright file",
        supplier: "Organization: Ubuntu",
        manager: "dpkg",
      };
      const docName = name.replace(/:.*/, "");
      const copyright = path.join("/usr/share/doc", docName, "copyright");
      const destination = path.join(
        licenseRoot,
        "debian-packages",
        safeSegment(name),
      );
      if (!copyTree(copyright, path.join(destination, "copyright"))) {
        throw new Error(`Debian package ${name} has no installed copyright file at ${copyright}`);
      }
      metadata.licenseEvidence = path.relative(root, destination).replaceAll("\\", "/");
      metadata.commonLicenseEvidence = path
        .relative(root, path.join(licenseRoot, "debian-common"))
        .replaceAll("\\", "/");
    } else {
      const ownerLine = run("apk", ["info", "--who-owns", origin.source]);
      const match = ownerLine.match(/ is owned by (\S+)$/) || ownerLine.match(/^(\S+) owns /);
      if (!match) throw new Error(`no Alpine package owns ${origin.source}: ${ownerLine}`);
      const owner = match[1];
      const found = apkPackages.get(owner);
      if (!found) throw new Error(`Alpine installed-package metadata is missing for ${owner}`);
      metadata = { ...found };
      key = `apk:${metadata.name}=${metadata.version}:${metadata.architecture}`;
    }

    if (!packages.has(key)) packages.set(key, { ...metadata, files: [] });
    packages.get(key).files.push(origin.destination);
  }

  if (isAlpine) {
    const spdxTextRoot = "/usr/share/spdx/text";
    if (!fs.existsSync(spdxTextRoot)) {
      throw new Error(
        "Alpine SPDX license texts are missing; install the pinned spdx-licenses-text package",
      );
    }
    for (const runtimePackage of packages.values()) {
      const destination = path.join(
        licenseRoot,
        "alpine-packages",
        safeSegment(runtimePackage.name),
      );
      fs.mkdirSync(destination, { recursive: true, mode: 0o755 });
      fs.writeFileSync(
        path.join(destination, "APK-METADATA.txt"),
        runtimePackage.installedMetadata,
        { mode: 0o644 },
      );
      const licenseIds = [
        ...new Set(
          runtimePackage.distributorLicense
            .replace(/[()]/g, " ")
            .split(/\s+/)
            .filter((token) => token && !["AND", "OR", "WITH"].includes(token)),
        ),
      ].sort();
      if (licenseIds.length === 0 || licenseIds.includes("NOASSERTION")) {
        throw new Error(
          `Alpine package ${runtimePackage.name} has no auditable license expression`,
        );
      }
      for (const licenseId of licenseIds) {
        const source = path.join(spdxTextRoot, `${licenseId}.txt`);
        if (!fs.existsSync(source) || !fs.statSync(source).isFile()) {
          throw new Error(
            `Alpine package ${runtimePackage.name} references unsupported license identifier ${licenseId}`,
          );
        }
        copyTree(source, path.join(destination, `${safeSegment(licenseId)}.txt`));
      }
      runtimePackage.licenseEvidence = path.relative(root, destination).replaceAll("\\", "/");
    }
  }
  return packages;
}

function walkFiles(directory, files = []) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) walkFiles(absolute, files);
    else if (entry.isFile()) files.push(absolute);
    else throw new Error(`package contains a non-regular filesystem entry: ${absolute}`);
  }
  return files;
}

const origins = parseOrigins();
const runtimePackages = resolvePackages(origins);
const spdxPath = path.join(root, "runtime-components.spdx.json");
const allFiles = walkFiles(root)
  .filter((file) => file !== spdxPath)
  .sort((left, right) => left.localeCompare(right, "en"));
const files = allFiles.map((absolute, index) => ({
  SPDXID: `SPDXRef-File-${index + 1}`,
  fileName: path.relative(root, absolute).replaceAll("\\", "/"),
  checksums: [
    {
      algorithm: "SHA1",
      checksumValue: crypto.createHash("sha1").update(fs.readFileSync(absolute)).digest("hex"),
    },
    {
      algorithm: "SHA256",
      checksumValue: crypto.createHash("sha256").update(fs.readFileSync(absolute)).digest("hex"),
    },
  ],
  licenseConcluded: "NOASSERTION",
  copyrightText: "NOASSERTION",
}));
const fileIds = new Map(files.map((file) => [file.fileName, file.SPDXID]));
const fileSha1 = new Map(
  files.map((file) => [
    file.fileName,
    file.checksums.find((checksum) => checksum.algorithm === "SHA1").checksumValue,
  ]),
);
function packageVerificationCode(fileNames, excludedFiles = []) {
  const checksums = [...new Set(fileNames)].map((fileName) => {
    const checksum = fileSha1.get(fileName);
    if (!checksum) throw new Error(`package verification references an absent file: ${fileName}`);
    return checksum;
  });
  if (checksums.length === 0) throw new Error("cannot verify an empty SPDX package");
  checksums.sort();
  const result = {
    packageVerificationCodeValue: crypto
      .createHash("sha1")
      .update(checksums.join(""))
      .digest("hex"),
  };
  if (excludedFiles.length > 0) {
    result.packageVerificationCodeExcludedFiles = [...excludedFiles].sort();
  }
  return result;
}
const productId = "SPDXRef-Package-SD300-Linux-Runtime";
const packages = [
  {
    SPDXID: productId,
    name: "SD-300 Linux private runtime",
    versionInfo: version,
    downloadLocation: "NOASSERTION",
    filesAnalyzed: true,
    packageVerificationCode: packageVerificationCode(
      files.map((file) => file.fileName),
      ["install-manifest.json", "runtime-components.spdx.json"],
    ),
    licenseConcluded: "NOASSERTION",
    licenseDeclared: "NOASSERTION",
    copyrightText: "NOASSERTION",
    primaryPackagePurpose: "APPLICATION",
  },
];
const relationships = [
  {
    spdxElementId: "SPDXRef-DOCUMENT",
    relationshipType: "DESCRIBES",
    relatedSpdxElement: productId,
  },
];
for (const file of files) {
  relationships.push({
    spdxElementId: productId,
    relationshipType: "CONTAINS",
    relatedSpdxElement: file.SPDXID,
  });
}
let packageIndex = 0;
for (const [key, runtimePackage] of [...runtimePackages.entries()].sort(([left], [right]) =>
  left.localeCompare(right, "en"),
)) {
  packageIndex += 1;
  const packageId = `SPDXRef-RuntimePackage-${packageIndex}`;
  const packageFiles = [...new Set(runtimePackage.files)].sort();
  packages.push({
    SPDXID: packageId,
    name: runtimePackage.name,
    versionInfo: runtimePackage.version,
    supplier: runtimePackage.supplier,
    downloadLocation: "NOASSERTION",
    homepage: runtimePackage.homepage,
    filesAnalyzed: true,
    packageVerificationCode: packageVerificationCode(packageFiles),
    licenseConcluded: "NOASSERTION",
    licenseDeclared: "NOASSERTION",
    licenseComments:
      `${runtimePackage.manager} metadata reports ${runtimePackage.distributorLicense}; ` +
      `license evidence is bundled at ${runtimePackage.licenseEvidence}.` +
      (runtimePackage.commonLicenseEvidence
        ? ` Shared Debian license texts are bundled at ${runtimePackage.commonLicenseEvidence}.`
        : ""),
    copyrightText: "NOASSERTION",
    attributionTexts: [`Bundled license evidence: ${runtimePackage.licenseEvidence}`],
    externalRefs: [
      {
        referenceCategory: "PACKAGE-MANAGER",
        referenceType: "purl",
        referenceLocator:
          `pkg:generic/${encodeURIComponent(runtimePackage.name)}@${encodeURIComponent(runtimePackage.version)}` +
          `?arch=${encodeURIComponent(runtimePackage.architecture)}&distro=${runtimePackage.manager}`,
      },
    ],
  });
  relationships.push({
    spdxElementId: productId,
    relationshipType: "CONTAINS",
    relatedSpdxElement: packageId,
  });
  for (const fileName of packageFiles) {
    const fileId = fileIds.get(fileName);
    if (!fileId) throw new Error(`SPDX runtime package maps an absent file: ${fileName}`);
    relationships.push({
      spdxElementId: packageId,
      relationshipType: "CONTAINS",
      relatedSpdxElement: fileId,
    });
  }
}

const sourceDateEpoch = process.env.SOURCE_DATE_EPOCH;
if (!sourceDateEpoch || !/^\d+$/.test(sourceDateEpoch)) {
  throw new Error("SOURCE_DATE_EPOCH must be the checked-out commit timestamp");
}
const namespaceDigest = crypto.createHash("sha256");
namespaceDigest.update(`${target}\0${version}\0`);
for (const file of files) {
  namespaceDigest.update(`${file.fileName}\0${file.checksums[1].checksumValue}\0`);
}
for (const [key, runtimePackage] of [...runtimePackages.entries()].sort(([left], [right]) =>
  left.localeCompare(right, "en"),
)) {
  namespaceDigest.update(
    `${key}\0${runtimePackage.distributorLicense}\0${runtimePackage.homepage}\0`,
  );
}
const document = {
  spdxVersion: "SPDX-2.3",
  dataLicense: "CC0-1.0",
  SPDXID: "SPDXRef-DOCUMENT",
  name: `SD-300-${target}-${version}`,
  documentNamespace:
    `https://github.com/QubeTX/qube-system-diagnostics/spdx/${version}/${target}/` +
    namespaceDigest.digest("hex"),
  creationInfo: {
    created: new Date(Number(sourceDateEpoch) * 1000).toISOString().replace(".000Z", "Z"),
    creators: [
      `Tool: sd300-generate-linux-runtime-sbom-1`,
      `Organization: QubeTX`,
    ],
  },
  packages,
  files,
  relationships,
};
fs.writeFileSync(spdxPath, `${JSON.stringify(document, null, 2)}\n`);
