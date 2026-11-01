import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { median, percentile } from "../phase7/metrics.mjs";
import { hashBytes } from "./source-seal.mjs";
import { loadPlan, planDigest } from "./plan.mjs";
import { parseJson, uintString, validateSdkTrace } from "./verify-chain.mjs";

function summary(values) {
  if (!values.length) throw new Error("Missing real transaction samples");
  const ordered = values.map(BigInt).sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
  return {
    count: ordered.length,
    minimum: ordered[0].toString(),
    median: median(ordered).toString(),
    maximum: ordered.at(-1).toString(),
    p95: ordered.length >= 5 ? percentile(ordered, 95).toString() : null,
  };
}

export function analyzeLive(plan, raw, attestation) {
  validateSdkTrace(plan, raw);
  if (!attestation || attestation.schema !== "fhec-cofhe-phase89-rpc-attestation-v1" ||
      attestation.evidence_tier !== "arbitrum-sepolia-receipt-corroborated-sdk-decryption-not-independently-proven" ||
      attestation.chain_id !== 421614 ||
      attestation.raw_trace_sha256 !== hashBytes(JSON.stringify(raw)) ||
      attestation.plan_sha256 !== planDigest(plan) ||
      attestation.source_seal_sha256 !== raw.source_seal_sha256 ||
      attestation.coprocessor_fees_measured !== false ||
      attestation.native_chain_fees_paid_wei !== raw.native_chain_fees_paid_wei ||
      attestation.verified_transaction_count !== plan.total_planned_transactions ||
      attestation.execution_count !== plan.execution_count ||
      attestation.deployment_count !== plan.deployment_count ||
      !Array.isArray(attestation.receipts) || attestation.receipts.length !== plan.total_planned_transactions) {
    throw new Error("Independent RPC attestation does not cover exactly the live SDK trace");
  }
  const receipts = new Map();
  for (const value of attestation.receipts) {
    if (!value || typeof value.transaction_hash !== "string" || receipts.has(value.transaction_hash.toLowerCase()) ||
        !["deploy", "execute"].includes(value.kind) ||
        typeof value.confirmations !== "string" || BigInt(value.confirmations) < BigInt(attestation.minimum_confirmations)) {
      throw new Error("Duplicate or invalid attested on-chain receipt");
    }
    receipts.set(value.transaction_hash.toLowerCase(), value);
  }
  for (const deploy of raw.deployments) {
    const observed = receipts.get(deploy.transaction_hash.toLowerCase());
    if (!observed || observed.kind !== "deploy" || observed.native_chain_fee_wei !== deploy.native_chain_fee_wei) {
      throw new Error("Missing corroborated deployment fee");
    }
  }
  const families = new Map();
  const expectedFamilies = new Set(plan.cases.map((item) => item.operation + "/u" + item.width));
  let executionTotal = 0n;
  for (const record of raw.records) {
    const family = record.operation + "/u" + record.width;
    if (!families.has(family)) families.set(family, []);
    families.get(family).push(record);
    for (const variant of plan.variants) {
      const value = record.variants[variant.name];
      const observed = receipts.get(value.transaction_hash.toLowerCase());
      if (!observed || observed.kind !== "execute" || observed.native_chain_fee_wei !== value.native_chain_fee_wei) {
        throw new Error("Missing corroborated execution transaction fee");
      }
      executionTotal += uintString(value.native_chain_fee_wei, "execution native fees");
    }
  }
  if (families.size !== expectedFamilies.size || [...families.keys()].some((key) => !expectedFamilies.has(key))) {
    throw new Error("Measured execution family coverage does not match the approved live plan");
  }
  const rows = [];
  let tunedWins = 0;
  let tunedRegressions = 0;
  for (const [family, records] of families) {
    const variantStats = {};
    for (const variant of plan.variants) {
      const name = variant.name;
      const values = records.map((record) => record.variants[name]);
      variantStats[name] = {
        transaction_gas: summary(values.map((item) => item.gas_used)),
        native_transaction_fee_wei: summary(values.map((item) => item.native_chain_fee_wei)),
        sdk_encryption_ms: summary(values.map((item) => String(item.sdk_encrypt_ms))),
        confirmation_ms: summary(values.map((item) => String(item.confirmation_ms))),
        sdk_decryption_wait_ms: summary(values.map((item) => String(item.sdk_decrypt_wait_ms))),
      };
    }
    const generated = BigInt(variantStats.optimized.transaction_gas.median);
    const tuned = BigInt(variantStats.tuned.transaction_gas.median);
    if (generated < tuned) tunedWins += 1;
    if (generated > tuned) tunedRegressions += 1;
    rows.push({
      family,
      cases_per_variant: records.length,
      evidence_strength: records.length >= 5 ? "limited-observational" : "exploratory-too-few-repeats-for-reliable-p95",
      optimized_vs_tuned: generated < tuned ? "lower-gas" : generated > tuned ? "higher-gas" : "equal-gas",
      exact_median_gas_difference_tuned_minus_optimized: (tuned - generated).toString(),
      variants: variantStats,
    });
  }
  const deploymentTotal = raw.deployments.reduce((sum, value) => sum + uintString(value.native_chain_fee_wei, "deployment fee"), 0n);
  if (executionTotal + deploymentTotal !== uintString(raw.native_chain_fees_paid_wei, "total fee")) {
    throw new Error("Phase 9 fee categories do not sum to total actual network receipts");
  }
  return {
    schema: "fhec-cofhe-phase9-real-testnet-comparison-v1",
    evidence_tier: "arbitrum-sepolia-transaction-fees-only-sdk-results-not-cryptographically-attested",
    chain_id: 421614,
    plan_sha256: planDigest(plan),
    trace_sha256: hashBytes(JSON.stringify(raw)),
    profile: plan.profile,
    variant_mode: plan.variant_mode,
    case_count: plan.case_count,
    family_count: families.size,
    successful_onchain_execution_receipts: plan.execution_count,
    successful_onchain_deploy_receipts: plan.deployment_count,
    optimized_lower_gas_families_vs_tuned: tunedWins,
    optimized_higher_gas_families_vs_tuned: tunedRegressions,
    total_native_chain_fees_wei: raw.native_chain_fees_paid_wei,
    deployment_native_chain_fees_wei: deploymentTotal.toString(),
    execution_native_chain_fees_wei: executionTotal.toString(),
    coprocessor_fee_wei: null,
    coprocessor_fees_measured: false,
    families: rows,
    caveats: [
      "These are Arbitrum Sepolia EVM transaction gas and effective native chain fees, not FHE coprocessor service fees.",
      "The SDK claimed decrypted output matches the independent oracle; cryptographic correctness was not independently audited.",
      "The experiment is a small, deterministic and order-dependent workload, not a randomized production performance study.",
      "P95 is intentionally omitted for fewer than five samples; even larger samples require careful independent replication.",
      "A single testnet run cannot predict mainnet gas, finality, fee market dynamics or security.",
    ],
  };
}

export function main(args) {
  if (args.length !== 8 || args[0] !== "--plan" || args[2] !== "--raw" || args[4] !== "--attestation" || args[6] !== "--out") {
    throw new Error("Usage: --plan <plan> --raw <sdk> --attestation <rpc> --out <comparison>");
  }
  const result = analyzeLive(loadPlan(args[1]), parseJson(args[3]), parseJson(args[5]));
  writeFileSync(args[7], JSON.stringify(result, null, 2) + "\n", { flag: "wx" });
  process.stdout.write("Phase 9 confirmed " + result.successful_onchain_execution_receipts + " real testnet execution receipts, without inventing coprocessor fees.\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 9 cost report failed: " + error.message + "\n"); process.exitCode = 1; }
}
