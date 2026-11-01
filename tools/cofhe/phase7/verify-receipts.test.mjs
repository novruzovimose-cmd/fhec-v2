import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createHttpRpc, verifyReceipts } from "./verify-receipts.mjs";

const signature = (number) => "0x" + number.toString(16).padStart(64, "0");
const signer = "0x" + "a".repeat(40);
const contract = "0x" + "b".repeat(40);
const blockHash = signature(7777);

function trace() {
  return {
    schema: "fhec-cofhe-phase6-live-testnet-v1",
    environment: "Arbitrum Sepolia testnet",
    chain_id: 421614,
    contract,
    signer,
    deployment_transaction: signature(1),
    deployment_block: 100,
    completed_encrypted_cases: 3,
    records: [
      { label: "maximum-u8", input_a: "19", input_b: "41", expected: "41", actual: "41",
        transaction_hash: signature(2), transaction_gas_used: "111111", block_number: 101, matched_independent_oracle: true },
      { label: "addition-wrap-u8", input_a: "255", input_b: "1", expected: "0", actual: "0",
        transaction_hash: signature(3), transaction_gas_used: "111112", block_number: 102, matched_independent_oracle: true },
      { label: "greater-u8", input_a: "241", input_b: "12", expected: "true", actual: "true",
        transaction_hash: signature(4), transaction_gas_used: "111113", block_number: 103, matched_independent_oracle: true },
    ],
  };
}

function fakeRpc(current = trace()) {
  const receipts = new Map();
  const transactions = new Map();
  for (const [index, hash] of [current.deployment_transaction, ...current.records.map((row) => row.transaction_hash)].entries()) {
    const block = index === 0 ? current.deployment_block : current.records[index - 1].block_number;
    const gas = index === 0 ? 450000 : Number(current.records[index - 1].transaction_gas_used);
    receipts.set(hash.toLowerCase(), {
      transactionHash: hash, blockHash, blockNumber: "0x" + block.toString(16),
      gasUsed: "0x" + gas.toString(16), status: "0x1",
      contractAddress: index === 0 ? contract : null,
    });
    transactions.set(hash.toLowerCase(), {
      hash, blockHash, blockNumber: "0x" + block.toString(16), from: signer,
      to: index === 0 ? null : contract,
    });
  }
  const overrides = { chain: "0x66eee", height: "0x78", code: "0x60016000" };
  const responder = async (method, params) => {
    if (method === "eth_chainId") return overrides.chain;
    if (method === "eth_blockNumber") return overrides.height;
    if (method === "eth_getCode") return overrides.code;
    if (method === "eth_getTransactionReceipt") return receipts.get(params[0].toLowerCase()) || null;
    if (method === "eth_getTransactionByHash") return transactions.get(params[0].toLowerCase()) || null;
    throw new Error("Unexpected method");
  };
  return { responder, overrides, receipts, transactions };
}

describe("Phase 7 read-only RPC corroboration", () => {
  it("verifies exact chain, all four receipts, deployment code and gas", async () => {
    const data = trace();
    const rpc = fakeRpc(data);
    const evidence = await verifyReceipts(data, rpc.responder, 2);
    assert.equal(evidence.verified_transaction_count, 3);
    assert.equal(evidence.runtime_bytecode_present, true);
    assert.match(evidence.evidence_tier, /not-independently-attested/);
  });
  it("rejects the wrong chain", async () => {
    const rpc = fakeRpc();
    rpc.overrides.chain = "0x1";
    await assert.rejects(verifyReceipts(trace(), rpc.responder), /Wrong chain/);
  });
  it("rejects a removed contract", async () => {
    const rpc = fakeRpc();
    rpc.overrides.code = "0x";
    await assert.rejects(verifyReceipts(trace(), rpc.responder), /contract code is missing/);
  });
  it("rejects unconfirmed testnet transactions", async () => {
    const rpc = fakeRpc();
    rpc.overrides.height = "0x66";
    await assert.rejects(verifyReceipts(trace(), rpc.responder, 2), /confirmations/);
  });
  it("rejects changed gas and reverted receipts", async () => {
    const rpc = fakeRpc();
    rpc.receipts.get(signature(3)).gasUsed = "0x999";
    await assert.rejects(verifyReceipts(trace(), rpc.responder), /differs/);
    const other = fakeRpc();
    other.receipts.get(signature(3)).status = "0x0";
    await assert.rejects(verifyReceipts(trace(), other.responder), /reverted/);
  });
  it("rejects a different signer and incorrect contract address", async () => {
    const rpc = fakeRpc();
    rpc.transactions.get(signature(4)).from = "0x" + "c".repeat(40);
    await assert.rejects(verifyReceipts(trace(), rpc.responder), /signer/);
    const other = fakeRpc();
    other.receipts.get(signature(1)).contractAddress = "0x" + "c".repeat(40);
    await assert.rejects(verifyReceipts(trace(), other.responder), /deployment contract/);
  });
  it("never treats a forged self-report as live evidence", async () => {
    const data = trace();
    data.records[0].actual = "40";
    await assert.rejects(verifyReceipts(data, fakeRpc().responder), /SDK trace/);
  });
  it("enforces minimal bounded confirmation depth", async () => {
    await assert.rejects(verifyReceipts(trace(), fakeRpc().responder, 0), /Invalid/);
    await assert.rejects(verifyReceipts(trace(), fakeRpc().responder, 65), /Invalid/);
  });
  it("uses HTTPS and validates JSON-RPC response envelopes", async () => {
    assert.throws(() => createHttpRpc("http://127.0.0.1:8545"), /HTTPS/);
    const client = createHttpRpc("https://example.invalid", async (url, request) => {
      assert.match(url, /^https:/);
      const body = JSON.parse(request.body);
      return { ok: true, json: async () => ({ jsonrpc: "2.0", id: body.id, result: "0x66eee" }) };
    });
    assert.equal(await client("eth_chainId", []), "0x66eee");
  });
});
