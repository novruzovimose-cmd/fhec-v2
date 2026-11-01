import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createPlan, planDigest } from "./plan.mjs";
import { hashBytes } from "./source-seal.mjs";
import { validateSdkTrace, verifyChain } from "./verify-chain.mjs";

const signer = "0x" + "a".repeat(40);
const code = "0x6001600055";

export function fakeNetwork() {
  const plan = createPlan();
  let sequence = 0;
  const receipts = new Map();
  const transactions = new Map();
  const deploymentAddress = new Map();
  const nativeFee = 200000n * 100n;
  const record = (kind, contract) => {
    sequence += 1;
    const hash = "0x" + sequence.toString(16).padStart(64, "0");
    const blockHash = "0x" + (sequence + 400).toString(16).padStart(64, "0");
    const blockNumber = 100 + sequence;
    const data = {
      transaction_hash: hash,
      calldata_sha256: hashBytes(Buffer.from("ab123456", "hex")),
      block_number: blockNumber,
      gas_used: "200000",
      effective_gas_price_wei: "100",
      native_chain_fee_wei: nativeFee.toString(),
    };
    receipts.set(hash, {
      transactionHash: hash,
      status: "0x1",
      blockNumber: "0x" + blockNumber.toString(16),
      blockHash,
      gasUsed: "0x30d40",
      effectiveGasPrice: "0x64",
      contractAddress: kind === "deploy" ? contract : null,
      to: kind === "execute" ? contract : null,
    });
    transactions.set(hash, {
      hash,
      from: signer,
      to: kind === "deploy" ? null : contract,
      blockHash,
      blockNumber: "0x" + blockNumber.toString(16),
      input: "0xab123456",
    });
    return data;
  };
  const deployments = plan.variants.map((variant, index) => {
    const contract = "0x" + (index + 10).toString(16).padStart(40, "0");
    deploymentAddress.set(variant.name, contract);
    return {
      variant: variant.name,
      contract,
      runtime_code_sha256: hashBytes(Buffer.from(code.slice(2), "hex")),
      confirmation_ms: 100,
      ...record("deploy", contract),
    };
  });
  const records = plan.cases.map((entry) => {
    const variants = {};
    for (const variant of plan.variants) {
      const contract = deploymentAddress.get(variant.name);
      variants[variant.name] = {
        contract,
        actual: String(entry.expected),
        sdk_result_matched_oracle: true,
        coprocessor_fee_wei: null,
        sdk_encrypt_ms: 80,
        confirmation_ms: 120,
        sdk_decrypt_wait_ms: 200,
        ...record("execute", contract),
      };
    }
    return { id: entry.id, width: entry.width, operation: entry.operation, expected: entry.expected, variants };
  });
  const raw = {
    schema: "fhec-cofhe-phase89-live-sdk-trace-v1",
    evidence_tier: "provisional-sdk-live-results-pending-independent-rpc-review",
    chain_id: 421614,
    chain: "arb-sepolia",
    profile: plan.profile,
    variant_mode: plan.variant_mode,
    plan_sha256: planDigest(plan),
    source_seal_sha256: "A".repeat(64),
    signer,
    case_count: plan.case_count,
    transaction_count: plan.total_planned_transactions,
    deployments,
    records,
    native_chain_fees_paid_wei: (nativeFee * BigInt(plan.total_planned_transactions)).toString(),
    coprocessor_fees_measured: false,
  };
  const rpc = async (method, params) => {
    if (method === "eth_chainId") return "0x66eee";
    if (method === "eth_blockNumber") return "0xff";
    if (method === "eth_getCode") return code;
    if (method === "eth_getTransactionReceipt") return receipts.get(params[0]);
    if (method === "eth_getTransactionByHash") return transactions.get(params[0]);
    throw new Error("Unsupported fake read-only RPC method");
  };
  return { plan, raw, receipts, transactions, rpc };
}

