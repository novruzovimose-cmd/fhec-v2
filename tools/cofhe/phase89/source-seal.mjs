import { createHash } from "node:crypto";
import { lstatSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { loadPlan, planDigest } from "./plan.mjs";

export const sourceNames = Object.freeze([
  "FHECPhase6Optimized.sol",
  "FHECPhase6Live.sol",
  "FHECPhase7ManualStraight.sol",
  "FHECPhase7ManualTuned.sol",
  "FHECPhase7Straight.sol",
  "FHECPhase7Tuned.sol",
  "phase89-plan.json",
]);

export function hashBytes(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

export function generateSeal(directory) {
  const entries = sourceNames.map((name) => {
    const path = join(directory, name);
    const info = lstatSync(path);
    if (!info.isFile() || info.isSymbolicLink()) throw new Error("Source seal forbids non-regular files: " + name);
    const bytes = readFileSync(path);
    return { file: name, bytes: bytes.length, sha256: hashBytes(bytes) };
  });
  return {
    schema: "fhec-cofhe-phase89-source-seal-v1",
    plan_sha256: planDigest(loadPlan(join(directory, "phase89-plan.json"))),
    files: entries,
    files_count: entries.length,
  };
}

export function validateSeal(directory, seal) {
  if (!seal || seal.schema !== "fhec-cofhe-phase89-source-seal-v1" ||
      seal.files_count !== sourceNames.length || !Array.isArray(seal.files) ||
      JSON.stringify(generateSeal(directory)) !== JSON.stringify(seal)) {
    throw new Error("Prepared Phase 8/9 sources were modified after preflight");
  }
  return seal;
}

export function readSeal(path) {
  return JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
}

export function main(args) {
  if (args.length !== 4 || args[0] !== "--directory" || !["--out", "--verify"].includes(args[2])) {
    throw new Error("Usage: --directory <generated> --out <seal.json> | --directory <generated> --verify <seal.json>");
  }
  const directory = resolve(args[1]);
  if (args[2] === "--out") {
    const seal = generateSeal(directory);
    writeFileSync(args[3], JSON.stringify(seal, null, 2) + "\n", { flag: "wx" });
    process.stdout.write("Sealed exact Rust output, reference Solidity and live plan: " + seal.plan_sha256 + "\n");
  } else {
    const result = validateSeal(directory, readSeal(args[3]));
    process.stdout.write("Prepared source integrity verified: " + result.plan_sha256 + "\n");
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 8/9 source integrity failed: " + error.message + "\n"); process.exitCode = 1; }
}
