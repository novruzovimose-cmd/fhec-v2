import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assertMatrix } from "../phase6/oracle.mjs";
import { variants } from "./generate-manual.mjs";

const variantNames = variants.map((variant) => variant.name);
const hashExpression = /^0x[0-9a-fA-F]{64}$/;
const integerExpression = /^(0|[1-9][0-9]*)$/;
const safeBigLimit = BigInt(Number.MAX_SAFE_INTEGER);

function requireDecimal(value, positive = false) {
  if (typeof value !== "string" || !integerExpression.test(value)) {
    throw new Error("Missing or malformed decimal-string measurement");
  }
  const result = BigInt(value);
  if ((positive && result === 0n) || result > safeBigLimit) {
    throw new Error("Measurement is zero or exceeds JSON-safe scale");
  }
  return result;
}

export function sha256(content) {
  return createHash("sha256").update(content).digest("hex").toUpperCase();
}

export function canonicalHash(document) {
  return sha256(JSON.stringify(document));
}

export function percentile(numbers, percentage) {
  if (!Array.isArray(numbers) || numbers.length === 0 || !Number.isInteger(percentage) ||
      percentage < 1 || percentage > 100 || numbers.some((value) => typeof value !== "bigint" || value < 0n)) {
    throw new Error("Percentile requires a nonempty array of unsigned bigint measurements");
  }
  const ascending = [...numbers].sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
  const index = Math.max(0, Math.ceil((percentage / 100) * ascending.length) - 1);
  return ascending[index];
}

export function median(numbers) {
  if (!numbers.length) throw new Error("Median requires measurements");
  const ascending = [...numbers].sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
  const middle = Math.floor(ascending.length / 2);
  return ascending.length % 2 ? ascending[middle] : (ascending[middle - 1] + ascending[middle]) / 2n;
}

function safeBasisPoints(reference, optimized) {
  if (reference === 0n) throw new Error("Zero reference gas cannot produce a percentage");
  return ((reference - optimized) * 10000n) / reference;
}

export function verifyTriplet(matrix, payload) {
  assertMatrix(matrix);
  if (!payload || payload.schema !== "fhec-cofhe-phase7-mock-triplet-v1" ||
      payload.environment !== "LOCAL_HARDHAT_MOCK_NOT_LIVE_FHE" ||
      payload.case_count !== matrix.total_cases ||
      payload.matrix_sha256 !== canonicalHash(matrix) ||
      !Array.isArray(payload.records) || payload.records.length !== matrix.total_cases) {
    throw new Error("Incomplete or incorrectly scoped Phase 7 mock report");
  }
  const byId = new Map();
  const allReceiptHashes = new Set();
  for (const record of payload.records) {
    if (!record || typeof record.id !== "string" || byId.has(record.id)) {
      throw new Error("Duplicate or malformed Phase 7 case ID");
    }
    byId.set(record.id, record);
  }
  const groups = new Map();
  for (const testCase of matrix.cases) {
    const record = byId.get(testCase.id);
    if (!record || record.operation !== testCase.operation || record.width !== testCase.width ||
        record.expected !== testCase.expected || !record.variants ||
        Object.keys(record.variants).sort().join(",") !== [...variantNames].sort().join(",")) {
      throw new Error("Missing family, wrong expectation, or incomplete triple: " + testCase.id);
    }
    for (const variant of variantNames) {
      const value = record.variants[variant];
      if (!value || value.result !== String(testCase.expected) ||
          !hashExpression.test(value.transaction_hash) || allReceiptHashes.has(value.transaction_hash.toLowerCase()) ||
          requireDecimal(value.gas_used, true) <= 0n || requireDecimal(value.elapsed_ms) < 0n) {
        throw new Error("Invalid mock gas, result, transaction, or duplicate receipt: " + testCase.id);
      }
      allReceiptHashes.add(value.transaction_hash.toLowerCase());
    }
    const key = testCase.operation + "/u" + testCase.width;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(record);
  }
  if (groups.size !== 24 || allReceiptHashes.size !== matrix.total_cases * 3) {
    throw new Error("Missing operation-width families or distinct transactions");
  }
  const familyResults = [];
  let winsAgainstStraight = 0;
  let winsAgainstTuned = 0;
  let regressionsAgainstTuned = 0;
  for (const [family, records] of groups) {
    const perVariant = {};
    for (const name of variantNames) {
      const gas = records.map((record) => BigInt(record.variants[name].gas_used));
      const elapsed = records.map((record) => BigInt(record.variants[name].elapsed_ms));
      perVariant[name] = {
        gas_min: percentile(gas, 1).toString(),
        gas_median: median(gas).toString(),
        gas_p95: percentile(gas, 95).toString(),
        gas_max: percentile(gas, 100).toString(),
        elapsed_median_ms: median(elapsed).toString(),
        elapsed_p95_ms: percentile(elapsed, 95).toString(),
      };
    }
    const optimized = BigInt(perVariant.optimized.gas_median);
    const straight = BigInt(perVariant.straight.gas_median);
    const tuned = BigInt(perVariant.tuned.gas_median);
    if (optimized < straight) winsAgainstStraight += 1;
    if (optimized < tuned) winsAgainstTuned += 1;
    if (optimized > tuned) regressionsAgainstTuned += 1;
    familyResults.push({
      family,
      cases: records.length,
      exact_gas_difference_vs_straight: (straight - optimized).toString(),
      exact_gas_difference_vs_tuned: (tuned - optimized).toString(),
      comparison_vs_straight: optimized < straight ? "win" : optimized > straight ? "regression" : "tie",
      comparison_vs_tuned: optimized < tuned ? "win" : optimized > tuned ? "regression" : "tie",
      gas_saving_vs_straight_basis_points: safeBasisPoints(straight, optimized).toString(),
      gas_saving_vs_tuned_basis_points: safeBasisPoints(tuned, optimized).toString(),
      variants: perVariant,
    });
  }
  return {
    schema: "fhec-cofhe-phase7-verified-mock-comparison-v1",
    evidence_tier: "verified-local-mock-only",
    source_matrix_sha256: canonicalHash(matrix),
    case_count: matrix.total_cases,
    mock_transactions_verified: matrix.total_cases * 3,
    families: familyResults,
    win_families_vs_straight: winsAgainstStraight,
    win_families_vs_tuned: winsAgainstTuned,
    regression_families_vs_tuned: regressionsAgainstTuned,
    neither_live_gas_nor_coprocessor_fees_measured: true,
    methodology: [
      "All three wrappers have byte-for-byte identical dispatch and ACL logic after name normalization.",
      "Straightforward reference deliberately computes the repeated expression twice; tuned reference shares it.",
      "Median comparisons are reported per operation and width. No selected benchmark is hidden.",
      "This report does not prove performance, correctness or authorization on a live CoFHE testnet.",
    ],
  };
}

function readJson(path) {
  const data = readFileSync(path, "utf8").replace(/^\uFEFF/, "");
  return JSON.parse(data);
}

export function main(args) {
  if (args.length !== 6 || args[0] !== "--matrix" || args[2] !== "--raw" || args[4] !== "--out") {
    throw new Error("Usage: --matrix <matrix.json> --raw <triplet.json> --out <summary.json>");
  }
  const report = verifyTriplet(readJson(args[1]), readJson(args[3]));
  writeFileSync(args[5], JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
  process.stdout.write("Verified " + report.case_count + " triple cases with " + report.mock_transactions_verified + " mock transactions.\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 7 mock verification failed: " + error.message + "\n"); process.exitCode = 1; }
}
