import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { digest, exact, fail, hex, integer, jsonFile, object, options, quantity, requireNewPath, sha256 } from "./strict.mjs";

const address = (value, label) => hex(value, label, 20);
const transactionHash = (value, label) => hex(value, label, 32);

export function validateTarget(target) {
  object(target, "on-chain target");
  exact(target.schema, "fhec-phase5-onchain-target-v1", "on-chain target schema");
  integer(target.chain_id, "expected chain ID", 1, 999999999);
  integer(target.min_confirmations, "minimum confirmations", 1, 1000);
  const contract = address(target.contract, "contract address");
  if (!Array.isArray(target.transactions) || target.transactions.length < 1 || target.transactions.length > 30) {
    fail("Target must contain 1-30 concrete transaction assertions");
  }
  const distinct = new Set();
  const transactions = target.transactions.map((entry, index) => {
    object(entry, `transaction ${index}`);
    const hash = transactionHash(entry.hash, `transaction ${index} hash`);
    const to = address(entry.to, `transaction ${index} destination`);
    if (to !== contract) fail(`Transaction ${index} destination differs from contract`);
    if (distinct.has(hash)) fail("Duplicate transaction hash in on-chain target");
    distinct.add(hash);
    const selector = hex(entry.method_selector, `transaction ${index} method selector`, 4);
    if (typeof entry.label !== "string" || !/^[a-z0-9][a-z0-9_-]{1,48}$/.test(entry.label)) {
      fail(`Transaction ${index} has an invalid label`);
    }
    return { hash, to, method_selector: selector, label: entry.label };
  });
  return { chain_id: target.chain_id, min_confirmations: target.min_confirmations, contract, transactions };
}

export function validateRpcUrl(url) {
  if (typeof url !== "string" || url.length > 2048) fail("Missing or oversized RPC endpoint");
  let parsed;
  try { parsed = new URL(url); } catch { fail("Invalid RPC endpoint URL"); }
  if (parsed.username || parsed.password || parsed.hash) fail("RPC URL cannot contain userinfo or fragment");
  const local = ["localhost", "127.0.0.1", "[::1]"].includes(parsed.hostname.toLowerCase());
  if (parsed.protocol !== "https:" && !(local && parsed.protocol === "http:")) {
    fail("RPC must use HTTPS (HTTP allowed only for loopback tests)");
  }
  return parsed.toString();
}

export function createRpc(endpoint, fetchImpl = fetch) {
  const url = validateRpcUrl(endpoint);
  let id = 0;
  return async function rpc(method, params = []) {
    const allowed = ["eth_chainId", "eth_blockNumber", "eth_getCode", "eth_getTransactionByHash", "eth_getTransactionReceipt"];
    if (!allowed.includes(method)) fail("Disallowed RPC method");
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 12000);
    try {
      const requestId = ++id;
      const response = await fetchImpl(url, {
        method: "POST",
        headers: { "content-type": "application/json", accept: "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: requestId, method, params }),
        signal: controller.signal,
        redirect: "error",
      });
      if (!response.ok) fail(`JSON-RPC returned HTTP ${response.status}`);
      const contentLength = response.headers?.get("content-length");
      if (contentLength && Number(contentLength) > 1024 * 1024) fail("RPC response exceeds size limit");
      const raw = await response.text();
      if (raw.length > 1024 * 1024) fail("RPC response exceeds size limit");
      let payload;
      try { payload = object(JSON.parse(raw), "JSON-RPC response"); }
      catch { fail("Malformed JSON-RPC response"); }
      exact(payload.id, requestId, "JSON-RPC request ID");
      exact(payload.jsonrpc, "2.0", "JSON-RPC version");
      if (payload.error) fail(`JSON-RPC error calling ${method}`);
      if (!Object.hasOwn(payload, "result")) fail("JSON-RPC response is missing result");
      return payload.result;
    } finally {
      clearTimeout(timeout);
    }
  };
}

