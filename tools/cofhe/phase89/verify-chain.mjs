import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { hashBytes } from "./source-seal.mjs";
import { loadPlan, planDigest, validatePlan } from "./plan.mjs";
import { createRpc, rpcAddress, rpcHash, rpcQuantity, sameHex } from "./rpc.mjs";

const decimal = /^(0|[1-9][0-9]*)$/;
const safe = BigInt(Number.MAX_SAFE_INTEGER);

export function uintString(value, name) {
  if (typeof value !== "string" || !decimal.test(value)) throw new Error("Malformed decimal field: " + name);
  return BigInt(value);
}

function finiteMs(value) {
  return Number.isSafeInteger(value) && value >= 0 && value < 86400000;
}

export function validateSdkTrace(plan, raw) {
  validatePlan(plan);
  if (!raw || raw.schema !== "fhec-cofhe-phase89-live-sdk-trace-v1" ||
      raw.evidence_tier !== "provisional-sdk-live-results-pending-independent-rpc-review" ||
      raw.chain_id !== 421614 || raw.chain !== "arb-sepolia" ||
      raw.plan_sha256 !== planDigest(plan) ||
      raw.case_count !== plan.case_count ||
      raw.profile !== plan.profile || raw.variant_mode !== plan.variant_mode ||
      !rpcHash("0x" + raw.source_seal_sha256) || !rpcAddress(raw.signer) ||
      raw.transaction_count !== plan.total_planned_transactions ||
      !Array.isArray(raw.deployments) || raw.deployments.length !== plan.deployment_count ||
      !Array.isArray(raw.records) || raw.records.length !== plan.case_count ||
      raw.coprocessor_fees_measured !== false) {
    throw new Error("Incomplete, incorrectly scoped or noncanonical SDK trace");
  }
  const receipts = new Set();
  let paidTotal = 0n;
  const addresses = new Map();
  for (let index = 0; index < plan.variants.length; index += 1) {
    const deployment = raw.deployments[index];
    const variant = plan.variants[index];
    if (!deployment || deployment.variant !== variant.name || !rpcAddress(deployment.contract) ||
        !rpcHash(deployment.transaction_hash) || !rpcHash("0x" + deployment.runtime_code_sha256) ||
        !rpcHash("0x" + deployment.calldata_sha256) ||
        !Number.isSafeInteger(deployment.block_number) || deployment.block_number < 1 ||
        !finiteMs(deployment.confirmation_ms)) throw new Error("Invalid SDK deployment: " + variant.name);
    const hash = deployment.transaction_hash.toLowerCase();
    if (receipts.has(hash)) throw new Error("Reused SDK receipt hash");
    receipts.add(hash);
    addresses.set(variant.name, deployment.contract);
    const gas = uintString(deployment.gas_used, "deployment gas");
    const price = uintString(deployment.effective_gas_price_wei, "deployment gas price");
    if (gas === 0n || price === 0n || gas * price !== uintString(deployment.native_chain_fee_wei, "deployment fee")) {
      throw new Error("Invalid recorded deployment native fee");
    }
    paidTotal += gas * price;
  }
  for (let index = 0; index < plan.cases.length; index += 1) {
    const sample = raw.records[index];
    const source = plan.cases[index];
    if (!sample || sample.id !== source.id || sample.width !== source.width ||
        sample.operation !== source.operation || sample.expected !== source.expected ||
        !sample.variants || Object.keys(sample.variants).sort().join(",") !== plan.variants.map((variant) => variant.name).sort().join(",")) {
      throw new Error("Missing or shuffled SDK live case: " + source.id);
    }
    for (const variant of plan.variants) {
      const value = sample.variants[variant.name];
      if (!value || !sameHex(value.contract, addresses.get(variant.name)) ||
          value.actual !== String(source.expected) || value.sdk_result_matched_oracle !== true ||
          value.coprocessor_fee_wei !== null || !rpcHash(value.transaction_hash) ||
          !rpcHash("0x" + value.calldata_sha256) ||
          !Number.isSafeInteger(value.block_number) || value.block_number < 1 ||
          !finiteMs(value.sdk_encrypt_ms) || !finiteMs(value.confirmation_ms) || !finiteMs(value.sdk_decrypt_wait_ms)) {
        throw new Error("Malformed or unverified SDK execution: " + source.id + "/" + variant.name);
      }
      const hash = value.transaction_hash.toLowerCase();
      if (receipts.has(hash)) throw new Error("Reused SDK receipt hash");
      receipts.add(hash);
      const gas = uintString(value.gas_used, "execution gas");
      const price = uintString(value.effective_gas_price_wei, "execution price");
      if (gas === 0n || price === 0n || gas * price !== uintString(value.native_chain_fee_wei, "execution fee")) {
        throw new Error("Invalid recorded execution native fee");
      }
      paidTotal += gas * price;
    }
  }
  if (receipts.size !== raw.transaction_count || paidTotal !== uintString(raw.native_chain_fees_paid_wei, "total native fees")) {
    throw new Error("SDK trace has missing receipts or incorrect fee totals");
  }
  return raw;
}

