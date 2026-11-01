import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { validateLive } from "../phase6/validate-results.mjs";
import { canonicalHash } from "./metrics.mjs";

const digits = /^0x[0-9a-fA-F]+$/;
const hash64 = /^0x[0-9a-fA-F]{64}$/;

function normalizeHex(value) {
  return typeof value === "string" ? value.toLowerCase() : "";
}

function asHexInt(value) {
  if (typeof value !== "string" || !digits.test(value)) throw new Error("Invalid Ethereum RPC hex integer");
  return BigInt(value);
}

function nonemptyCode(value) {
  return typeof value === "string" && /^0x[0-9a-fA-F]+$/.test(value) && value !== "0x" && !/^0x0+$/.test(value);
}

function requiredReceipt(value, hash) {
  if (!value || normalizeHex(value.transactionHash) !== normalizeHex(hash) ||
      !hash64.test(value.blockHash) || asHexInt(value.status) !== 1n ||
      asHexInt(value.gasUsed) === 0n) {
    throw new Error("Missing, reverted, or malformed on-chain transaction receipt: " + hash);
  }
  return value;
}

function requiredTransaction(value, receipt, signer) {
  if (!value || normalizeHex(value.hash) !== normalizeHex(receipt.transactionHash) ||
      normalizeHex(value.from) !== normalizeHex(signer) ||
      normalizeHex(value.blockHash) !== normalizeHex(receipt.blockHash) ||
      asHexInt(value.blockNumber) !== asHexInt(receipt.blockNumber)) {
    throw new Error("Transaction identity or signer does not match receipt");
  }
  return value;
}

export function createHttpRpc(url, fetchImpl = fetch) {
  if (typeof url !== "string" || !/^https:\/\/[^/]+/.test(url)) {
    throw new Error("FHEC_TESTNET_RPC_URL must be an HTTPS RPC URL");
  }
  let requestId = 0;
  return async (method, params) => {
    requestId += 1;
    let lastError;
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        const response = await fetchImpl(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ jsonrpc: "2.0", id: requestId, method, params }),
          signal: AbortSignal.timeout(12000),
        });
        if (!response.ok) throw new Error("RPC HTTP error " + response.status);
        const payload = await response.json();
        if (payload.id !== requestId || payload.jsonrpc !== "2.0" ||
            payload.error || !Object.prototype.hasOwnProperty.call(payload, "result")) {
          throw new Error("RPC returned an invalid envelope");
        }
        return payload.result;
      } catch (error) {
        lastError = error;
        if (attempt < 2) await new Promise((resolveDelay) => setTimeout(resolveDelay, 300 * (attempt + 1)));
      }
    }
    throw new Error("Read-only RPC request failed: " + method + " / " + lastError?.message);
  };
}

export async function verifyReceipts(trace, rpc, minConfirmations = 2) {
  validateLive(trace);
  if (!Number.isSafeInteger(minConfirmations) || minConfirmations < 1 || minConfirmations > 64) {
    throw new Error("Invalid required confirmation depth");
  }
  const reportedChain = asHexInt(await rpc("eth_chainId", []));
  if (reportedChain !== 421614n) throw new Error("Wrong chain: expected Arbitrum Sepolia chain ID 421614");
  const latest = asHexInt(await rpc("eth_blockNumber", []));

  const deploymentHash = trace.deployment_transaction;
  const deploymentReceipt = requiredReceipt(await rpc("eth_getTransactionReceipt", [deploymentHash]), deploymentHash);
  const deploymentTx = requiredTransaction(await rpc("eth_getTransactionByHash", [deploymentHash]), deploymentReceipt, trace.signer);
  if (deploymentTx.to !== null || normalizeHex(deploymentReceipt.contractAddress) !== normalizeHex(trace.contract) ||
      asHexInt(deploymentReceipt.blockNumber) !== BigInt(trace.deployment_block)) {
    throw new Error("On-chain deployment contract or block does not match the SDK trace");
  }
  const deploymentConf = latest - asHexInt(deploymentReceipt.blockNumber) + 1n;
  if (deploymentConf < BigInt(minConfirmations)) throw new Error("Deployment lacks required confirmation depth");

  const code = await rpc("eth_getCode", [trace.contract, "latest"]);
  if (!nonemptyCode(code)) throw new Error("Deployed contract code is missing");

  const transactions = [];
  const seen = new Set([normalizeHex(deploymentHash)]);
  for (const record of trace.records) {
    const hash = normalizeHex(record.transaction_hash);
    if (seen.has(hash)) throw new Error("Repeated receipt or reused deployment transaction");
    seen.add(hash);
    const receipt = requiredReceipt(await rpc("eth_getTransactionReceipt", [hash]), hash);
    const transaction = requiredTransaction(await rpc("eth_getTransactionByHash", [hash]), receipt, trace.signer);
    if (normalizeHex(transaction.to) !== normalizeHex(trace.contract) ||
        asHexInt(receipt.blockNumber) !== BigInt(record.block_number) ||
        asHexInt(receipt.gasUsed) !== BigInt(record.transaction_gas_used)) {
      throw new Error("On-chain execution differs from Phase 6 SDK trace: " + record.label);
    }
    const confirmations = latest - asHexInt(receipt.blockNumber) + 1n;
    if (confirmations < BigInt(minConfirmations)) {
      throw new Error("Execution lacks required confirmations: " + record.label);
    }
    transactions.push({
      label: record.label,
      transaction_hash: record.transaction_hash,
      block_hash: receipt.blockHash,
      gas_used: asHexInt(receipt.gasUsed).toString(),
      confirmations: confirmations.toString(),
    });
  }
  return {
    schema: "fhec-cofhe-phase7-rpc-corroboration-v1",
    evidence_tier: "testnet-transport-receipt-verified-sdk-decryption-not-independently-attested",
    chain_id: 421614,
    trace_sha256: canonicalHash(trace),
    signer: trace.signer,
    contract: trace.contract,
    deployment_transaction: deploymentHash,
    verified_transaction_count: transactions.length,
    minimum_confirmations_required: minConfirmations,
    transactions,
    runtime_bytecode_present: true,
    limitations: [
      "The configured RPC provider can misreport data; use an independent RPC provider to corroborate.",
      "Receipts prove chain inclusion and gas, not correct off-chain FHE computation or decryption.",
      "The SDK trace's decrypted values have not been independently reproduced by another signer or auditor.",
      "No mainnet authorization, security audit, or measured coprocessor fee is asserted.",
    ],
  };
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
}

export async function main(args) {
  if (args.length !== 6 || args[0] !== "--trace" || args[2] !== "--out" ||
      args[4] !== "--confirmations") {
    throw new Error("Usage: --trace <phase6-live-trace.json> --out <corroboration.json> --confirmations <2>");
  }
  const rpc = createHttpRpc(process.env.FHEC_TESTNET_RPC_URL);
  const result = await verifyReceipts(readJson(args[1]), rpc, Number(args[5]));
  writeFileSync(args[3], JSON.stringify(result, null, 2) + "\n", { flag: "wx" });
  process.stdout.write("Verified " + result.verified_transaction_count + " Arbitrum Sepolia transaction receipts; decryption remains SDK-reported.\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch((error) => {
    process.stderr.write("Phase 7 live receipt verification failed: " + error.message + "\n");
    process.exitCode = 1;
  });
}
