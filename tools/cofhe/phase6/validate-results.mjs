import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assertMatrix, expected, operations, widths } from "./oracle.mjs";

function readJson(path) {
  const bytes = readFileSync(path, "utf8");
  const data = JSON.parse(bytes.startsWith("\uFEFF") ? bytes.slice(1) : bytes);
  if (!data || typeof data !== "object" || Array.isArray(data)) throw new Error("Invalid JSON document");
  return data;
}

export function validateMock(matrix, result) {
  assertMatrix(matrix);
  if (!result || result.schema !== "fhec-cofhe-phase6-mock-coverage-v1" || !Array.isArray(result.records)) {
    throw new Error("Invalid Phase 6 mock result schema");
  }
  if (result.environment !== "Fhenix Hardhat mock, NOT actual FHE coprocessor" ||
      result.family_count !== 24 || result.cases_per_family !== matrix.cases_per_family ||
      result.completed_cases !== matrix.total_cases || result.records.length !== matrix.total_cases) {
    throw new Error("Mock evidence is incomplete or mislabelled");
  }
  const byId = new Map();
  for (const record of result.records) {
    if (!record || typeof record.id !== "string" || byId.has(record.id) ||
        typeof record.measured_mock_gas !== "string" || !/^[1-9][0-9]*$/.test(record.measured_mock_gas) ||
        typeof record.elapsed_wall_ms_including_mock_receipt !== "string" ||
        !/^[0-9]+$/.test(record.elapsed_wall_ms_including_mock_receipt)) {
      throw new Error("Duplicate, malformed or zero-gas mock evidence record");
    }
    byId.set(record.id, record);
  }
  for (const testCase of matrix.cases) {
    const value = byId.get(testCase.id);
    if (!value || value.width !== testCase.width || value.operation !== testCase.operation ||
        value.expected !== testCase.expected) {
      throw new Error(`Mock evidence is missing or incorrect: ${testCase.id}`);
    }
    const independent = expected(testCase.width, testCase.operation, BigInt(testCase.a), BigInt(testCase.b));
    const canonical = typeof independent === "boolean" ? independent : independent.toString();
    if (value.expected !== canonical) throw new Error(`Independent oracle conflict: ${testCase.id}`);
  }
  return { schema: "fhec-cofhe-phase6-validated-mock-evidence-v1", tier: "verified-local-mock",
    completed_cases: matrix.total_cases, distinct_families: 24, encrypted_runtime_proven: false,
    limitations: ["No cryptographic FHE task completion is proven", "No real network gas is measured"] };
}

export function validateLive(result) {
  if (!result || result.schema !== "fhec-cofhe-phase6-live-testnet-v1" || result.chain_id !== 421614 ||
      !Array.isArray(result.records) || result.records.length !== 3 || result.completed_encrypted_cases !== 3 ||
      !/^0x[0-9a-fA-F]{40}$/.test(result.contract) || !/^0x[0-9a-fA-F]{40}$/.test(result.signer) ||
      !/^0x[0-9a-fA-F]{64}$/.test(result.deployment_transaction)) {
    throw new Error("Malformed or incomplete Phase 6 live SDK trace");
  }
  const map = new Map([
    ["maximum-u8", { operation: "max", a: 19n, b: 41n }],
    ["addition-wrap-u8", { operation: "add", a: 255n, b: 1n }],
    ["greater-u8", { operation: "gt", a: 241n, b: 12n }],
  ]);
  const observed = new Set();
  for (const record of result.records) {
    const oracle = map.get(record.label);
    if (!oracle || observed.has(record.label)) throw new Error("Unknown or repeated live test case");
    observed.add(record.label);
    const independent = String(expected(8, oracle.operation, oracle.a, oracle.b));
    if (record.input_a !== String(oracle.a) || record.input_b !== String(oracle.b) ||
        record.expected !== independent || record.actual !== independent ||
        record.matched_independent_oracle !== true ||
        !/^0x[0-9a-fA-F]{64}$/.test(record.transaction_hash) ||
        !/^[1-9][0-9]*$/.test(record.transaction_gas_used) ||
        !Number.isSafeInteger(record.block_number) || record.block_number <= 0) {
      throw new Error(`SDK trace does not establish the expected result: ${record.label}`);
    }
  }
  return {
    schema: "fhec-cofhe-phase6-provisional-live-evidence-v1",
    tier: "provisional-sdk-trace-awaiting-external-receipt-check",
    chain_id: 421614,
    test_count: observed.size,
    transaction_hashes: result.records.map((record) => record.transaction_hash),
    live_fhe_evidence_independently_verified: false,
    note: "Re-run Phase 5 RPC receipt checks and obtain independently reviewed ciphertext/ACL/task-completion proofs before elevating this claim.",
  };
}

export function main(args) {
  if (args.length !== 5 && args.length !== 4) throw new Error("Usage: --mock <matrix.json> <results.json> --out <verified.json> OR --live <results.json> --out <provisional.json>");
  let result;
  let out;
  if (args[0] === "--mock" && args.length === 5 && args[3] === "--out") {
    result = validateMock(readJson(args[1]), readJson(args[2]));
    out = args[4];
  } else if (args[0] === "--live" && args.length === 4 && args[2] === "--out") {
    result = validateLive(readJson(args[1]));
    out = args[3];
  } else {
    throw new Error("Invalid Phase 6 evidence verification arguments");
  }
  writeFileSync(out, `${JSON.stringify(result, null, 2)}\n`, { flag: "wx" });
  process.stdout.write(`Phase 6 evidence validated with tier ${result.tier}.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write(`Phase 6 evidence validation failed: ${error.message}\n`); process.exitCode = 1; }
}
