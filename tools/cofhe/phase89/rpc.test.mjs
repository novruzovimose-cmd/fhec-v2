import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createRpc, rpcAddress, rpcHash, rpcQuantity, sameHex } from "./rpc.mjs";

describe("Phase 8/9 strictly read-only HTTPS JSON-RPC client", () => {
  it("rejects HTTP, URL credentials and invalid addresses", () => {
    assert.throws(() => createRpc("http://localhost:8545"), /HTTPS/);
    assert.throws(() => createRpc("https://user:password@example.org"), /HTTPS/);
    assert.equal(rpcAddress("0x" + "a".repeat(40)), true);
    assert.equal(rpcHash("0x" + "a".repeat(64)), true);
    assert.equal(rpcAddress("0x123"), false);
    assert.equal(sameHex("0xAB", "0xab"), true);
  });
  it("parses arbitrary exact Ethereum hex quantities", () => {
    assert.equal(rpcQuantity("0x30d40"), 200000n);
    assert.throws(() => rpcQuantity("200000"), /Malformed/);
    assert.throws(() => rpcQuantity(-1), /Malformed/);
  });
  it("uses only explicitly allowlisted read RPC methods", async () => {
    const rpc = createRpc("https://example.org/jsonrpc", async (_, options) => ({
      ok: true,
      json: async () => {
        const request = JSON.parse(options.body);
        assert.equal(request.method, "eth_chainId");
        return { id: request.id, jsonrpc: "2.0", result: "0x66eee" };
      },
    }));
    assert.equal(await rpc("eth_chainId", []), "0x66eee");
    await assert.rejects(() => rpc("eth_sendRawTransaction", ["0xdeadbeef"]), /Unsupported read-only/);
  });
  it("rejects malformed response envelopes after bounded retries", async () => {
    let calls = 0;
    const rpc = createRpc("https://example.org", async () => { calls += 1; return { ok: true, json: async () => ({ id: 999, jsonrpc: "2.0", result: "0x1" }) }; });
    await assert.rejects(() => rpc("eth_chainId", []), /invalid|Malformed/i);
    assert.equal(calls, 3);
  });
});
