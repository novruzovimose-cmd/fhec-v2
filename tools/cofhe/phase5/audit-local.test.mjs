import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { describe, it } from "node:test";
import { summarize } from "../summarize-phase4.mjs";
import { auditLocal } from "./audit-local.mjs";
import { digest } from "./strict.mjs";

const widths = [8, 16, 32, 64];
const operations = ["add", "sub", "mul", "gt", "max", "repeated"];

function put(root, path, value) {
  const name = join(root, path);
  mkdirSync(dirname(name), { recursive: true });
  writeFileSync(name, value);
  return digest(name);
}

function fixture() {
  const project = mkdtempSync(join(tmpdir(), "fhec-phase5-audit-"));
  const run = join(project, "target", "cofhe-phase4-test");
  mkdirSync(run, { recursive: true });
  const sourceHash = put(project, "examples/cofhe_phase4.fhe", "fn test() {}");
  const corpusHash = put(project, "integration/cofhe-hardhat/testdata/cofhe-vectors.jsonl", "{}\n");
  const manualHash = put(project, "integration/cofhe-hardhat/phase4/FHECPhase4Manual.sol", "library Manual {}");
  const samples = [];
  for (const width of widths) for (const operation of operations) {
    const a = 8n;
    const b = 3n;
    const max = (1n << BigInt(width)) - 1n;
    const expected = {
      add: ((a + b) & max).toString(), sub: ((a - b) & max).toString(),
      mul: ((a * b) & max).toString(), gt: a > b,
      max: (a > b ? a : b).toString(), repeated: ((2n * a * b) & max).toString(),
    }[operation];
    samples.push({ width, operation, vector_id: `${width}-${operation}`,
      operands: { a: "8", b: "3" }, expected,
      mock_gas: { baseline: "60000", optimized: "40000", manual: "35000" },
    });
  }
  const raw = { schema: "fhec-cofhe-phase4-mock-samples-v1", cases_per_family: 1,
    family_count: 24, expected_comparisons: 24, completed_comparisons: 24, samples };
  const summary = summarize(raw);
  const fileHashes = {
    baseline_solidity_sha256: put(run, "FHECPhase4Baseline.sol", "library Baseline {}"),
    optimized_solidity_sha256: put(run, "FHECPhase4Optimized.sol", "library Optimized {}"),
    cost_report_sha256: put(run, "optimized-static-report.json", JSON.stringify({ schema: "fhec-cofhe-static-optimization-v1", call_graph: Array.from({ length: 26 }, (_, n) => ({ name: `f${n}` })) })),
    budget_decision_sha256: put(run, "encrypted-operation-budget.json", "{}"),
    budget_contrast_sha256: put(run, "encrypted-operation-budget-contrast.json", "{}"),
    raw_mock_samples_sha256: put(run, "mock-samples.json", JSON.stringify(raw)),
    mock_summary_sha256: put(run, "mock-gas-summary.json", JSON.stringify(summary)),
    mock_html_report_sha256: put(run, "mock-evidence-report.html", "<!doctype html><html></html>"),
  };
  const validation = {
    schema: "fhec-cofhe-phase4-local-validation-v1",
    environment: "Fhenix Hardhat CoFHE MOCK",
    source_sha256: sourceHash,
    corpus_sha256: corpusHash,
    manual_solidity_sha256: manualHash,
    ...fileHashes,
    family_count: 24,
    cases_per_family: 1,
    completed_mock_comparisons: 24,
    three_way_oracle_mock_tests: "passed",
    signer_contract_proof_acl_negative_tests: "passed",
    contract_sources_restored: true,
    real_tfhe_execution: "not checked by this script",
    live_cofhe_execution: "not tested",
    real_network_gas: "not measured",
  };
  put(run, "validation-report.json", JSON.stringify(validation));
  return { project, run, validation, raw, summary };
}

describe("Phase 5 independent mock-evidence audit", () => {
  it("accepts coherent Phase 4 evidence without elevating it to live FHE", () => {
    const { project, run } = fixture();
    const result = auditLocal(project, run);
    assert.equal(result.completed_mock_comparisons, 24);
    assert.equal(result.source_hashes.length, 11);
    assert.equal(result.cryptographic_claims.live_fhe_proven, false);
    assert.equal(result.cryptographic_claims.generated_solidity_hash_verified, true);
  });
  it("rejects even a one-byte generated Solidity tamper", () => {
    const { project, run } = fixture();
    put(run, "FHECPhase4Baseline.sol", "library Baseline{ } ");
    assert.throws(() => auditLocal(project, run), /SHA-256/);
  });
  it("rejects an altered mock summary even if validation hash is updated", () => {
    const { project, run, validation, summary } = fixture();
    summary.families[0].variants.baseline.median_mock_gas = "1";
    validation.mock_summary_sha256 = put(run, "mock-gas-summary.json", JSON.stringify(summary));
    put(run, "validation-report.json", JSON.stringify(validation));
    assert.throws(() => auditLocal(project, run), /mock summary no longer matches/);
  });
  it("rejects altered proof status even if all hashes remain unchanged", () => {
    const { project, run, validation } = fixture();
    validation.live_cofhe_execution = "passed";
    put(run, "validation-report.json", JSON.stringify(validation));
    assert.throws(() => auditLocal(project, run), /live execution status/);
  });
  it("rejects a false test count", () => {
    const { project, run, validation } = fixture();
    validation.completed_mock_comparisons = 48;
    put(run, "validation-report.json", JSON.stringify(validation));
    assert.throws(() => auditLocal(project, run), /expected comparisons/);
  });
  it("accepts a UTF-8 BOM in Windows PowerShell's validation-report.json", () => {
    const { project, run, validation } = fixture();
    const bytes = Buffer.concat([
      Buffer.from([0xef, 0xbb, 0xbf]),
      Buffer.from(JSON.stringify(validation), "utf8"),
    ]);
    put(run, "validation-report.json", bytes);
    const result = auditLocal(project, run);
    assert.equal(result.completed_mock_comparisons, 24);
    assert.equal(result.cryptographic_claims.public_network_fhe_proven, false);
  });
  it("rejects an outside-run path", () => {
    const { project } = fixture();
    assert.throws(() => auditLocal(project, tmpdir()), /escapes expected root/);
  });
});
