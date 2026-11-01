import { test } from "node:test";
import assert from "node:assert/strict";
import { escapeHtml, renderQualification } from "./report.mjs";
import { handoffMarkdown } from "./handoff.mjs";

test("HTML text escapes untrusted descriptive fields", () => {
  assert.equal(escapeHtml('a<&"\'z>'), "a&lt;&amp;&quot;&#39;z&gt;");
});

test("static HTML and handoff preserve evidence-tier limitations", () => {
  const data = {
    source_seal_sha256: "A".repeat(64),
    phase11_evidence_sha256: "B".repeat(64),
    verified_local_gates: ["P11_OFFLINE_DOSSIER"],
    pending_external_gates: ["LIVE_TESTNET", "HOSTED_CI"],
    essential_limitations: ["Not a grant award", "Costs only mock"],
    full_width_coverage: { operation_families: 24, corpus_cases: 72, verified_mock_executions: 216, variants: ["optimized", "tuned", "straight"], matrix: [{ family: "add/u8", cases: 3, mock_executions: 9, variants: ["optimized", "tuned", "straight"] }] },
  };
  const inventory = { source_files: 24, source_bytes: 999, tree_sha256: "C".repeat(64) };
  const html = renderQualification(data, inventory);
  const handoff = handoffMarkdown(data, inventory);
  assert.match(html, /Local qualification only/);
  assert.match(html, /LIVE_TESTNET/);
  assert.match(html, /not<\/strong> a production/);
  assert.match(handoff, /grant awarded/);
  assert.match(handoff, /HOSTED_CI/);
});
