import { existsSync, lstatSync, readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { allUnique, sha256, requireSame } from "./io.mjs";

const excludedDirectories = new Set([
  "target", "node_modules", "artifacts", "cache", "typechain-types", "generated",
  ".git", ".pnpm", ".pnpm-store", ".next", "dist", "build", "coverage", "out",
  "__pycache__", ".cache", "venv", ".venv", ".turbo", "tmp", "temp",
]);
const allowedExtensions = new Set([
  "rs", "toml", "lock", "fhe", "sol", "ts", "tsx", "js", "mjs", "cjs",
  "json", "md", "ps1", "yaml", "yml", "sh", "html", "css", "txt",
]);
const selectedRoots = Object.freeze([
  "crates", "examples", "docs", "tools/cofhe", "integration/cofhe-hardhat/contracts",
  "integration/cofhe-hardhat/phase4", "integration/cofhe-hardhat/phase6",
  "integration/cofhe-hardhat/phase7", "integration/cofhe-hardhat/phase89",
  "integration/cofhe-hardhat/phase10", ".github/workflows",
]);
const selectedFiles = Object.freeze([
  "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "README.md", "LICENSE", "integration/cofhe-hardhat/package.json",
  "integration/cofhe-hardhat/pnpm-lock.yaml", "integration/cofhe-hardhat/hardhat.config.ts",
  "integration/cofhe-hardhat/tsconfig.json",
]);
const secretName = /(?:^|[\/_-])(?:\.env|private[-_]key|secret|secrets|mnemonic|wallet|keystore|credential|credentials|password)(?:[.\/_-]|$)/i;
const maxFileSize = 4 * 1024 * 1024;
const maxInventoryBytes = 64 * 1024 * 1024;

function allowedFile(name) {
  const basename = name.split("/").at(-1);
  const extension = basename.slice(basename.lastIndexOf(".") + 1).toLowerCase();
  return (basename === "LICENSE" || allowedExtensions.has(extension)) && !basename.startsWith(".");
}

function filesIn(root, relativeRoot, out) {
  const directory = join(root, ...relativeRoot.split("/"));
  if (!existsSync(directory)) return;
  const stat = lstatSync(directory);
  if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error("Included source directory is not a regular directory: " + relativeRoot);
  for (const child of readdirSync(directory, { withFileTypes: true })) {
    if (excludedDirectories.has(child.name)) continue;
    const relativePath = relativeRoot + "/" + child.name;
    const filePath = join(root, ...relativePath.split("/"));
    const entryStat = lstatSync(filePath);
    if (entryStat.isSymbolicLink()) throw new Error("Symlink inside selected source tree: " + relativePath);
    if (entryStat.isDirectory()) {
      filesIn(root, relativePath, out);
    } else if (entryStat.isFile() && allowedFile(relativePath)) {
      out.push(relativePath);
    } else if (!entryStat.isFile()) {
      throw new Error("Unexpected special source-tree filesystem entry: " + relativePath);
    }
  }
}

export function sourceInventory(projectRoot) {
  const root = resolve(projectRoot);
  for (const file of ["Cargo.toml", "Cargo.lock", "integration/cofhe-hardhat/pnpm-lock.yaml"]) {
    if (!existsSync(join(root, ...file.split("/")))) throw new Error("Missing required FHEC V2 source anchor: " + file);
  }
  const names = selectedFiles.filter((name) => existsSync(join(root, ...name.split("/"))));
  for (const path of selectedRoots) filesIn(root, path, names);
  names.sort();
  if (!allUnique(names.map((name) => name.toLowerCase())) || names.length < 12 || names.length > 10000) {
    throw new Error("Source inventory is incomplete, unexpectedly large or case-colliding");
  }
  let bytes = 0;
  const files = names.map((name) => {
    if (secretName.test(name)) throw new Error("Credential-like path requires manual exclusion/review before release inventory: " + name);
    const full = join(root, ...name.split("/"));
    const stat = lstatSync(full);
    if (!stat.isFile() || stat.isSymbolicLink() || stat.size > maxFileSize) {
      throw new Error("Source file is not a regular bounded file: " + name);
    }
    bytes += stat.size;
    if (bytes > maxInventoryBytes) throw new Error("Source files exceed bounded release inventory capacity");
    return { file: name, bytes: stat.size, sha256: sha256(readFileSync(full)) };
  });
  return {
    schema: "fhec-cofhe-phase12-source-inventory-v1",
    scope: "selected reviewable FHEC V2 source paths; this manifest is not a cryptographic signature",
    excludes: "build outputs, generated Solidity, dependency installs, credentials, testnet keys and arbitrary external files",
    source_files: files.length,
    source_bytes: bytes,
    tree_sha256: sha256(JSON.stringify(files)),
    files,
  };
}

export function verifySourceInventory(projectRoot, recorded) {
  validateSourceInventory(recorded);
  const current = sourceInventory(projectRoot);
  requireSame(current, recorded, "Source file contents changed after the release candidate inventory was recorded");
  return current;
}

export function validateSourceInventory(recorded) {
  if (!recorded || recorded.schema !== "fhec-cofhe-phase12-source-inventory-v1" ||
      !Array.isArray(recorded.files) || !Number.isSafeInteger(recorded.source_files) ||
      recorded.source_files < 12 || recorded.source_files > 10000 ||
      recorded.source_files !== recorded.files.length ||
      !Number.isSafeInteger(recorded.source_bytes) || recorded.source_bytes < 0 ||
      recorded.source_bytes > maxInventoryBytes) {
    throw new Error("Invalid or unbounded source inventory schema");
  }
  let total = 0;
  const names = [];
  for (const entry of recorded.files) {
    const name = assertSafeInventoryPath(entry.file);
    if (!allowedFile(name) || secretName.test(name) ||
        !Number.isSafeInteger(entry.bytes) || entry.bytes < 0 || entry.bytes > maxFileSize ||
        !/^[A-F0-9]{64}$/.test(entry.sha256)) {
      throw new Error("Unreviewed, malformed or unsafe source manifest entry: " + name);
    }
    if (![...selectedFiles, ...selectedRoots].some((root) => name === root || name.startsWith(root + "/"))) {
      throw new Error("Source manifest includes an unreviewed path root: " + name);
    }
    if (name.split("/").some((part) => excludedDirectories.has(part))) {
      throw new Error("Generated or transient path is not eligible for a release inventory: " + name);
    }
    total += entry.bytes;
    names.push(name);
  }
  if (total !== recorded.source_bytes || !allUnique(names.map((name) => name.toLowerCase())) ||
      names.join("\n") !== [...names].sort().join("\n") ||
      recorded.tree_sha256 !== sha256(JSON.stringify(recorded.files))) {
    throw new Error("Source inventory totals, order, digest or uniqueness were corrupted");
  }
  return recorded;
}

export function assertSafeInventoryPath(value) {
  if (typeof value !== "string" || value.startsWith("/") || value.includes("\\") ||
      value.split("/").some((part) => !part || part === "." || part === "..")) {
    throw new Error("Invalid source inventory path");
  }
  return value;
}
