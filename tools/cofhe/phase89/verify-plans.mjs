import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createPlan, planDigest, profiles, variantModes, validatePlan } from "./plan.mjs";

export const fixtureDirectory = join(dirname(fileURLToPath(import.meta.url)), "fixtures");

export function verifyFixtures(directory = fixtureDirectory) {
  const results = [];
  for (const profile of profiles) {
    for (const variantMode of variantModes) {
      const file = `${profile}-${variantMode}.json`;
      const actual = JSON.parse(readFileSync(join(directory, file), "utf8").replace(/^\uFEFF/, ""));
      validatePlan(actual);
      const canonical = createPlan(profile, variantMode);
      if (JSON.stringify(actual) !== JSON.stringify(canonical)) {
        throw new Error("Canonical plan fixture changed: " + file);
      }
      results.push({ file, cases: canonical.case_count, transactions: canonical.total_planned_transactions, plan_sha256: planDigest(actual) });
    }
  }
  return results;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const results = verifyFixtures();
    process.stdout.write(JSON.stringify({ schema: "fhec-phase89-plan-fixture-validation-v1", verified: results.length, fixtures: results }, null, 2) + "\n");
  } catch (error) {
    process.stderr.write("Phase 8/9 plan fixtures failed: " + error.message + "\n");
    process.exitCode = 1;
  }
}
