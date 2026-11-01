import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const reportSchema = "fhec-cofhe-static-optimization-v1";
const metricFields = {
  operations: "expanded_encrypted_operations",
  calls: "expanded_helper_calls",
  depth: "max_helper_call_depth",
};

function isUnsignedSafeInteger(value) {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

export function nonnegativeInteger(value, label) {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) {
    throw new Error(`${label} must be a nonnegative integer in decimal notation`);
  }
  const number = Number(value);
  if (!Number.isSafeInteger(number)) {
    throw new Error(`${label} exceeds JavaScript safe integer range`);
  }
  return number;
}

export function evaluatePolicy(report, config) {
  if (!report || report.schema !== reportSchema || !Array.isArray(report.call_graph)) {
    throw new Error("Input is not an FHEC CoFHE static optimization report");
  }
  if (typeof config.entry !== "string" || !/^[A-Za-z_][A-Za-z_0-9]*$/.test(config.entry)) {
    throw new Error("A valid --entry function name is required");
  }
  if (!["baseline", "optimized"].includes(config.mode)) {
    throw new Error("--mode must be baseline or optimized");
  }
  if (!config.limits || Object.keys(config.limits).length === 0) {
    throw new Error("At least one resource limit is required");
  }
  for (const [key, limit] of Object.entries(config.limits)) {
    if (!(key in metricFields) || !isUnsignedSafeInteger(limit)) {
      throw new Error(`Invalid budget limit: ${key}`);
    }
  }
  const names = new Set();
  for (const functionInfo of report.call_graph) {
    if (!functionInfo || typeof functionInfo.name !== "string" || names.has(functionInfo.name)) {
      throw new Error("Invalid or repeated call-graph function name");
    }
    names.add(functionInfo.name);
  }
  const node = report.call_graph.find((item) => item.name === config.entry);
  if (!node) {
    throw new Error(`Unknown CoFHE entry function: ${config.entry}`);
  }
  const profile = node[config.mode];
  if (!profile || typeof profile !== "object") {
    throw new Error(`Missing ${config.mode} cost profile for ${config.entry}`);
  }
  const checks = Object.entries(config.limits).map(([metric, limit]) => {
    const measured = profile[metricFields[metric]];
    if (!isUnsignedSafeInteger(measured)) {
      throw new Error(`Invalid ${metric} profile metric for ${config.entry}`);
    }
    return {
      metric,
      metric_field: metricFields[metric],
      measured,
      maximum: limit,
      passed: measured <= limit,
    };
  });
  return {
    schema: "fhec-cofhe-cost-policy-v1",
    verdict: checks.every((check) => check.passed) ? "pass" : "reject",
    entry: config.entry,
    variant: config.mode,
    measurement: "static expanded call-graph estimates; not live gas or latency",
    checks,
    limitations: [
      "The model expands direct helper calls once per syntactic call site and evaluates encrypted selection arms eagerly.",
      "Passing does not prove a network gas upper bound or cryptographic runtime bound.",
      "Values larger than JavaScript safe integers must not be approved by this policy tool.",
    ],
  };
}

export function parseArguments(args) {
  const config = { mode: "optimized", limits: {} };
  const seen = new Set();
  const flags = {
    "--report": "report",
    "--entry": "entry",
    "--mode": "mode",
    "--max-ops": "operations",
    "--max-calls": "calls",
    "--max-depth": "depth",
    "--out": "out",
  };
  for (let index = 0; index < args.length; index += 2) {
    const flag = args[index];
    const value = args[index + 1];
    if (!(flag in flags) || seen.has(flag) || value === undefined || value.startsWith("--")) {
      throw new Error("Usage: node check-cost-policy.mjs --report <path> --entry <name> [--mode baseline|optimized] --max-ops <n> [--max-calls <n>] [--max-depth <n>] [--out <file>]");
    }
    seen.add(flag);
    const field = flags[flag];
    if (["operations", "calls", "depth"].includes(field)) {
      config.limits[field] = nonnegativeInteger(value, flag);
    } else {
      config[field] = value;
    }
  }
  if (!config.report || !config.entry || Object.keys(config.limits).length === 0) {
    throw new Error("--report, --entry and at least one --max-* limit are required");
  }
  if (!["baseline", "optimized"].includes(config.mode)) {
    throw new Error("--mode must be baseline or optimized");
  }
  return config;
}

export function execute(args) {
  const config = parseArguments(args);
  const report = JSON.parse(readFileSync(config.report, "utf8"));
  const result = evaluatePolicy(report, config);
  const evidence = `${JSON.stringify(result, null, 2)}\n`;
  if (config.out) {
    if (resolve(config.out) === resolve(config.report)) {
      throw new Error("Budget output path must differ from the source report");
    }
    writeFileSync(config.out, evidence, { flag: "wx" });
  }
  process.stdout.write(evidence);
  return result.verdict === "pass" ? 0 : 3;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    process.exitCode = execute(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`CoFHE cost policy error: ${error.message}\n`);
    process.exitCode = 2;
  }
}
