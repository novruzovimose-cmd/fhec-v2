import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { analyzeLive } from "./analyze-live.mjs";
import { syntheticEvidence } from "./test-data.mjs";

describe("Phase 9 independently checked real-chain native-fee analysis", () => {
  it("accepts a fully matched six-family synthetic RPC trace, without inventing coprocessor fees", () => {
    const { plan, raw, attestation } = syntheticEvidence();
    const result = analyzeLive(plan, raw, attestation);
    assert.equal(result.family_count, 6);
    assert.equal(result.successful_onchain_execution_receipts, 12);
    assert.equal(result.successful_onchain_deploy_receipts, 2);
    assert.equal(result.total_native_chain_fees_wei, "280000000");
    assert.equal(result.deployment_native_chain_fees_wei, "40000000");
    assert.equal(result.execution_native_chain_fees_wei, "240000000");
    assert.equal(result.coprocessor_fee_wei, null);
    assert.equal(result.families[0].variants.optimized.transaction_gas.p95, null);
  });
  it("keeps all 24 families under the larger deterministic live plan", () => {
    const { plan, raw, attestation } = syntheticEvidence("coverage24");
    const report = analyzeLive(plan, raw, attestation);
    assert.equal(report.family_count, 24);
    assert.equal(report.case_count, 24);
    assert.equal(report.optimized_higher_gas_families_vs_tuned, 0);
  });
  it("rejects a forged or stale RPC attestation hash", () => {
    const { plan, raw, attestation } = syntheticEvidence();
    attestation.raw_trace_sha256 = "F".repeat(64);
    assert.throws(() => analyzeLive(plan, raw, attestation), /attestation/);
  });
  it("rejects a missing deployment receipt and falsified fee coverage", () => {
    const { plan, raw, attestation } = syntheticEvidence();
    attestation.receipts.shift();
    assert.throws(() => analyzeLive(plan, raw, attestation), /attestation/);
    const again = syntheticEvidence();
    again.attestation.receipts[0].native_chain_fee_wei = "10";
    assert.throws(() => analyzeLive(again.plan, again.raw, again.attestation), /deployment fee/);
  });
  it("does not confuse a correct SDK output with on-chain gas proof", () => {
    const { plan, raw, attestation } = syntheticEvidence();
    raw.records[0].variants.optimized.gas_used = "199999";
    assert.throws(() => analyzeLive(plan, raw, attestation), /Invalid recorded execution native fee/);
  });
});
