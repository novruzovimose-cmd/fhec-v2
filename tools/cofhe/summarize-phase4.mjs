import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const widths = [8, 16, 32, 64];
const operations = ["add", "sub", "mul", "gt", "max", "repeated"];
const variants = ["baseline", "optimized", "manual"];

function parseGas(input) {
  if (typeof input !== "string" || !/^[0-9]+$/.test(input)) {
    throw new Error("Expected an unsigned decimal gas string");
  }
  const gas = BigInt(input);
  if (gas <= 0n) {
    throw new Error("A successful transaction must use positive gas");
  }
  return gas;
}

export function median(values) {
  if (!values.length) throw new Error("Cannot summarize an empty gas sample");
  const sorted = [...values].sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
  const center = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) return sorted[center].toString();
  const sum = sorted[center - 1] + sorted[center];
  return sum % 2n === 0n ? (sum / 2n).toString() : `${sum / 2n}.5`;
}

export function p95(values) {
  if (!values.length) throw new Error("Cannot summarize an empty gas sample");
  const sorted = [...values].sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
  return sorted[Math.ceil(sorted.length * 0.95) - 1].toString();
}

export function percentageChange(original, improved) {
  function inHalfUnits(input) {
    if (typeof input !== "string" || !/^[0-9]+(\.5)?$/.test(input)) {
      throw new Error("Expected a decimal median mock-gas value");
    }
    const half = input.endsWith(".5");
    return BigInt(half ? input.slice(0, -2) : input) * 2n + (half ? 1n : 0n);
  }
  const a = inHalfUnits(original);
  const b = inHalfUnits(improved);
  if (a === 0n) return null;
  const difference = a - b;
  const hundredths = (difference * 10000n) / a;
  const negative = hundredths < 0n;
  const positive = negative ? -hundredths : hundredths;
  return `${negative ? "-" : ""}${positive / 100n}.${String(positive % 100n).padStart(2, "0")}`;
}

export function summarize(payload) {
  if (!payload || payload.schema !== "fhec-cofhe-phase4-mock-samples-v1" || !Array.isArray(payload.samples)) {
    throw new Error("Unrecognized Phase 4 mock sample schema");
  }
  if (!Number.isInteger(payload.cases_per_family) || payload.cases_per_family < 1 || payload.cases_per_family > 8) {
    throw new Error("Invalid Phase 4 per-family sample count");
  }
  if (payload.family_count !== widths.length * operations.length ||
      payload.expected_comparisons !== widths.length * operations.length * payload.cases_per_family ||
      payload.completed_comparisons !== payload.expected_comparisons ||
      payload.samples.length !== payload.expected_comparisons) {
    throw new Error("Phase 4 evidence is incomplete or inconsistent");
  }
  const families = [];
  const byKey = new Map();
  const ids = new Set();
  for (const item of payload.samples) {
    if (!widths.includes(item.width) || !operations.includes(item.operation) ||
        !item.vector_id || !item.operands || !item.mock_gas ||
        typeof item.operands.a !== "string" || typeof item.operands.b !== "string") {
      throw new Error("Malformed Phase 4 sample");
    }
    const width = BigInt(item.width);
    const max = (1n << width) - 1n;
    if (!/^[0-9]+$/.test(item.operands.a) || !/^[0-9]+$/.test(item.operands.b)) {
      throw new Error("Malformed sample operand");
    }
    const a = BigInt(item.operands.a);
    const b = BigInt(item.operands.b);
    if (a > max || b > max) throw new Error("Encrypted operand exceeds its declared width");
    let expected;
    switch (item.operation) {
      case "add": expected = ((a + b) & max).toString(); break;
      case "sub": expected = ((a - b) & max).toString(); break;
      case "mul": expected = ((a * b) & max).toString(); break;
      case "gt": expected = a > b; break;
      case "max": expected = (a > b ? a : b).toString(); break;
      case "repeated": expected = ((2n * a * b) & max).toString(); break;
    }
    if (item.expected !== expected) throw new Error(`Independent oracle mismatch: ${item.vector_id}`);
    const unique = `${item.width}/${item.operation}/${item.vector_id}`;
    if (ids.has(unique)) throw new Error(`Repeated evidence input: ${unique}`);
    ids.add(unique);
    for (const variant of variants) parseGas(item.mock_gas[variant]);
    const key = `${item.width}/${item.operation}`;
    if (!byKey.has(key)) byKey.set(key, []);
    byKey.get(key).push(item);
  }

  for (const width of widths) {
    for (const operation of operations) {
      const key = `${width}/${operation}`;
      const rows = byKey.get(key);
      if (!rows || rows.length !== payload.cases_per_family) {
        throw new Error(`Incomplete differential family ${key}`);
      }
      const stats = {};
      for (const variant of variants) {
        const gas = rows.map((row) => parseGas(row.mock_gas[variant]));
        stats[variant] = { median_mock_gas: median(gas), p95_mock_gas: p95(gas) };
      }
      families.push({
        width,
        operation,
        sample_count: rows.length,
        variants: stats,
        optimized_vs_baseline_median_reduction_percent: percentageChange(stats.baseline.median_mock_gas, stats.optimized.median_mock_gas),
        optimized_vs_manual_median_reduction_percent: percentageChange(stats.manual.median_mock_gas, stats.optimized.median_mock_gas),
      });
    }
  }
  return {
    schema: "fhec-cofhe-phase4-gas-summary-v1",
    environment: "Fhenix Hardhat mock (not live network gas)",
    total_cases: payload.samples.length,
    families,
    interpretation: [
      "Only local Hardhat mock transaction gas is measured.",
      "Each family is reported separately. No unweighted cross-operation speedup is claimed.",
      "Manual Solidity provides an explicit independent performance comparison.",
      "Measurement includes contract wrapper overhead; it does not isolate FHE coprocessor fees.",
    ],
  };
}

export function main(args) {
  if (args.length !== 4 || args[0] !== "--input" || args[2] !== "--out") {
    throw new Error("Usage: node summarize-phase4.mjs --input <samples.json> --out <summary.json>");
  }
  if (resolve(args[1]) === resolve(args[3])) throw new Error("Summary must not overwrite input evidence");
  const result = summarize(JSON.parse(readFileSync(args[1], "utf8")));
  writeFileSync(args[3], `${JSON.stringify(result, null, 2)}\n`, { flag: "wx" });
  process.stdout.write(`Phase 4 mock summary: ${result.total_cases} verified comparisons across ${result.families.length} families\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`Phase 4 summary error: ${error.message}\n`);
    process.exitCode = 1;
  }
}
