import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createMatrix } from "../phase6/oracle.mjs";
import { canonicalHash, median, percentile, verifyTriplet } from "./metrics.mjs";
import { variants } from "./generate-manual.mjs";

function fakeEvidence(matrix) {
  let nonce = 0n;
  const records = matrix.cases.map((entry) => {
    const triplets = {};
    for (const variant of variants) {
      nonce += 1n;
      triplets[variant.name] = {
        result: String(entry.expected),
        gas_used: "100000",
        elapsed_ms: "12",
        transaction_hash: "0x" + nonce.toString(16).padStart(64, "0"),
      };
    }
    return { id: entry.id, operation: entry.operation, width: entry.width, expected: entry.expected, variants: triplets };
  });
  return {
    schema: "fhec-cofhe-phase7-mock-triplet-v1",
    environment: "LOCAL_HARDHAT_MOCK_NOT_LIVE_FHE",
    matrix_sha256: canonicalHash(matrix),
    case_count: matrix.total_cases,
    records,
  };
}

describe("Phase 7 three-way audit and gas statistics", () => {
  it("uses exact integer arithmetic for median and nearest-rank p95", () => {
    assert.equal(median([2n, 8n, 4n]), 4n);
    assert.equal(median([2n, 4n]), 3n);
    assert.equal(percentile([1n, 2n, 4n, 8n, 16n], 95), 16n);
    assert.equal(percentile([8n, 1n, 3n, 4n], 1), 1n);
    assert.throws(() => percentile([], 95));
  });
  it("verifies an entire three-way 24-family matrix without inventing speedup", () => {
    const matrix = createMatrix(2);
    const report = verifyTriplet(matrix, fakeEvidence(matrix));
    assert.equal(report.case_count, 48);
    assert.equal(report.mock_transactions_verified, 144);
    assert.equal(report.families.length, 24);
    assert.equal(report.win_families_vs_straight, 0);
    assert.equal(report.win_families_vs_tuned, 0);
    assert.equal(report.regression_families_vs_tuned, 0);
    assert.equal(report.neither_live_gas_nor_coprocessor_fees_measured, true);
  });
  it("does not hide a measured regression in an aggregate result", () => {
    const matrix = createMatrix(2);
    const evidence = fakeEvidence(matrix);
    for (const record of evidence.records.filter((item) => item.operation === "gt" && item.width === 8)) {
      record.variants.optimized.gas_used = "150000";
    }
    const report = verifyTriplet(matrix, evidence);
    assert.equal(report.regression_families_vs_tuned, 1);
    const family = report.families.find((item) => item.family === "gt/u8");
    assert.equal(family.gas_saving_vs_tuned_basis_points, "-5000");
    assert.equal(family.comparison_vs_tuned, "regression");
    assert.equal(family.exact_gas_difference_vs_tuned, "-50000");
  });
  it("detects truncated and falsely labelled input", () => {
    const matrix = createMatrix(2);
    const evidence = fakeEvidence(matrix);
    evidence.records.pop();
    assert.throws(() => verifyTriplet(matrix, evidence), /Incomplete/);
    const other = fakeEvidence(matrix);
    other.environment = "LIVE_MAINNET";
    assert.throws(() => verifyTriplet(matrix, other), /incorrectly scoped/);
  });
  it("rejects forged oracle expectations and missing reference versions", () => {
    const matrix = createMatrix(2);
    const evidence = fakeEvidence(matrix);
    evidence.records[0].expected = "forged";
    assert.throws(() => verifyTriplet(matrix, evidence), /wrong expectation/);
    const other = fakeEvidence(matrix);
    delete other.records[0].variants.tuned;
    assert.throws(() => verifyTriplet(matrix, other), /incomplete triple/);
  });
  it("rejects duplicate receipt hashes and malformed gas values", () => {
    const matrix = createMatrix(2);
    const evidence = fakeEvidence(matrix);
    evidence.records[0].variants.tuned.transaction_hash = evidence.records[0].variants.straight.transaction_hash;
    assert.throws(() => verifyTriplet(matrix, evidence), /duplicate receipt/);
    const other = fakeEvidence(matrix);
    other.records[0].variants.straight.gas_used = "NaN";
    assert.throws(() => verifyTriplet(matrix, other), /decimal-string measurement/);
  });
  it("rejects unsafe numeric scales and counterfeit matrix digests", () => {
    const matrix = createMatrix(2);
    const evidence = fakeEvidence(matrix);
    evidence.records[0].variants.optimized.gas_used = "9007199254740992";
    assert.throws(() => verifyTriplet(matrix, evidence), /JSON-safe scale/);
    const other = fakeEvidence(matrix);
    other.matrix_sha256 = "0".repeat(64);
    assert.throws(() => verifyTriplet(matrix, other), /incorrectly scoped/);
  });
});
