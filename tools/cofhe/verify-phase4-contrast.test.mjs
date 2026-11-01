import assert from "node:assert/strict";
import { test } from "node:test";
import { verifyContrast } from "./verify-phase4-contrast.mjs";

const fixture = {
  schema: "fhec-cofhe-static-optimization-v1",
  call_graph: [{
    name: "repeated_u8",
    baseline: { expanded_encrypted_operations: 3, expanded_helper_calls: 0, max_helper_call_depth: 0 },
    optimized: { expanded_encrypted_operations: 2, expanded_helper_calls: 0, max_helper_call_depth: 0 },
  }],
};

test("accepts verified improvement and preserves both policy decisions", () => {
  const result = verifyContrast(fixture);
  assert.equal(result.baseline.verdict, "reject");
  assert.equal(result.optimized.verdict, "pass");
  assert.equal(result.fixed_budget.operations, 2);
});

test("rejects unoptimized output that wrongly passes", () => {
  const wrong = structuredClone(fixture);
  wrong.call_graph[0].baseline.expanded_encrypted_operations = 2;
  assert.throws(() => verifyContrast(wrong), /improvement not demonstrated/);
});

test("rejects optimized output exceeding budget", () => {
  const wrong = structuredClone(fixture);
  wrong.call_graph[0].optimized.expanded_encrypted_operations = 3;
  assert.throws(() => verifyContrast(wrong), /improvement not demonstrated/);
});
