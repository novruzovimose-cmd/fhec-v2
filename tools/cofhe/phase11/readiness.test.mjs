import assert from "node:assert/strict";
import { it } from "node:test";
import { evaluateReadiness } from "./readiness.mjs";

const valid = () => ({
  schema: "fhec-cofhe-phase10-validated-mock-results-v1",
  evidence_tier: "independently-recomputed-local-mock-differential-results",
  cases: 72, successful_mock_executions: 216, operation_width_families: 24,
});

it("distinguishes passed local evidence from live, security, release and grant gates", () => {
  const result = evaluateReadiness(valid());
  assert.equal(result.total_gates, 9);
  assert.equal(result.locally_verified_gates, 4);
  assert.equal(result.external_evidence_required_gates, 4);
  assert.equal(result.outside_tool_control_gates, 1);
  assert.equal(result.granted_funding_usd, null);
  assert.ok(result.gates.every((gate) => gate.status !== "verified-live"));
});

it("refuses malformed local evidence and will not promote mock data to real testnet", () => {
  const corrupted = valid();
  corrupted.evidence_tier = "verified-real-testnet";
  assert.throws(() => evaluateReadiness(corrupted), /full independently validated/);
  const incomplete = valid();
  incomplete.operation_width_families = 6;
  assert.throws(() => evaluateReadiness(incomplete), /full independently validated/);
});