export async function verifyChain(plan, raw, rpc, minConfirmations = 2) {
  validateSdkTrace(plan, raw);
  if (!Number.isSafeInteger(minConfirmations) || minConfirmations < 1 || minConfirmations > 64) {
    throw new Error("Invalid required on-chain confirmation depth");
  }
  if (rpcQuantity(await rpc("eth_chainId", []), "chain ID") !== 421614n) {
    throw new Error("RPC is not connected to Arbitrum Sepolia chain 421614");
  }
  const latest = rpcQuantity(await rpc("eth_blockNumber", []), "latest block");
  const checked = [];

  async function attest(entry, type, expectedTo, contractAddress) {
    const hash = entry.transaction_hash;
    const receipt = await rpc("eth_getTransactionReceipt", [hash]);
    const transaction = await rpc("eth_getTransactionByHash", [hash]);
    if (!receipt || !transaction || !sameHex(receipt.transactionHash, hash) ||
        !sameHex(transaction.hash, hash) || !rpcHash(receipt.blockHash) ||
        !sameHex(receipt.blockHash, transaction.blockHash) ||
        !sameHex(transaction.from, raw.signer) ||
        typeof transaction.input !== "string" || !/^0x(?:[0-9a-fA-F]{2})+$/.test(transaction.input) ||
        hashBytes(Buffer.from(transaction.input.slice(2), "hex")) !== entry.calldata_sha256 ||
        rpcQuantity(receipt.status, "receipt status") !== 1n ||
        rpcQuantity(receipt.blockNumber, "receipt block") !== BigInt(entry.block_number) ||
        rpcQuantity(transaction.blockNumber, "transaction block") !== BigInt(entry.block_number) ||
        rpcQuantity(receipt.gasUsed, "receipt gas") !== uintString(entry.gas_used, "raw gas") ||
        rpcQuantity(receipt.effectiveGasPrice, "receipt gas price") !== uintString(entry.effective_gas_price_wei, "raw price") ||
        rpcQuantity(receipt.gasUsed, "receipt gas") * rpcQuantity(receipt.effectiveGasPrice, "gas price") !== uintString(entry.native_chain_fee_wei, "raw fee")) {
      throw new Error("On-chain receipt does not corroborate SDK transaction " + hash);
    }
    if (type === "deploy") {
      if (transaction.to !== null || !sameHex(receipt.contractAddress, contractAddress)) {
        throw new Error("Deployment is not the expected contract address");
      }
    } else if (!sameHex(transaction.to, expectedTo) || !sameHex(receipt.to, expectedTo)) {
      throw new Error("Execution destination does not match the deployed contract");
    }
    const confirmations = latest - BigInt(entry.block_number) + 1n;
    if (confirmations < BigInt(minConfirmations) || confirmations > safe) {
      throw new Error("Receipt has insufficient or invalid block confirmations");
    }
    checked.push({
      kind: type,
      transaction_hash: hash,
      block_hash: receipt.blockHash,
      block_number: entry.block_number,
      confirmations: confirmations.toString(),
      native_chain_fee_wei: entry.native_chain_fee_wei,
      calldata_sha256: entry.calldata_sha256,
    });
  }

  for (const deployment of raw.deployments) {
    await attest(deployment, "deploy", null, deployment.contract);
    const code = await rpc("eth_getCode", [deployment.contract, "latest"]);
    if (typeof code !== "string" || !/^0x(?:[0-9a-fA-F]{2})+$/.test(code) || code === "0x" ||
        hashBytes(Buffer.from(code.slice(2), "hex")) !== deployment.runtime_code_sha256) {
      throw new Error("Deployed contract runtime hash does not match preflight source trace");
    }
  }
  for (const record of raw.records) {
    for (const variant of plan.variants) {
      const value = record.variants[variant.name];
      await attest(value, "execute", value.contract, null);
    }
  }
  if (checked.length !== plan.total_planned_transactions) {
    throw new Error("On-chain receipt coverage is incomplete");
  }
  return {
    schema: "fhec-cofhe-phase89-rpc-attestation-v1",
    evidence_tier: "arbitrum-sepolia-receipt-corroborated-sdk-decryption-not-independently-proven",
    chain_id: 421614,
    raw_trace_sha256: hashBytes(JSON.stringify(raw)),
    plan_sha256: planDigest(plan),
    source_seal_sha256: raw.source_seal_sha256,
    signer: raw.signer,
    minimum_confirmations: minConfirmations,
    deployment_count: plan.deployment_count,
    execution_count: plan.execution_count,
    verified_transaction_count: checked.length,
    native_chain_fees_paid_wei: raw.native_chain_fees_paid_wei,
    coprocessor_fees_measured: false,
    receipts: checked,
    limitations: [
      "RPC receipts corroborate transaction inclusion, sender, code hash and paid native transaction fees only.",
      "SDK-reported decryption was not independently reproduced by a cryptographic auditor.",
      "CoFHE coprocessor service fees and confidential task prices are not measured by these receipts.",
      "Results are valid for this testnet and workload, not for mainnet deployment or future SDK versions.",
    ],
  };
}

export function parseJson(path) {
  return JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
}

export async function main(args) {
  if (args.length !== 8 || args[0] !== "--plan" || args[2] !== "--raw" || args[4] !== "--out" || args[6] !== "--confirmations") {
    throw new Error("Usage: --plan <plan> --raw <sdk-trace> --out <rpc-attestation> --confirmations 2");
  }
  const rpc = createRpc(process.env.FHEC_TESTNET_RPC_URL);
  const document = await verifyChain(loadPlan(args[1]), parseJson(args[3]), rpc, Number(args[7]));
  writeFileSync(args[5], JSON.stringify(document, null, 2) + "\n", { flag: "wx" });
  process.stdout.write("Corroborated " + document.verified_transaction_count + " Arbitrum Sepolia transaction receipts and deployment bytecode hashes.\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch((error) => {
    process.stderr.write("Independent Phase 8/9 RPC check failed: " + error.message + "\n");
    process.exitCode = 1;
  });
}
