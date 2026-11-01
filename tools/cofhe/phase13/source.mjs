import { existsSync, lstatSync } from "node:fs";
import { join, resolve } from "node:path";
import { verifyCandidate } from "../phase12/verify.mjs";
import { assertDigest, boundedFile, hash, jsonFile, safeName, within } from "./io.mjs";

const requiredPredecessors = [
  "Cargo.toml", "Cargo.lock", "tools/cofhe/phase12/verify.mjs",
  "tools/cofhe/phase89/verify-chain.mjs", "tools/cofhe/phase89/analyze-live.mjs",
  "integration/cofhe-hardhat/package.json", "integration/cofhe-hardhat/pnpm-lock.yaml",
];

export function verifyLocal(projectDirectory, capsuleDirectory) {
  const project = resolve(projectDirectory);
  const candidate = resolve(capsuleDirectory);
  for (const name of requiredPredecessors) {
    const file = join(project, ...name.split("/"));
    if (!existsSync(file) || !lstatSync(file).isFile() || lstatSync(file).isSymbolicLink()) {
      throw new Error("Missing or unsafe Phase 13 predecessor: " + name);
    }
  }
  const previous = verifyCandidate(candidate, null);
  const inventory = jsonFile(join(candidate, "source-inventory.json"));
  if (inventory.schema !== "fhec-cofhe-phase12-source-inventory-v1" ||
      !Array.isArray(inventory.files) || inventory.files.length !== inventory.source_files) {
    throw new Error("Invalid Phase 12 source inventory");
  }
  let unchanged = 0;
  for (const item of inventory.files) {
    const filename = within(project, join(project, ...safeName(item.file).split("/")));
    const actual = boundedFile(filename, 4 * 1024 * 1024);
    if (actual.length !== item.bytes || hash(actual) !== assertDigest(item.sha256, "recorded source digest")) {
      throw new Error("Original Phase 12 source changed: " + item.file);
    }
    unchanged += 1;
  }
  const dependency = jsonFile(join(project, "integration/cofhe-hardhat/package.json"), 1024 * 1024);
  const dev = dependency.devDependencies || {};
  return {
    schema: "fhec-cofhe-phase13-source-preflight-v1",
    candidate_status: previous.status,
    candidate_files: previous.files,
    phase12_source_inventory_tree_sha256: assertDigest(inventory.tree_sha256),
    phase12_recorded_source_files_unchanged: unchanged,
    phase12_source_files_unchanged: unchanged === inventory.source_files,
    new_phase13_files_excluded_from_phase12_original_source_seal: true,
    dependency_snapshot: {
      sdk: dev["@cofhe/sdk"] ?? null,
      hardhat_plugin: dev["@cofhe/hardhat-plugin"] ?? null,
      cofhe_contracts: dev["@fhenixprotocol/cofhe-contracts"] ?? null,
    },
    warnings: [
      "The original source seal is verified file-by-file; newly installed Phase 13 files are not part of the prior capsule.",
      "A verified local candidate does not prove a live network run or complete upstream CoFHE API coverage.",
      "Upstream contract and SDK versions must be reviewed before any dependency migration.",
    ],
  };
}
