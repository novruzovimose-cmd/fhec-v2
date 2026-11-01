import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { evaluatePolicy } from "./check-cost-policy.mjs";

export function verifyContrast(report, entry = "repeated_u8") {
  const limits = { operations: 2, calls: 0, depth: 0 };
  const baseline = evaluatePolicy(report, { entry, mode: "baseline", limits });
  const optimized = evaluatePolicy(report, { entry, mode: "optimized", limits });
  const baselineOps = baseline.checks.find((item) => item.metric === "operations");
  const optimizedOps = optimized.checks.find((item) => item.metric === "operations");
  if (baseline.verdict !== "reject" || optimized.verdict !== "pass" ||
      baselineOps.measured !== 3 || optimizedOps.measured !== 2 ||
      !baseline.checks.filter((check) => check.metric !== "operations").every((check) => check.passed)) {
    throw new Error("Expected 3-to-2 encrypted-operation improvement not demonstrated by FHEC");
  }
  return {
    schema: "fhec-cofhe-phase4-policy-contrast-v1",
    entry,
    fixed_budget: limits,
    baseline,
    optimized,
    verified_improvement: "3 baseline expanded encrypted operations to 2 optimized expanded encrypted operations",
    warning: "A static operation count comparison is not a network gas or runtime speedup claim",
  };
}

export function main(args) {
  if (args.length !== 4 || args[0] !== "--report" || args[2] !== "--out") {
    throw new Error("Usage: node verify-phase4-contrast.mjs --report <report.json> --out <contrast.json>");
  }
  if (resolve(args[1]) === resolve(args[3])) throw new Error("Contrast output must differ from source report");
  const result = verifyContrast(JSON.parse(readFileSync(args[1], "utf8")));
  writeFileSync(args[3], `${JSON.stringify(result, null, 2)}\n`, { flag: "wx" });
  console.log("Verified Phase 4 static operation-budget contrast: baseline rejected; optimized accepted");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`Phase 4 contrast verification failed: ${error.message}\n`);
    process.exitCode = 1;
  }
}
