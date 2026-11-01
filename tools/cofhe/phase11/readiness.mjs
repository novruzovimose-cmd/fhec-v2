import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function evaluateReadiness(validated) {
  if (!validated || validated.schema !== "fhec-cofhe-phase10-validated-mock-results-v1" ||
      validated.evidence_tier !== "independently-recomputed-local-mock-differential-results" ||
      !Number.isSafeInteger(validated.cases) || validated.cases <= 0 ||
      !Number.isSafeInteger(validated.successful_mock_executions) || validated.successful_mock_executions <= 0 ||
      validated.operation_width_families !== 24) {
    throw new Error("A full independently validated local mock report is required before readiness assessment");
  }
  const gates = [
    { id: "P10_SOURCE_SEAL", status: "verified-local", requirement: "Reviewed generated Solidity source seal and full project predecessor provenance", evidence: "Prepared source seal copied into the self-contained dossier" },
    { id: "P10_INDEPENDENT_ORACLE", status: "verified-local", requirement: "Independent BigInt modular reference over all unsigned types", evidence: `${validated.cases} canonical property vectors across 24 families` },
    { id: "P10_MOCK_DIFFERENTIAL", status: "verified-local", requirement: "Complete source-sealed CoFHE mock execution results for all selected variants", evidence: `${validated.successful_mock_executions} full-width mock ciphertext executions` },
    { id: "P11_OFFLINE_DOSSIER", status: "verified-local", requirement: "Independent read-only rechecking of bundle source, oracle and result hashes", evidence: "Phase 11 SHA-256 manifest and independently regenerated output" },
    { id: "LIVE_TESTNET", status: "requires-external-evidence", requirement: "Separately approved real Arbitrum Sepolia execution and independently corroborated receipts", evidence: "No live transaction is demonstrated by this local mock dossier" },
    { id: "COPROCESSOR_FEES", status: "requires-external-evidence", requirement: "Actual protocol task prices and off-chain service costs", evidence: "Not inferable from mock gas, FHE operation count or native receipts alone" },
    { id: "HOSTED_CI", status: "requires-external-evidence", requirement: "Public immutable revision passing fresh Windows and Linux hosted runners", evidence: "Local execution does not prove hosted CI" },
    { id: "INDEPENDENT_SECURITY_REVIEW", status: "requires-external-evidence", requirement: "External cryptographic/contract/backend review and remediation", evidence: "Arithmetic mock tests cannot replace a security audit" },
    { id: "GRANT_REVIEW", status: "outside-tool-control", requirement: "Application eligibility, ecosystem impact, and reviewer approval", evidence: "No software test can guarantee a financial award" },
  ];
  return {
    schema: "fhec-cofhe-phase11-release-gates-v1",
    scope: "Evidence gates, never an invented software-completion percentage",
    granted_funding_usd: null,
    total_gates: gates.length,
    locally_verified_gates: gates.filter((gate) => gate.status === "verified-local").length,
    external_evidence_required_gates: gates.filter((gate) => gate.status === "requires-external-evidence").length,
    outside_tool_control_gates: gates.filter((gate) => gate.status === "outside-tool-control").length,
    gates,
  };
}

export function main(args) {
  if (args.length !== 4 || args[0] !== "--validated" || args[2] !== "--out") {
    throw new Error("Usage: --validated <independent-mock-report> --out <new-readiness-gates.json>");
  }
  const validated = JSON.parse(readFileSync(args[1], "utf8").replace(/^\uFEFF/, ""));
  const result = evaluateReadiness(validated);
  writeFileSync(args[3], JSON.stringify(result, null, 2) + "\n", { flag: "wx" });
  process.stdout.write(`${result.locally_verified_gates} local gates verified, ${result.external_evidence_required_gates} independent external gates remain; no grant or completeness percentage asserted.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 11 readiness gate evaluation failed: " + error.message + "\n"); process.exitCode = 1; }
}
