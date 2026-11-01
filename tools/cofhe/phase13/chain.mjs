import { analyzeLive } from "../phase89/analyze-live.mjs";
import { loadPlan } from "../phase89/plan.mjs";
import { createRpc } from "../phase89/rpc.mjs";
import { validateSdkTrace, verifyChain } from "../phase89/verify-chain.mjs";
import { checkedUrl, hash, jsonFile } from "./io.mjs";

function keyReceipt(value) {
  return JSON.stringify({
    kind: value.kind,
    hash: value.transaction_hash.toLowerCase(),
    block: value.block_hash.toLowerCase(),
    height: value.block_number,
    fee: value.native_chain_fee_wei,
    calldata: value.calldata_sha256.toUpperCase(),
  });
}

export function compareRpcAttestations(left, right) {
  if (!left || !right || left.schema !== "fhec-cofhe-phase89-rpc-attestation-v1" ||
      right.schema !== left.schema || left.chain_id !== 421614 || right.chain_id !== 421614 ||
      left.source_seal_sha256 !== right.source_seal_sha256 ||
      left.plan_sha256 !== right.plan_sha256 || left.raw_trace_sha256 !== right.raw_trace_sha256 ||
      left.native_chain_fees_paid_wei !== right.native_chain_fees_paid_wei ||
      left.verified_transaction_count !== right.verified_transaction_count ||
      left.deployment_count !== right.deployment_count || left.execution_count !== right.execution_count ||
      left.coprocessor_fees_measured !== false || right.coprocessor_fees_measured !== false ||
      !Array.isArray(left.receipts) || !Array.isArray(right.receipts) ||
      left.receipts.length !== left.verified_transaction_count || right.receipts.length !== left.receipts.length) {
    throw new Error("The two independent RPC reviews disagree about source, plan, execution count or native fees");
  }
  const receipt1 = [...left.receipts].map(keyReceipt).sort();
  const receipt2 = [...right.receipts].map(keyReceipt).sort();
  if (new Set(receipt1).size !== receipt1.length || new Set(receipt2).size !== receipt2.length ||
      JSON.stringify(receipt1) !== JSON.stringify(receipt2)) {
    throw new Error("Two RPC providers disagree on a receipt, transaction, code or fee record");
  }
  return {
    schema: "fhec-cofhe-phase13-two-provider-testnet-corroboration-v1",
    evidence_tier: "two-independent-read-only-arbitrum-sepolia-rpc-replays",
    chain_id: 421614,
    plan_sha256: left.plan_sha256,
    source_seal_sha256: left.source_seal_sha256,
    sdk_trace_sha256: left.raw_trace_sha256,
    transaction_count: left.verified_transaction_count,
    deployment_count: left.deployment_count,
    execution_count: left.execution_count,
    native_chain_fees_paid_wei: left.native_chain_fees_paid_wei,
    coprocessor_fees_measured: false,
    independent_rpc_agreement: true,
    receipt_records_digest: hash(receipt1.join("\n")),
    provider_confirmations_at_least: Math.min(left.minimum_confirmations, right.minimum_confirmations),
    limitations: [
      "Both RPC providers may rely on overlapping blockchain infrastructure; this is corroboration, not a formal consensus proof.",
      "SDK-observed plaintext correctness is not an independent proof of coprocessor cryptographic security.",
      "Native gas/transaction fees exclude unmetered off-chain coprocessor service fees.",
      "No live transaction is submitted by Phase 13.",
    ],
  };
}

export async function checkLive(planFile, sdkTraceFile, rpcA, rpcB, confirmations = 2, makeRpc = createRpc) {
  if (!Number.isInteger(confirmations) || confirmations < 2 || confirmations > 64) {
    throw new Error("Require at least two receipt confirmations");
  }
  const a = checkedUrl(rpcA);
  const b = checkedUrl(rpcB);
  if (a.hostname.toLowerCase() === b.hostname.toLowerCase()) {
    throw new Error("Two RPC providers must have different hostnames");
  }
  const plan = loadPlan(planFile);
  const raw = jsonFile(sdkTraceFile);
  validateSdkTrace(plan, raw);
  const first = await verifyChain(plan, raw, makeRpc(rpcA), confirmations);
  const second = await verifyChain(plan, raw, makeRpc(rpcB), confirmations);
  const consensus = compareRpcAttestations(first, second);
  const analysis = analyzeLive(plan, raw, first);
  return {
    consensus,
    analysis,
    public_receipt_reference: {
      chain_id: 421614,
      deployment_transactions: raw.deployments.map((entry) => entry.transaction_hash),
      execution_transactions: raw.records.flatMap((sample) =>
        plan.variants.map((variant) => sample.variants[variant.name].transaction_hash)),
    },
  };
}
