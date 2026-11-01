import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { corpusDigest, digest, loadCorpus } from "./oracle.mjs";

const unsigned = /^(0|[1-9][0-9]*)$/;

function bytes(path) {
  return readFileSync(path);
}

function milliseconds(value) {
  return Number.isSafeInteger(value) && value >= 0 && value <= 600000;
}

function median(data) {
  const values = data.map(BigInt).sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
  return values.length % 2 === 1 ? values[Math.floor(values.length / 2)] :
    (values[values.length / 2 - 1] + values[values.length / 2]) / 2n;
}

export function checkMockResults(corpus, raw, sourceSealBytes, corpusFileBytes) {
  const expectedHash = corpusDigest(corpus);
  if (!raw || raw.schema !== "fhec-cofhe-phase10-mock-observations-v1" ||
      raw.evidence_tier !== "locally-executed-hardhat-cofhe-mocks-not-live-network" ||
      raw.corpus_sha256 !== expectedHash ||
      raw.corpus_file_sha256 !== digest(corpusFileBytes) ||
      raw.source_seal_file_sha256 !== digest(sourceSealBytes) ||
      raw.cases_expected !== corpus.case_count || raw.cases_recorded !== corpus.case_count ||
      raw.full_corpus_completed !== true ||
      JSON.stringify(raw.variants) !== JSON.stringify(corpus.variants) ||
      !Array.isArray(raw.records) || raw.records.length !== corpus.case_count) {
    throw new Error("Phase 10 mock evidence does not cover the exact source-sealed independent oracle corpus");
  }
  const byFamily = new Map();
  for (let index = 0; index < corpus.cases.length; index += 1) {
    const expected = corpus.cases[index];
    const observed = raw.records[index];
    if (!observed || observed.id !== expected.id || observed.width !== expected.width ||
        observed.operation !== expected.operation || observed.expected !== String(expected.expected) ||
        !observed.variants || JSON.stringify(Object.keys(observed.variants)) !== JSON.stringify(corpus.variants)) {
      throw new Error("Phase 10 mock case ordering or expected value mismatch at " + expected.id);
    }
    const key = `${expected.operation}/u${expected.width}`;
    if (!byFamily.has(key)) byFamily.set(key, Object.fromEntries(corpus.variants.map((v) => [v, []])));
    for (const name of corpus.variants) {
      const result = observed.variants[name];
      if (!result || result.actual !== String(expected.expected) || result.oracle_match !== true ||
          typeof result.mock_transaction_gas !== "string" || !unsigned.test(result.mock_transaction_gas) ||
          BigInt(result.mock_transaction_gas) === 0n || !milliseconds(result.sdk_encrypt_ms) ||
          !milliseconds(result.mock_execution_and_decryption_ms)) {
        throw new Error("Phase 10 mock execution differs from independent oracle: " + expected.id + "/" + name);
      }
      byFamily.get(key)[name].push(result);
    }
  }
  if (byFamily.size !== corpus.families) throw new Error("Phase 10 did not exercise all 24 CoFHE type/operation families");
  const families = [];
  for (const [family, variants] of byFamily) {
    const stats = {};
    for (const [name, records] of Object.entries(variants)) {
      const gas = records.map((record) => record.mock_transaction_gas);
      stats[name] = {
        sample_count: gas.length,
        median_mock_gas: median(gas).toString(),
        total_mock_gas: gas.reduce((a, v) => a + BigInt(v), 0n).toString(),
        minimum_mock_gas: gas.reduce((a, v) => BigInt(v) < a ? BigInt(v) : a, BigInt(gas[0])).toString(),
        maximum_mock_gas: gas.reduce((a, v) => BigInt(v) > a ? BigInt(v) : a, BigInt(gas[0])).toString(),
        median_sdk_encrypt_ms: median(records.map((record) => String(record.sdk_encrypt_ms))).toString(),
      };
    }
    families.push({ family, cases: corpus.cases_per_family, variants: stats });
  }
  return {
    schema: "fhec-cofhe-phase10-validated-mock-results-v1",
    evidence_tier: "independently-recomputed-local-mock-differential-results",
    corpus_sha256: expectedHash,
    raw_evidence_sha256: digest(JSON.stringify(raw)),
    source_seal_file_sha256: raw.source_seal_file_sha256,
    cases: corpus.case_count,
    variants: corpus.variants,
    successful_mock_executions: corpus.case_count * corpus.variants.length,
    operation_width_families: corpus.families,
    negative_consuming_contract_test_required: true,
    families,
    limitations: [
      "All gas readings are Hardhat CoFHE mock transactions, not Arbitrum Sepolia receipts.",
      "Passing matches sampled inputs against an independent arithmetic oracle, not all input space.",
      "No cryptographic coprocessor execution, protocol fees, testnet deployment or wallet use is implied.",
    ],
  };
}

export function main(args) {
  if (args.length !== 10 || args[0] !== "--corpus" || args[2] !== "--raw" ||
      args[4] !== "--seal" || args[6] !== "--out" || args[8] !== "--negative-confirmed") {
    throw new Error("Usage: --corpus <json> --raw <json> --seal <source-seal> --out <new-validation> --negative-confirmed true");
  }
  if (args[9] !== "true") throw new Error("Explicit Hardhat negative consuming-contract test must pass");
  const result = checkMockResults(loadCorpus(args[1]), JSON.parse(bytes(args[3]).toString("utf8").replace(/^\uFEFF/, "")), bytes(args[5]), bytes(args[1]));
  writeFileSync(args[7], JSON.stringify(result, null, 2) + "\n", { flag: "wx" });
  process.stdout.write(`Phase 10 validated ${result.successful_mock_executions} mock ciphertext executions across ${result.operation_width_families} families.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 10 independent result validation failed: " + error.message + "\n"); process.exitCode = 1; }
}
