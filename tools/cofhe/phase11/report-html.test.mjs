import assert from "node:assert/strict";
import { it } from "node:test";
import { render } from "./report-html.mjs";

const mocked = () => ({
  schema: "fhec-cofhe-phase10-validated-mock-results-v1",
  evidence_tier: "independently-recomputed-local-mock-differential-results",
  cases: 48, successful_mock_executions: 96, operation_width_families: 24,
  variants: ["optimized", "tuned"],
  corpus_sha256: "A".repeat(64), raw_evidence_sha256: "B".repeat(64), source_seal_file_sha256: "C".repeat(64),
  limitations: ["not live network"],
  families: Array.from({ length: 24 }, (_, index) => ({
    family: "family_" + index, cases: 2,
    variants: {
      optimized: { sample_count: 2, median_mock_gas: "123" },
      tuned: { sample_count: 2, median_mock_gas: "456" },
    },
  })),
});

it("renders an offline clearly labeled mock report with complete family coverage", () => {
  const html = render(mocked());
  assert.match(html, /LOCAL MOCKS ONLY/);
  assert.match(html, /family_23/);
  assert.match(html, /not live network/);
  assert.match(html, /Arbitrum Sepolia/);
  assert.match(html, /mock transaction gas units/);
});

it("escapes malicious report fields and refuses unsupported evidence tiers", () => {
  const sample = mocked();
  sample.families[0].family = `<img src=x onerror=alert(1)>`;
  sample.limitations[0] = `<script>alert(7)</script>`;
  const html = render(sample);
  assert.doesNotMatch(html, /<script>alert/);
  assert.doesNotMatch(html, /<img src=x/);
  assert.match(html, /&lt;script&gt;/);
  sample.evidence_tier = "live-network-proven";
  assert.throws(() => render(sample), /refuses/);
});
