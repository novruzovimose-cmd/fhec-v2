import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { createPlan, profiles, variantModes } from "./plan.mjs";
import { verifyFixtures } from "./verify-plans.mjs";

test("all six canonical live profiles remain reproducible", () => {
  const verified = verifyFixtures();
  assert.equal(verified.length, 6);
  const planned = [14, 21, 50, 75, 146, 219];
  assert.deepEqual(verified.map((entry) => entry.transactions), planned);
  for (const profile of profiles) {
    for (const variantMode of variantModes) {
      const plan = createPlan(profile, variantMode);
      assert.equal(plan.total_planned_transactions, plan.deployment_count + plan.execution_count);
      assert.equal(plan.cases.length, plan.case_count);
      assert.ok(plan.cases.every((item) => item.input_a !== undefined && item.input_b !== undefined));
    }
  }
});

test("tampered plan fixture is rejected before any live transaction", () => {
  const temp = mkdtempSync(join(tmpdir(), "fhec-phase89-fixture-test-"));
  for (const profile of profiles) {
    for (const variantMode of variantModes) {
      const path = join(temp, `${profile}-${variantMode}.json`);
      writeFileSync(path, JSON.stringify(createPlan(profile, variantMode)) + "\n");
    }
  }
  assert.equal(verifyFixtures(temp).length, 6);
  const target = join(temp, "coverage24-three-way.json");
  const bad = JSON.parse(readFileSync(target, "utf8"));
  bad.cases[0].input_a = "0";
  writeFileSync(target, JSON.stringify(bad) + "\n");
  assert.throws(() => verifyFixtures(temp), /deterministic canonical oracle|changed/);
});