describe("Phase 8/9 adversarial on-chain evidence corroboration", () => {
  it("matches all 14 default smoke receipts, real gas prices and compiled code hashes", async () => {
    const sample = fakeNetwork();
    const report = await verifyChain(sample.plan, sample.raw, sample.rpc);
    assert.equal(report.verified_transaction_count, 14);
    assert.equal(report.execution_count, 12);
    assert.equal(report.native_chain_fees_paid_wei, sample.raw.native_chain_fees_paid_wei);
    assert.equal(report.coprocessor_fees_measured, false);
  });
  it("rejects an incomplete plan, false expected result and duplicated receipt", () => {
    const { plan, raw } = fakeNetwork();
    raw.records[0].variants.tuned.actual = "999";
    assert.throws(() => validateSdkTrace(plan, raw), /Malformed/);
    const other = fakeNetwork();
    other.raw.records[0].variants.tuned.transaction_hash = other.raw.records[0].variants.optimized.transaction_hash;
    assert.throws(() => validateSdkTrace(other.plan, other.raw), /Reused/);
  });
  it("rejects a forged SDK native fee and missing SDK result", () => {
    const { plan, raw } = fakeNetwork();
    raw.native_chain_fees_paid_wei = "1";
    assert.throws(() => validateSdkTrace(plan, raw), /fee totals/);
    const other = fakeNetwork();
    other.raw.records[0].variants.tuned.sdk_result_matched_oracle = false;
    assert.throws(() => validateSdkTrace(other.plan, other.raw), /Malformed/);
  });
  it("rejects wrong chain, status and block hashes", async () => {
    const sample = fakeNetwork();
    const wrongChain = async (method, params) => method === "eth_chainId" ? "0x1" : sample.rpc(method, params);
    await assert.rejects(() => verifyChain(sample.plan, sample.raw, wrongChain), /not connected/);
    const hash = sample.raw.deployments[0].transaction_hash;
    sample.receipts.get(hash).status = "0x0";
    await assert.rejects(() => verifyChain(sample.plan, sample.raw, sample.rpc), /does not corroborate/);
  });
  it("rejects a different signer and contract destination", async () => {
    const sample = fakeNetwork();
    sample.transactions.get(sample.raw.deployments[0].transaction_hash).from = "0x" + "c".repeat(40);
    await assert.rejects(() => verifyChain(sample.plan, sample.raw, sample.rpc), /does not corroborate/);
    const second = fakeNetwork();
    const runHash = second.raw.records[0].variants.optimized.transaction_hash;
    second.transactions.get(runHash).to = "0x" + "d".repeat(40);
    await assert.rejects(() => verifyChain(second.plan, second.raw, second.rpc), /destination/);
  });
  it("rejects missing deployment bytecode and shallow confirmations", async () => {
    const sample = fakeNetwork();
    const noCode = async (method, params) => method === "eth_getCode" ? "0x" : sample.rpc(method, params);
    await assert.rejects(() => verifyChain(sample.plan, sample.raw, noCode), /runtime hash/);
    await assert.rejects(() => verifyChain(sample.plan, sample.raw, sample.rpc, 1000), /confirmation/);
  });
  it("detects on-chain price disagreement after the SDK trace looked valid", async () => {
    const sample = fakeNetwork();
    sample.receipts.get(sample.raw.deployments[0].transaction_hash).effectiveGasPrice = "0x65";
    await assert.rejects(() => verifyChain(sample.plan, sample.raw, sample.rpc), /does not corroborate/);
  });
  it("rejects altered execution calldata and fabricated calldata digests", async () => {
    const sample = fakeNetwork();
    const hash = sample.raw.records[0].variants.optimized.transaction_hash;
    sample.transactions.get(hash).input = "0xab123457";
    await assert.rejects(() => verifyChain(sample.plan, sample.raw, sample.rpc), /does not corroborate/);
    const changed = fakeNetwork();
    changed.raw.deployments[0].calldata_sha256 = "B".repeat(64);
    await assert.rejects(() => verifyChain(changed.plan, changed.raw, changed.rpc), /does not corroborate/);
  });
});
