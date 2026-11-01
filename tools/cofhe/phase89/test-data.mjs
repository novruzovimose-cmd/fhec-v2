import { createPlan, planDigest } from "./plan.mjs";
import { hashBytes } from "./source-seal.mjs";

export function syntheticEvidence(profile = "smoke6") {
  const plan = createPlan(profile);
  let counter = 0;
  const fee = "20000000";
  const newTransaction = () => ({
    transaction_hash: "0x" + (++counter).toString(16).padStart(64, "0"),
    calldata_sha256: hashBytes(Buffer.from("ab123456", "hex")),
    block_number: 100 + counter,
    gas_used: "200000",
    effective_gas_price_wei: "100",
    native_chain_fee_wei: fee,
  });
  const deployments = plan.variants.map((variant, index) => ({
    variant: variant.name,
    contract: "0x" + (index + 10).toString(16).padStart(40, "0"),
    runtime_code_sha256: hashBytes(Buffer.from("6001600055", "hex")),
    confirmation_ms: 100,
    ...newTransaction(),
  }));
  const records = plan.cases.map((item) => ({
    id: item.id,
    width: item.width,
    operation: item.operation,
    expected: item.expected,
    variants: Object.fromEntries(plan.variants.map((variant, index) => [variant.name, {
      contract: deployments[index].contract,
      actual: String(item.expected),
      sdk_result_matched_oracle: true,
      coprocessor_fee_wei: null,
      sdk_encrypt_ms: 15,
      confirmation_ms: 30,
      sdk_decrypt_wait_ms: 10,
      ...newTransaction(),
    }])),
  }));
  const raw = {
    schema: "fhec-cofhe-phase89-live-sdk-trace-v1",
    evidence_tier: "provisional-sdk-live-results-pending-independent-rpc-review",
    chain: "arb-sepolia", chain_id: 421614,
    profile: plan.profile, variant_mode: plan.variant_mode,
    plan_sha256: planDigest(plan), source_seal_sha256: "A".repeat(64),
    signer: "0x" + "a".repeat(40),
    case_count: plan.case_count, transaction_count: plan.total_planned_transactions,
    deployments, records,
    native_chain_fees_paid_wei: (BigInt(fee) * BigInt(plan.total_planned_transactions)).toString(),
    coprocessor_fees_measured: false,
  };
  const receipts = [
    ...deployments.map((item) => ({ ...item, kind: "deploy", confirmations: "3" })),
    ...records.flatMap((record) => plan.variants.map((variant) => ({ ...record.variants[variant.name], kind: "execute", confirmations: "3" }))),
  ];
  const attestation = {
    schema: "fhec-cofhe-phase89-rpc-attestation-v1",
    evidence_tier: "arbitrum-sepolia-receipt-corroborated-sdk-decryption-not-independently-proven",
    chain_id: 421614,
    raw_trace_sha256: hashBytes(JSON.stringify(raw)),
    plan_sha256: planDigest(plan),
    source_seal_sha256: raw.source_seal_sha256,
    minimum_confirmations: 2,
    deployment_count: plan.deployment_count,
    execution_count: plan.execution_count,
    verified_transaction_count: plan.total_planned_transactions,
    native_chain_fees_paid_wei: raw.native_chain_fees_paid_wei,
    coprocessor_fees_measured: false,
    receipts,
  };
  return { plan, raw, attestation };
}
