import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createMatrix, widths, operations } from "../phase6/oracle.mjs";
import { canonicalHash } from "../phase7/metrics.mjs";

const variantNames = Object.freeze({
  optimized: "FHECPhase6Live",
  tuned: "FHECPhase7Tuned",
  straight: "FHECPhase7Straight",
});
const profiles = Object.freeze(["smoke6", "coverage24", "comparison72"]);
const variantModes = Object.freeze(["optimized-tuned", "three-way"]);

export { profiles, variantModes };

export function createPlan(profile = "smoke6", variantMode = "optimized-tuned") {
  if (!profiles.includes(profile) || !variantModes.includes(variantMode)) {
    throw new Error("Unknown Phase 8/9 live profile or comparison mode");
  }
  const source = createMatrix(28);
  const widthsToUse = profile === "smoke6" ? [8] : widths;
  const selectedIndices = profile === "comparison72" ? [3, 7, 17] : profile === "coverage24" ? [17] : null;
  const selectedVariants = variantMode === "three-way" ? ["optimized", "tuned", "straight"] : ["optimized", "tuned"];
  const cases = [];
  for (const width of widthsToUse) {
    for (let operationIndex = 0; operationIndex < operations.length; operationIndex += 1) {
      const operation = operations[operationIndex];
      const indices = selectedIndices ?? [[3], [7], [17], [12], [25], [6]][operationIndex];
      for (const index of indices) {
        const value = source.cases.find((entry) => entry.width === width && entry.operation === operation && entry.id.endsWith("-" + String(index).padStart(2, "0")));
        if (!value) throw new Error("Reference corpus missing canonical input pair");
        cases.push({
          id: value.id,
          width,
          operation,
          operation_selector: operationIndex,
          input_a: value.a,
          input_b: value.b,
          expected: value.expected,
        });
      }
    }
  }
  return {
    schema: "fhec-cofhe-phase89-live-plan-v1",
    chain: "arb-sepolia",
    chain_id: 421614,
    profile,
    variant_mode: variantMode,
    variants: selectedVariants.map((name) => ({ name, contract: variantNames[name] })),
    case_count: cases.length,
    deployment_count: selectedVariants.length,
    execution_count: cases.length * selectedVariants.length,
    total_planned_transactions: (cases.length + 1) * selectedVariants.length,
    cases,
    disclosure: {
      input_values_are_public_demo_vectors: true,
      fee_ceiling_covers_only_native_chain_transactions: true,
      offchain_fhe_task_fees: "unverified; no cap or estimate asserted",
      result_verification: "SDK decryptForView matched to independent plaintext oracle; not independently cryptographically attested",
      no_live_transactions_without_separate_explicit_consent: true,
    },
  };
}

export function validatePlan(value) {
  if (!value || !profiles.includes(value.profile) || !variantModes.includes(value.variant_mode)) {
    throw new Error("Invalid Phase 8/9 plan profile");
  }
  const canonical = createPlan(value.profile, value.variant_mode);
  if (JSON.stringify(canonical) !== JSON.stringify(value)) {
    throw new Error("Live plan differs from deterministic canonical oracle");
  }
  return value;
}

export function planDigest(plan) {
  return canonicalHash(validatePlan(plan));
}

export function loadPlan(path) {
  return validatePlan(JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, "")));
}

export function main(args) {
  if (args.length !== 6 || args[0] !== "--profile" || args[2] !== "--variants" || args[4] !== "--out") {
    throw new Error("Usage: --profile smoke6|coverage24|comparison72 --variants optimized-tuned|three-way --out <plan.json>");
  }
  const plan = createPlan(args[1], args[3]);
  writeFileSync(args[5], JSON.stringify(plan, null, 2) + "\n", { flag: "wx" });
  process.stdout.write("Phase 8/9 plan: " + plan.case_count + " oracle cases, " + plan.execution_count + " executions, " + plan.deployment_count + " deployments, " + planDigest(plan) + "\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 8/9 plan failed: " + error.message + "\n"); process.exitCode = 1; }
}
