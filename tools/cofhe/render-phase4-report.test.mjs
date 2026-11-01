import assert from "node:assert/strict";
import { test } from "node:test";
import { htmlEscape, render } from "./render-phase4-report.mjs";

function summary() {
  const widths = [8, 16, 32, 64];
  const operations = ["add", "sub", "mul", "gt", "max", "repeated"];
  return {
    schema: "fhec-cofhe-phase4-gas-summary-v1",
    total_cases: 48,
    families: widths.flatMap((width) => operations.map((operation) => ({
      width,
      operation,
      sample_count: 2,
      variants: {
        baseline: { median_mock_gas: "100000.5", p95_mock_gas: "103000" },
        optimized: { median_mock_gas: "90000.5", p95_mock_gas: "93000" },
        manual: { median_mock_gas: "88000.5", p95_mock_gas: "91000" },
      },
      optimized_vs_baseline_median_reduction_percent: "9.99",
      optimized_vs_manual_median_reduction_percent: "-2.27",
    }))),
  };
}

test("escapes all HTML-sensitive characters", () => {
  assert.equal(htmlEscape(`<script>alert('x') & "unsafe"</script>`), "&lt;script&gt;alert(&#39;x&#39;) &amp; &quot;unsafe&quot;&lt;/script&gt;");
});

test("renders 24 width-aware rows with explicit mock warning", () => {
  const html = render(summary());
  assert.equal((html.match(/<tr>/g) ?? []).length, 25);
  assert.match(html, /NOT LIVE CoFHE EXECUTION/);
  assert.match(html, /-2\.27%/);
  assert.match(html, /Content-Security-Policy/);
  assert.doesNotMatch(html, /<script/);
});

test("rejects incomplete summary", () => {
  const invalid = summary();
  invalid.families.pop();
  assert.throws(() => render(invalid), /Invalid or incomplete/);
});

test("rejects injected nonnumeric gas measurements", () => {
  const invalid = summary();
  invalid.families[0].variants.optimized.median_mock_gas = "<script>";
  assert.throws(() => render(invalid), /Malformed gas measurement/);
});
