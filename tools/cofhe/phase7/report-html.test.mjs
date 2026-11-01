import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { escapeHtml, renderReport } from "./report-html.mjs";

function sample() {
  const costs = { gas_median: "100000", gas_p95: "120000" };
  return {
    schema: "fhec-cofhe-phase7-verified-mock-comparison-v1",
    evidence_tier: "verified-local-mock-only",
    source_matrix_sha256: "A".repeat(64),
    case_count: 48,
    mock_transactions_verified: 144,
    win_families_vs_straight: 0,
    win_families_vs_tuned: 0,
    regression_families_vs_tuned: 0,
    neither_live_gas_nor_coprocessor_fees_measured: true,
    methodology: ["No live network gas was measured.", "Source includes <script> and plain text."],
    families: Array.from({ length: 24 }, (_, index) => ({
      family: ["add", "sub", "mul", "gt", "max", "repeated"][Math.floor(index / 4)] + "/u" + [8, 16, 32, 64][index % 4],
      cases: 2,
      gas_saving_vs_straight_basis_points: "0",
      gas_saving_vs_tuned_basis_points: "0",
      exact_gas_difference_vs_straight: "0",
      exact_gas_difference_vs_tuned: "0",
      comparison_vs_straight: "tie",
      comparison_vs_tuned: "tie",
      variants: { optimized: costs, straight: costs, tuned: costs },
    })),
  };
}

describe("Phase 7 shareable HTML evidence report", () => {
  it("escapes arbitrary text, quotes and markup", () => {
    assert.equal(escapeHtml('<script src="x">&</script>'), "&lt;script src=&quot;x&quot;&gt;&amp;&lt;/script&gt;");
  });
  it("renders all 24 benchmark groups without remote scripts", () => {
    const output = renderReport(sample());
    assert.match(output, /144<\/strong>/);
    assert.match(output, /0\.00%/);
    assert.match(output, /LOCAL MOCKS ONLY/);
    assert.match(output, /No live network evidence/);
    assert.match(output, /&lt;script&gt;/);
    assert.doesNotMatch(output, /<script|<iframe|https:\/\//);
  });
  it("cannot disguise partial statistics as verified evidence", () => {
    const report = sample();
    report.families.pop();
    assert.throws(() => renderReport(report), /complete independently verified/);
  });
  it("cannot relabel mock evidence as mainnet results", () => {
    const report = sample();
    report.evidence_tier = "verified-mainnet";
    assert.throws(() => renderReport(report), /complete independently verified/);
  });
  it("includes only a clearly limited RPC receipt report", () => {
    const receipts = {
      schema: "fhec-cofhe-phase7-rpc-corroboration-v1",
      evidence_tier: "testnet-transport-receipt-verified-sdk-decryption-not-independently-attested",
      verified_transaction_count: 3,
      contract: "0x" + "c".repeat(40),
    };
    const output = renderReport(sample(), receipts);
    assert.match(output, /not independently prove FHE computation/);
  });
  it("rejects counterfeit regression directions and summary counts", () => {
    const wrongDirection = sample();
    wrongDirection.families[0].comparison_vs_tuned = "win";
    assert.throws(() => renderReport(wrongDirection), /direction/);
    const wrongCount = sample();
    wrongCount.win_families_vs_tuned = 1;
    assert.throws(() => renderReport(wrongCount), /aggregate/);
  });
});
