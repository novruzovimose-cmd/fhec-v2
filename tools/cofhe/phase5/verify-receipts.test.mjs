import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createRpc, validateRpcUrl, validateTarget, verifyReceipts } from "./verify-receipts.mjs";

const address = `0x${"1".repeat(40)}`;
const hash = `0x${"a".repeat(64)}`;
const blockHash = `0x${"b".repeat(64)}`;
const selector = "0x12345678";

function target() {
  return { schema: "fhec-phase5-onchain-target-v1", chain_id: 421614, contract: address,
    min_confirmations: 3,
    transactions: [{ hash, to: address, method_selector: selector, label: "encrypted_input_tx" }],
  };
}

function fixture() {
  const responses = {
    eth_chainId: "0x66eee",
    eth_blockNumber: "0x10",
    eth_getCode: "0x60016000",
    eth_getTransactionByHash: { hash, to: address, input: `${selector}${"00".repeat(32)}`, blockNumber: "0xd", blockHash },
    eth_getTransactionReceipt: { transactionHash: hash, to: address, status: "0x1", gasUsed: "0x5208", blockNumber: "0xd", blockHash },
  };
  return { responses, rpc: async (method) => responses[method] };
}

describe("Phase 5 on-chain receipt verifier", () => {
  it("validates confirmed transaction inclusion without claiming FHE", async () => {
    const { rpc } = fixture();
    const result = await verifyReceipts(target(), rpc);
    assert.equal(result.verified_transactions.length, 1);
    assert.equal(result.verified_transactions[0].confirmations, "4");
    assert.equal(result.verified_transactions[0].transaction_gas_used, "21000");
    assert.equal(result.cryptographic_claims.ciphertext_proof_verified, false);
    assert.equal(result.cryptographic_claims.fhe_coprocessor_execution_verified, false);
  });

  it("rejects mismatched chains", async () => {
    const { responses, rpc } = fixture();
    responses.eth_chainId = "0x1";
    await assert.rejects(verifyReceipts(target(), rpc), /chain ID/);
  });

  it("rejects unconfirmed transactions", async () => {
    const { responses, rpc } = fixture();
    responses.eth_blockNumber = "0xe";
    await assert.rejects(verifyReceipts(target(), rpc), /confirmations/);
  });

  it("rejects reverted transactions", async () => {
    const { responses, rpc } = fixture();
    responses.eth_getTransactionReceipt.status = "0x0";
    await assert.rejects(verifyReceipts(target(), rpc), /status/);
  });

  it("rejects wrong destination contracts", async () => {
    const { responses, rpc } = fixture();
    responses.eth_getTransactionByHash.to = `0x${"2".repeat(40)}`;
    await assert.rejects(verifyReceipts(target(), rpc), /destination/);
  });

  it("rejects wrong input selectors", async () => {
    const { responses, rpc } = fixture();
    responses.eth_getTransactionByHash.input = "0x87654321";
    await assert.rejects(verifyReceipts(target(), rpc), /selector/);
  });

  it("rejects empty deployed code", async () => {
    const { responses, rpc } = fixture();
    responses.eth_getCode = "0x";
    await assert.rejects(verifyReceipts(target(), rpc), /bytecode/);
  });

  it("rejects duplicate transaction hashes and unknown destinations", () => {
    const duplicate = target();
    duplicate.transactions.push({ ...duplicate.transactions[0], label: "duplicated_tx" });
    assert.throws(() => validateTarget(duplicate), /Duplicate/);
    const wrong = target();
    wrong.transactions[0].to = `0x${"2".repeat(40)}`;
    assert.throws(() => validateTarget(wrong), /destination differs/);
  });

  it("enforces HTTPS except loopback", () => {
    assert.equal(validateRpcUrl("http://127.0.0.1:8545/"), "http://127.0.0.1:8545/");
    assert.match(validateRpcUrl("https://example.org/rpc"), /^https:/);
    assert.throws(() => validateRpcUrl("http://example.org"), /HTTPS/);
    assert.throws(() => validateRpcUrl("https://u:p@example.org"), /userinfo/);
    assert.throws(() => validateRpcUrl("file:///tmp/rpc"), /HTTPS/);
  });

  it("only permits read-only RPC methods", async () => {
    const rpc = createRpc("http://127.0.0.1:8545");
    await assert.rejects(rpc("eth_sendRawTransaction", ["0x00"]), /Disallowed/);
  });

  it("checks RPC IDs and rejects provider errors", async () => {
    const badFetch = async () => ({
      ok: true, headers: { get: () => null },
      text: async () => JSON.stringify({ jsonrpc: "2.0", id: 999, result: "0x1" }),
    });
    const rpc = createRpc("http://127.0.0.1:8545", badFetch);
    await assert.rejects(rpc("eth_chainId"), /request ID/);
  });
});
