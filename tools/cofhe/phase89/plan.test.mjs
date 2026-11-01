import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createPlan, planDigest, validatePlan } from "./plan.mjs";

describe("Phase 8/9 deterministic, explicitly bounded live plans", () => {
  it("smoke plan covers six independent u8 families with two variants", () => {
    const plan = createPlan();
    assert.equal(plan.case_count, 6);
    assert.equal(plan.execution_count, 12);
    assert.equal(plan.total_planned_transactions, 14);
    assert.equal(new Set(plan.cases.map((entry) => entry.operation)).size, 6);
    assert.deepEqual(plan.variants.map((v) => v.name), ["optimized", "tuned"]);
  });
  it("coverage and repetition expose exact transaction ceilings", () => {
    const coverage = createPlan("coverage24", "three-way");
    const repeated = createPlan("comparison72", "optimized-tuned");
    assert.equal(coverage.case_count, 24);
    assert.equal(coverage.total_planned_transactions, 75);
    assert.equal(repeated.case_count, 72);
    assert.equal(repeated.total_planned_transactions, 146);
    assert.equal(new Set(repeated.cases.map((entry) => entry.width)).size, 4);
  });
  it("oracle outputs are stable across regeneration and plan hashes", () => {
    const first = createPlan("comparison72", "three-way");
    const second = createPlan("comparison72", "three-way");
    assert.deepEqual(first, second);
    assert.equal(planDigest(first), planDigest(second));
    assert.match(planDigest(first), /^[0-9A-F]{64}$/);
  });
  it("rejects forged typed arithmetic, width and consent semantics", () => {
    const changed = createPlan();
    changed.cases[0].expected = "100";
    assert.throws(() => validatePlan(changed), /differs/);
    const other = createPlan();
    other.disclosure.no_live_transactions_without_separate_explicit_consent = false;
    assert.throws(() => validatePlan(other), /differs/);
  });
  it("rejects unsupported modes and no empty implicit fallback", () => {
    assert.throws(() => createPlan("auto", "optimized-tuned"), /Unknown/);
    assert.throws(() => createPlan("smoke6", "all"), /Unknown/);
  });
});
