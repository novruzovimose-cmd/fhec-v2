import { deepStrictEqual } from "node:assert/strict";
import { writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { summarize } from "../summarize-phase4.mjs";
import { digest, exact, integer, jsonFile, object, options, requireNewPath, within } from "./strict.mjs";

const localHashes = [
  ["baseline_solidity_sha256", "FHECPhase4Baseline.sol"],
  ["optimized_solidity_sha256", "FHECPhase4Optimized.sol"],
  ["cost_report_sha256", "optimized-static-report.json"],
  ["budget_decision_sha256", "encrypted-operation-budget.json"],
  ["budget_contrast_sha256", "encrypted-operation-budget-contrast.json"],
  ["raw_mock_samples_sha256", "mock-samples.json"],
  ["mock_summary_sha256", "mock-gas-summary.json"],
  ["mock_html_report_sha256", "mock-evidence-report.html"],
];

export function auditLocal(projectRoot, runDirectory) {
  const project = resolve(projectRoot);
  const run = within(project, resolve(runDirectory));
  const validationPath = within(run, join(run, "validation-report.json"));
  const validation = jsonFile(validationPath, "Phase 4 validation");
  exact(validation.schema, "fhec-cofhe-phase4-local-validation-v1", "validation schema");
  exact(validation.environment, "Fhenix Hardhat CoFHE MOCK", "validation environment");
  exact(validation.three_way_oracle_mock_tests, "passed", "differential test status");
  exact(validation.signer_contract_proof_acl_negative_tests, "passed", "negative test status");
  exact(validation.contract_sources_restored, true, "source restoration");
  exact(validation.live_cofhe_execution, "not tested", "live execution status");
  exact(validation.real_network_gas, "not measured", "real gas status");
  exact(validation.real_tfhe_execution, "not checked by this script", "runtime execution status");
  integer(validation.family_count, "family count", 24, 24);
  integer(validation.cases_per_family, "cases per family", 1, 8);
  integer(validation.completed_mock_comparisons, "comparisons", 24, 192);
  exact(validation.completed_mock_comparisons, validation.cases_per_family * 24, "expected comparisons");

  const checked = [];
  function assertDigest(expectedHash, relativePath, root) {
    if (typeof expectedHash !== "string" || !/^[a-fA-F0-9]{64}$/.test(expectedHash)) {
      throw new Error(`Missing or invalid SHA-256 for ${relativePath}`);
    }
    const absolute = within(root, join(root, relativePath));
    const computed = digest(absolute);
    exact(computed, expectedHash.toUpperCase(), `SHA-256 for ${relativePath}`);
    checked.push({ path: relativePath, sha256: computed });
  }

  assertDigest(validation.source_sha256, "examples/cofhe_phase4.fhe", project);
  assertDigest(validation.corpus_sha256, "integration/cofhe-hardhat/testdata/cofhe-vectors.jsonl", project);
  assertDigest(validation.manual_solidity_sha256, "integration/cofhe-hardhat/phase4/FHECPhase4Manual.sol", project);
  for (const [field, name] of localHashes) assertDigest(validation[field], name, run);

  const samples = jsonFile(join(run, "mock-samples.json"), "Phase 4 mock samples");
  const actualSummary = jsonFile(join(run, "mock-gas-summary.json"), "Phase 4 summary");
  const independentSummary = summarize(samples);
  deepStrictEqual(actualSummary, independentSummary, "Phase 4 mock summary no longer matches the independent oracle");
  exact(actualSummary.total_cases, validation.completed_mock_comparisons, "mock sample count");

  const report = jsonFile(join(run, "optimized-static-report.json"), "static cost report");
  exact(report.schema, "fhec-cofhe-static-optimization-v1", "cost report schema");
  integer(report.call_graph?.length, "cost graph function count", 26, 26);

  return {
    schema: "fhec-cofhe-phase5-local-attestation-v1",
    tier: "verified-local-mock-only",
    validation_sha256: digest(validationPath),
    completed_mock_comparisons: validation.completed_mock_comparisons,
    function_count: report.call_graph.length,
    source_hashes: checked.sort((a, b) => a.path.localeCompare(b.path)),
    cryptographic_claims: {
      generated_solidity_hash_verified: true,
      deterministic_mock_oracle_rechecked: true,
      live_fhe_proven: false,
      public_network_fhe_proven: false,
      production_security_audited: false,
    },
    caveats: [
      "All execution figures are local Hardhat MOCK measurements.",
      "The static cost estimate does not measure coprocessor or network fees.",
      "This attestation verifies evidence integrity, not FHE cryptographic security.",
    ],
  };
}

export function main(args) {
  const { project, run, out } = options(args, { "--project": "project", "--run": "run", "--out": "out" });
  const artifact = auditLocal(project, run);
  writeFileSync(requireNewPath(out), `${JSON.stringify(artifact, null, 2)}\n`, { flag: "wx" });
  process.stdout.write(`Phase 5 local attestation verified ${artifact.completed_mock_comparisons} mock comparisons; no live execution claimed.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write(`Phase 5 local audit failed: ${error.message}\n`); process.exitCode = 1; }
}
