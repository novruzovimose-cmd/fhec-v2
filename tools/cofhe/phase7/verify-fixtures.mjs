import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assertMatrix, createMatrix, expected, widths, operations } from "../phase6/oracle.mjs";
import { sha256 } from "./metrics.mjs";

export const fixtures = Object.freeze([
  Object.freeze({ filename: "golden-benchmark-384-v1.json", perFamily: 16, total: 384 }),
  Object.freeze({ filename: "golden-stress-672-v1.json", perFamily: 28, total: 672 }),
]);

export function analyzeFixture(matrix, profile) {
  assertMatrix(matrix);
  if (matrix.cases_per_family !== profile.perFamily || matrix.total_cases !== profile.total ||
      JSON.stringify(matrix) !== JSON.stringify(createMatrix(profile.perFamily))) {
    throw new Error("Golden fixture differs from canonical independent Phase 6 oracle");
  }
  const seen = new Set();
  const groups = new Map();
  for (const entry of matrix.cases) {
    if (seen.has(entry.id)) throw new Error("Duplicate golden case ID");
    seen.add(entry.id);
    if (!widths.includes(entry.width) || !operations.includes(entry.operation)) {
      throw new Error("Unknown operation-width family");
    }
    const key = entry.operation + "/u" + entry.width;
    groups.set(key, (groups.get(key) ?? 0) + 1);
    const computed = expected(entry.width, entry.operation, BigInt(entry.a), BigInt(entry.b));
    if (String(computed) !== String(entry.expected)) {
      throw new Error("Incorrect independently recomputed oracle value: " + entry.id);
    }
  }
  if (groups.size !== 24 || [...groups.values()].some((count) => count !== profile.perFamily)) {
    throw new Error("A golden operation-width family is incomplete");
  }
  return {
    profile: profile.filename,
    families: groups.size,
    cases: matrix.total_cases,
    boundary_cases: matrix.cases.filter((item) => item.boundary).length,
    generated_cases: matrix.cases.filter((item) => !item.boundary).length,
    canonical_sha256: sha256(JSON.stringify(matrix)),
  };
}

export function checkFixtures(directory) {
  return fixtures.map((profile) => {
    const path = resolve(directory, profile.filename);
    const matrix = JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
    return analyzeFixture(matrix, profile);
  });
}

export function main(args) {
  if (args.length !== 2 || args[0] !== "--directory") {
    throw new Error("Usage: --directory <tools/cofhe/phase7>");
  }
  const results = checkFixtures(args[1]);
  for (const item of results) {
    process.stdout.write(item.profile + ": " + item.cases + " deterministic cases, " + item.families + " complete families, SHA256 " + item.canonical_sha256 + "\n");
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 7 golden fixture verification failed: " + error.message + "\n"); process.exitCode = 1; }
}
