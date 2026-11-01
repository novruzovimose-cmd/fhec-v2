import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { analyzeLive } from "./analyze-live.mjs";
import { renderHtml } from "./report-html.mjs";
import { syntheticEvidence } from "./test-data.mjs";

function sample() {
  const { plan, raw, attestation } = syntheticEvidence();
  return analyzeLive(plan, raw, attestation);
}

describe("Phase 9 offline grant-evidence renderer", () => {
  it("renders native chain gas and fees, but never labels coprocessor fees as measured", () => {
    const html = renderHtml(sample());
    assert.match(html, /REAL ARBITRUM SEPOLIA TRANSACTION FEES ONLY/);
    assert.match(html, /UNKNOWN \/ NOT MEASURED/);
    assert.match(html, /280000000/);
    assert.doesNotMatch(html, /<script|<iframe|https:\/\//);
  });
  it("rejects forged tiers and absent gas rows", () => {
    const changed = sample();
    changed.evidence_tier = "production-mainnet-measured-all-costs";
    assert.throws(() => renderHtml(changed), /Only independently/);
    const missing = sample();
    missing.families.pop();
    assert.throws(() => renderHtml(missing), /Only independently/);
  });
  it("detects a contradictory win/regression label", () => {
    const changed = sample();
    changed.families[0].optimized_vs_tuned = "higher-gas";
    assert.throws(() => renderHtml(changed), /inconsistent/);
  });
});
