import assert from "node:assert/strict";
import { test } from "node:test";
import { median, p95, percentageChange, summarize } from "./summarize-phase4.mjs";

function fixture() {
  const widths = [8, 16, 32, 64];
  const operations = ["add", "sub", "mul", "gt", "max", "repeated"];
  const samples = [];
  for (const width of widths) {
    for (const operation of operations) {
      samples.push({
        width,
        operation,
        vector_id: `add-u${width}-${operation}`,
        operands: { a: "8", b: "3" },
        expected: operation === "gt" ? true :
          operation === "add" ? "11" :
          operation === "sub" ? "5" :
          operation === "mul" ? "24" :
          operation === "max" ? "8" : "48",
        mock_gas: { baseline: "100000", optimized: "90000", manual: "88000" },
      });
    }
  }
  return {
    schema: "fhec-cofhe-phase4-mock-samples-v1",
    cases_per_family: 1,
    family_count: 24,
    expected_comparisons: 24,
    completed_comparisons: 24,
    samples,
  };
}

test("median preserves half-gas units exactly", () => {
  assert.equal(median([2n, 3n]), "2.5");
  assert.equal(median([1n, 9n, 3n]), "3");
});

test("p95 chooses the nearest observed rank", () => {
  assert.equal(p95([1n, 2n, 3n, 4n, 100n]), "100");
  assert.equal(p95([3n]), "3");
});

test("percentages stay signed and stable", () => {
  assert.equal(percentageChange("10000", "9000"), "10.00");
  assert.equal(percentageChange("10000", "12000"), "-20.00");
  assert.equal(percentageChange("10000.5", "9000.5"), "9.99");
  assert.equal(percentageChange("0", "1"), null);
});

test("summarizes 24 independent width-and-operation families", () => {
  const result = summarize(fixture());
  assert.equal(result.total_cases, 24);
  assert.equal(result.families.length, 24);
  assert.equal(result.families[0].optimized_vs_baseline_median_reduction_percent, "10.00");
  assert.equal(result.families[0].optimized_vs_manual_median_reduction_percent, "-2.27");
});

test("rejects incomplete and duplicate evidence", () => {
  const payload = fixture();
  payload.samples.pop();
  assert.throws(() => summarize(payload), /incomplete or inconsistent/);
  const duplicated = fixture();
  duplicated.samples[1] = structuredClone(duplicated.samples[0]);
  assert.throws(() => summarize(duplicated), /Repeated evidence input/);
});

test("rejects mismatched independent arithmetic oracle", () => {
  const payload = fixture();
  payload.samples[0].expected = "13";
  assert.throws(() => summarize(payload), /Independent oracle mismatch/);
});

test("rejects nondecimal gas measurements", () => {
  const payload = fixture();
  payload.samples[0].mock_gas.optimized = "-100";
  assert.throws(() => summarize(payload), /unsigned decimal gas/);
});

test("rejects an unknown report schema", () => {
  assert.throws(() => summarize({ ...fixture(), schema: "fake" }), /Unrecognized/);
});
