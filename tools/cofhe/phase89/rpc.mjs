export function rpcQuantity(value, description = "RPC quantity") {
  if (typeof value !== "string" || !/^0x[0-9a-fA-F]+$/.test(value)) {
    throw new Error("Malformed " + description);
  }
  return BigInt(value);
}

export function rpcAddress(value) {
  return typeof value === "string" && /^0x[0-9a-fA-F]{40}$/.test(value);
}

export function rpcHash(value) {
  return typeof value === "string" && /^0x[0-9a-fA-F]{64}$/.test(value);
}

export function sameHex(left, right) {
  return typeof left === "string" && typeof right === "string" && left.toLowerCase() === right.toLowerCase();
}

export function createRpc(url, fetchImpl = fetch) {
  let parsed;
  try { parsed = new URL(url); } catch { throw new Error("A valid HTTPS JSON-RPC URL is required"); }
  if (parsed.protocol !== "https:" || !parsed.hostname || parsed.username || parsed.password) {
    throw new Error("Only HTTPS RPC endpoints without URL-embedded credentials are allowed");
  }
  let nextId = 0;
  return async (method, params = []) => {
    if (!["eth_chainId", "eth_blockNumber", "eth_getTransactionReceipt", "eth_getTransactionByHash", "eth_getCode"].includes(method)) {
      throw new Error("Unsupported read-only RPC method");
    }
    const id = ++nextId;
    let lastError;
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        const response = await fetchImpl(url, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ jsonrpc: "2.0", id, method, params }),
          signal: AbortSignal.timeout(15000),
        });
        if (!response.ok) throw new Error("RPC transport status " + response.status);
        const body = await response.json();
        if (!body || body.jsonrpc !== "2.0" || body.id !== id || body.error ||
            !Object.prototype.hasOwnProperty.call(body, "result")) {
          throw new Error("Malformed JSON-RPC response envelope");
        }
        return body.result;
      } catch (error) {
        lastError = error;
        if (attempt < 2) await new Promise((done) => setTimeout(done, 250 * (attempt + 1)));
      }
    }
    throw new Error("Read-only RPC operation failed for " + method + ": " + lastError?.message);
  };
}