export async function verifyReceipts(targetInput, rpc) {
  const target = validateTarget(targetInput);
  const chainId = quantity(await rpc("eth_chainId"), "RPC chain ID");
  if (chainId !== BigInt(target.chain_id)) fail(`RPC chain ID ${chainId} differs from target ${target.chain_id}`);
  const head = quantity(await rpc("eth_blockNumber"), "current block height");
  const code = await rpc("eth_getCode", [target.contract, "latest"]);
  if (typeof code !== "string" || !/^0x(?:[0-9a-fA-F]{2})+$/.test(code)) {
    fail("Target contract has no deployed bytecode");
  }
  const codeDigest = sha256(Buffer.from(code.slice(2), "hex"));
  const verified = [];

  for (const txTarget of target.transactions) {
    const transaction = object(await rpc("eth_getTransactionByHash", [txTarget.hash]), `transaction ${txTarget.label}`);
    const receipt = object(await rpc("eth_getTransactionReceipt", [txTarget.hash]), `receipt ${txTarget.label}`);
    exact(transactionHash(transaction.hash, "transaction hash"), txTarget.hash, "transaction hash");
    exact(transactionHash(receipt.transactionHash, "receipt transaction hash"), txTarget.hash, "receipt transaction hash");
    exact(address(transaction.to, "transaction destination"), txTarget.to, "transaction destination");
    exact(address(receipt.to, "receipt destination"), txTarget.to, "receipt destination");
    exact(quantity(receipt.status, "transaction status"), 1n, "successful receipt status");
    const block = quantity(receipt.blockNumber, "receipt block number");
    exact(quantity(transaction.blockNumber, "transaction block number"), block, "transaction-receipt block number");
    if (block > head) fail("Receipt block is above observed chain head");
    const confirmations = head - block + 1n;
    if (confirmations < BigInt(target.min_confirmations)) fail(`Not enough confirmations for ${txTarget.label}`);
    const input = transaction.input;
    if (typeof input !== "string" || !/^0x(?:[0-9a-fA-F]{2})*$/.test(input) ||
        !input.toLowerCase().startsWith(txTarget.method_selector)) {
      fail(`Function selector mismatch for ${txTarget.label}`);
    }
    if (transaction.blockHash && receipt.blockHash) {
      exact(transactionHash(transaction.blockHash, "transaction block hash"), transactionHash(receipt.blockHash, "receipt block hash"), "receipt block hash");
    }
    const used = quantity(receipt.gasUsed, "receipt gas usage");
    if (used === 0n) fail("Successful on-chain transaction has zero gas usage");
    verified.push({
      label: txTarget.label,
      transaction_hash: txTarget.hash,
      block_number: block.toString(),
      confirmations: confirmations.toString(),
      transaction_gas_used: used.toString(),
      method_selector: txTarget.method_selector,
    });
  }

  return {
    schema: "fhec-phase5-readonly-receipt-evidence-v1",
    tier: "read-only-public-chain-receipts",
    chain_id: target.chain_id,
    target_contract: target.contract,
    contract_code_sha256: codeDigest,
    head_block: head.toString(),
    verified_transactions: verified,
    confirmations_required: target.min_confirmations,
    cryptographic_claims: {
      chain_transactions_verified: true,
      ciphertext_proof_verified: false,
      fhe_coprocessor_execution_verified: false,
      decrypted_outputs_verified: false,
      protocol_security_audited: false,
    },
    caveat: "A transaction receipt proves inclusion/success and bytecode presence, NOT that encrypted computation or FHE output validation occurred.",
  };
}

export async function main(args) {
  const { target, rpcEnv, out } = options(args, { "--target": "target", "--rpc-env": "rpcEnv", "--out": "out" });
  if (!/^[A-Z][A-Z0-9_]{3,70}$/.test(rpcEnv)) fail("RPC environment variable name is invalid");
  const endpoint = process.env[rpcEnv];
  if (!endpoint) fail(`Missing RPC endpoint environment variable ${rpcEnv}`);
  const result = await verifyReceipts(jsonFile(target, "on-chain target"), createRpc(endpoint));
  writeFileSync(requireNewPath(out), `${JSON.stringify(result, null, 2)}\n`, { flag: "wx" });
  process.stdout.write(`Verified ${result.verified_transactions.length} transaction receipt(s); FHE execution remains unverified.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`Phase 5 receipt verification failed: ${error.message}\n`);
    process.exitCode = 1;
  });
}
