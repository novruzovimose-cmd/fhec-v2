import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createMatrix } from "./oracle.mjs";
import { validateLive, validateMock } from "./validate-results.mjs";

function mockFixture() {
  const matrix = createMatrix(2);
  const result = {
    schema: "fhec-cofhe-phase6-mock-coverage-v1",
    environment: "Fhenix Hardhat mock, NOT actual FHE coprocessor",
    family_count: 24,
    cases_per_family: 2,
    completed_cases: matrix.total_cases,
    records: matrix.cases.map(({ id, width, operation, expected }) => ({
      id, width, operation, expected, measured_mock_gas: "55000", elapsed_wall_ms_including_mock_receipt: "100",
    })),
  };
  return { matrix, result };
}

function liveFixture() {
  const specs = [
    ["maximum-u8", "19", "41", "41"],
    ["addition-wrap-u8", "255", "1", "0"],
    ["greater-u8", "241", "12", "true"],
  ];
  return {
    schema: "fhec-cofhe-phase6-live-testnet-v1",
    chain_id: 421614,
    contract: `0x${"1".repeat(40)}`,
    signer: `0x${"2".repeat(40)}`,
    deployment_transaction: `0x${"a".repeat(64)}`,
    completed_encrypted_cases: 3,
    records: specs.map(([label, input_a, input_b, answer], index) => ({
      label, input_a, input_b, actual: answer, expected: answer,
      matched_independent_oracle: true,
      transaction_hash: `0x${String(index + 1).repeat(64)}`,
      transaction_gas_used: "55000", block_number: 12345,
    })),
  };
}

describe("Phase 6 evidence verifier", () => {
  it("verifies the 24 operation-width families only in the mock tier", () => {
    const { matrix, result } = mockFixture();
    const check = validateMock(matrix, result);
    assert.equal(check.completed_cases, 48);
    assert.equal(check.encrypted_runtime_proven, false);
  });
  it("rejects a duplicated mock result even if its length is preserved", () => {
    const { matrix, result } = mockFixture();
    result.records[0] = result.records[1];
    assert.throws(() => validateMock(matrix, result), /Duplicate/);
  });
  it("rejects overstated or incomplete mock coverage", () => {
    const { matrix, result } = mockFixture();
    result.completed_cases += 1;
    assert.throws(() => validateMock(matrix, result), /incomplete/);
  });
  it("rejects a forged mock oracle result", () => {
    const { matrix, result } = mockFixture();
    result.records[0].expected = "wrong";
    assert.throws(() => validateMock(matrix, result), /incorrect/);
  });
  it("accepts a bounded SDK trace only as provisional", () => {
    const report = validateLive(liveFixture());
    assert.equal(report.test_count, 3);
    assert.equal(report.live_fhe_evidence_independently_verified, false);
  });
  it("rejects false live answers and fabricated chain ID", () => {
    const wrong = liveFixture();
    wrong.records[0].actual = "42";
    assert.throws(() => validateLive(wrong), /expected result/);
    const other = liveFixture();
    other.chain_id = 1;
    assert.throws(() => validateLive(other), /Malformed/);
  });
  it("rejects repeated or missing live test cases", () => {
    const wrong = liveFixture();
    wrong.records[0] = wrong.records[1];
    assert.throws(() => validateLive(wrong), /repeated/);
  });
});
