import { Encryptable, FheTypes } from "@cofhe/sdk";
import { createHash } from "node:crypto";
import { appendFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative, resolve, sep } from "node:path";
import hre from "hardhat";

type Case = {
  id: string;
  width: number;
  operation: string;
  operation_selector: number;
  input_a: string;
  input_b: string;
  expected: string | boolean;
};
type Variant = { name: string; contract: string };

const consent = "YES_I_ACCEPT_ARBITRUM_SEPOLIA_TRANSACTIONS_AND_FEES";
const requiredSources = [
  "FHECPhase6Optimized.sol", "FHECPhase6Live.sol", "FHECPhase7ManualStraight.sol",
  "FHECPhase7ManualTuned.sol", "FHECPhase7Straight.sol", "FHECPhase7Tuned.sol", "phase89-plan.json",
];
const operationNames = ["add", "sub", "mul", "gt", "max", "repeated"];

function sha256(value: string | Uint8Array): string {
  return createHash("sha256").update(value).digest("hex").toUpperCase();
}

function amount(value: string | undefined, label: string): bigint {
  if (!value || !/^[1-9][0-9]*$/.test(value)) throw new Error(label + " must be a positive decimal-string wei amount");
  return BigInt(value);
}

function json(path: string): any {
  return JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
}

function encryptable(width: number, value: string) {
  const numeric = BigInt(value);
  if (width === 8) return Encryptable.uint8(numeric);
  if (width === 16) return Encryptable.uint16(numeric);
  if (width === 32) return Encryptable.uint32(numeric);
  if (width === 64) return Encryptable.uint64(numeric);
  throw new Error("Unsupported encrypted operand width");
}

function resultType(width: number, operation: string) {
  if (operation === "gt") return FheTypes.Bool;
  if (width === 8) return FheTypes.Uint8;
  if (width === 16) return FheTypes.Uint16;
  if (width === 32) return FheTypes.Uint32;
  if (width === 64) return FheTypes.Uint64;
  throw new Error("Unsupported encrypted result width");
}

function rejectChangedSources(directory: string, sealPath: string) {
  const seal = json(sealPath);
  if (seal.schema !== "fhec-cofhe-phase89-source-seal-v1" || seal.files_count !== requiredSources.length ||
      JSON.stringify(seal.files.map((item: any) => item.file)) !== JSON.stringify(requiredSources)) {
    throw new Error("Prepared source bundle seal is invalid");
  }
  for (const entry of seal.files) {
    const path = join(directory, entry.file);
    const bytes = readFileSync(path);
    if (entry.bytes !== bytes.length || entry.sha256 !== sha256(bytes)) {
      throw new Error("Prepared contract source has changed since compilation: " + entry.file);
    }
  }
  const plan = json(join(directory, "phase89-plan.json"));
  if (seal.plan_sha256 !== sha256(JSON.stringify(plan))) {
    throw new Error("Sealed plan checksum mismatch");
  }
  return { plan, planHash: seal.plan_sha256, sealHash: sha256(readFileSync(sealPath)) };
}

async function main() {
  if (process.env.FHEC_PHASE89_CONSENT !== consent) throw new Error("Explicit Phase 8/9 live-transaction consent is missing");
  if (hre.network.name !== "arb-sepolia") throw new Error("Only arb-sepolia may run the live benchmark");
  if (!process.env.FHEC_TESTNET_RPC_URL?.startsWith("https://")) throw new Error("An HTTPS Arbitrum Sepolia RPC endpoint is required");
  if (!/^0x[0-9a-fA-F]{64}$/.test(process.env.FHEC_TESTNET_PRIVATE_KEY ?? "")) throw new Error("A dedicated testnet private key must be set locally");

  const generated = process.env.FHEC_PHASE89_SOURCES_DIR;
  const sealFile = process.env.FHEC_PHASE89_SEAL_FILE;
  const journal = process.env.FHEC_PHASE89_JOURNAL_FILE;
  const evidence = process.env.FHEC_PHASE89_RAW_EVIDENCE_FILE;
  if (!generated || !sealFile || !journal || !evidence || existsSync(journal) || existsSync(evidence)) {
    throw new Error("Fresh prepared sources and two nonexistent journal/evidence paths are required");
  }
  const fullGenerated = resolve(generated);
  const root = resolve(__dirname, "..");
  const sourcePrefix = relative(root, fullGenerated).split(sep).join("/");
  if (sourcePrefix === "" || sourcePrefix.startsWith("../") || sourcePrefix === ".." ||
      basename(dirname(fullGenerated)) !== "generated") {
    throw new Error("Sources must reside in an isolated phase89/generated/<run-id> directory");
  }

  const { plan, planHash, sealHash } = rejectChangedSources(fullGenerated, sealFile);
  if (plan.chain_id !== 421614 || plan.chain !== "arb-sepolia" ||
      !["smoke6", "coverage24", "comparison72"].includes(plan.profile) ||
      !["optimized-tuned", "three-way"].includes(plan.variant_mode) ||
      plan.total_planned_transactions !== (plan.case_count + 1) * plan.variants.length ||
      plan.case_count !== plan.cases.length || plan.execution_count !== plan.case_count * plan.variants.length ||
      !plan.disclosure.no_live_transactions_without_separate_explicit_consent) {
    throw new Error("Unexpected or malformed prepared live execution plan");
  }
  const allowed = new Map([ ["optimized", "FHECPhase6Live"], ["tuned", "FHECPhase7Tuned"], ["straight", "FHECPhase7Straight"] ]);
  const expectedNames = plan.variant_mode === "three-way" ? ["optimized", "tuned", "straight"] : ["optimized", "tuned"];
  if (JSON.stringify(plan.variants.map((variant: Variant) => variant.name)) !== JSON.stringify(expectedNames) ||
      plan.variants.some((variant: Variant) => allowed.get(variant.name) !== variant.contract)) {
    throw new Error("Prepared variants are not the reviewed contracts");
  }
  for (const item of plan.cases as Case[]) {
    if (!item || ![8, 16, 32, 64].includes(item.width) ||
        operationNames[item.operation_selector] !== item.operation ||
        !/^\w+-u(8|16|32|64)-\d{2}$/.test(item.id)) {
      throw new Error("Unexpected operation in prepared plan");
    }
  }

  const maxGasPriceWei = amount(process.env.FHEC_PHASE89_MAX_GAS_PRICE_WEI, "MaxGasPriceWei");
  const maxChainFeesWei = amount(process.env.FHEC_PHASE89_MAX_NATIVE_FEES_WEI, "MaxNativeFeesWei");
  const txCeiling = Number(process.env.FHEC_PHASE89_MAX_TX_COUNT);
  if (!Number.isSafeInteger(txCeiling) || txCeiling < plan.total_planned_transactions || txCeiling > 500) {
    throw new Error("Explicit MaxTransactions does not cover the exact planned transaction count");
  }

  const network = await hre.ethers.provider.getNetwork();
  if (network.chainId !== 421614n) throw new Error("Connected chain does not match Arbitrum Sepolia");
  const [owner] = await hre.ethers.getSigners();
  if (!owner) throw new Error("A funded, dedicated testnet wallet is required");
  const balance = await hre.ethers.provider.getBalance(owner.address);
  if (balance === 0n) throw new Error("Dedicated testnet wallet has no native gas balance");
  if (balance < maxChainFeesWei) {
    throw new Error("Dedicated wallet balance is below the explicitly configured maximum native-chain fee budget");
  }

  const feeData = await hre.ethers.provider.getFeeData();
  if (feeData.maxFeePerGas == null || feeData.maxPriorityFeePerGas == null ||
      feeData.maxFeePerGas > maxGasPriceWei || feeData.maxPriorityFeePerGas > maxGasPriceWei) {
    throw new Error("RPC gas-price quote exceeds the explicitly authorized EIP-1559 fee cap");
  }
  const overrides = { maxFeePerGas: maxGasPriceWei, maxPriorityFeePerGas: feeData.maxPriorityFeePerGas };
  const signerAddress = owner.address;
  let feesPaidWei = 0n;
  let transactionsSent = 0;
  const deployments: any[] = [];
  const records: any[] = [];
  const contracts = new Map<string, any>();
  const started = new Date().toISOString();
  writeFileSync(journal, JSON.stringify({ schema: "fhec-phase89-journal-v1", stage: "started", plan_sha256: planHash, source_seal_sha256: sealHash, chain_id: 421614, signer: signerAddress, started_at: started }) + "\n", { flag: "wx" });
  const emit = (item: any) => appendFileSync(journal, JSON.stringify(item) + "\n");

  function guardLimit(estimateGas: bigint) {
    const gasLimit = estimateGas + estimateGas / 5n + 10000n;
    if (gasLimit <= 0n || feesPaidWei + gasLimit * maxGasPriceWei > maxChainFeesWei ||
        transactionsSent >= txCeiling) {
      throw new Error("Native chain fee or transaction-count budget prevents another submission");
    }
    return gasLimit;
  }

  function consumeReceipt(receipt: any) {
    if (!receipt || receipt.status !== 1 || !receipt.hash || receipt.gasPrice == null) {
      throw new Error("A transaction failed or has no independently checkable receipt fee");
    }
    const paid = BigInt(receipt.gasUsed) * BigInt(receipt.gasPrice);
    feesPaidWei += paid;
    if (feesPaidWei > maxChainFeesWei) throw new Error("Observed chain fees exceeded authorization ceiling");
    return paid;
  }

  try {
    for (const variant of plan.variants as Variant[]) {
      const qualified = sourcePrefix + "/" + variant.contract + ".sol:" + variant.contract;
      const factory = await hre.ethers.getContractFactory(qualified);
      const deployment = await factory.getDeployTransaction();
      const artifact = await hre.artifacts.readArtifact(qualified);
      if (!deployment.data || deployment.data.toLowerCase() !== artifact.bytecode.toLowerCase()) {
        throw new Error("Unsigned deployment payload is not identical to the expected compiled creation bytecode");
      }
      const gasLimit = guardLimit(await owner.estimateGas(deployment));
      const transactionStart = Date.now();
      const contract = await factory.connect(owner).deploy({ gasLimit, ...overrides });
      transactionsSent += 1;
      const sent = contract.deploymentTransaction();
      if (!sent) throw new Error("Deployment transaction not returned by provider");
      if (!sent.data || sent.data.toLowerCase() !== artifact.bytecode.toLowerCase()) {
        throw new Error("Sent deployment init code differs from approved compiled artifact");
      }
      emit({ stage: "transaction_sent", kind: "deploy", variant: variant.name, transaction_hash: sent.hash });
      const receipt = await sent.wait();
      if (!receipt) throw new Error("Deployment transaction did not return a mined receipt");
      const paid = consumeReceipt(receipt);
      await contract.waitForDeployment();
      const address = await contract.getAddress();
      const code = await hre.ethers.provider.getCode(address);
      if (code === "0x" || artifact.deployedBytecode.toLowerCase() !== code.toLowerCase()) {
        throw new Error("Deployed runtime bytecode does not match the compiled source artifact");
      }
      const entry = {
        variant: variant.name,
        contract: address,
        transaction_hash: receipt.hash,
        calldata_sha256: sha256(Buffer.from(sent.data.slice(2), "hex")),
        block_number: receipt.blockNumber,
        gas_used: receipt.gasUsed.toString(),
        effective_gas_price_wei: receipt.gasPrice.toString(),
        native_chain_fee_wei: paid.toString(),
        runtime_code_sha256: sha256(Buffer.from(code.slice(2), "hex")),
        confirmation_ms: Date.now() - transactionStart,
      };
      deployments.push(entry);
      contracts.set(variant.name, contract);
      emit({ stage: "deployment_verified", ...entry });
    }

    const client = await hre.cofhe.createClientWithBatteries(owner);
    for (const item of plan.cases as Case[]) {
      const values: Record<string, any> = {};
      for (const variant of plan.variants as Variant[]) {
        const contract = contracts.get(variant.name)!;
        const address = await contract.getAddress();
        const encryptedAt = Date.now();
        const left = await client.encryptInputs([encryptable(item.width, item.input_a)]).setConsumingContract(address).execute();
        const right = await client.encryptInputs([encryptable(item.width, item.input_b)]).setConsumingContract(address).execute();
        const encryptionMs = Date.now() - encryptedAt;
        const method = contract.connect(owner).getFunction("runU" + item.width);
        const argumentsForCall = [item.operation_selector, left[0], left[1], right[0], right[1]];
        const gasLimit = guardLimit(await method.estimateGas(...argumentsForCall));
        const confirmedAt = Date.now();
        const transaction = await method.send(...argumentsForCall, { gasLimit, ...overrides });
        transactionsSent += 1;
        if (!transaction.data || !/^0x(?:[a-fA-F0-9]{2})+$/.test(transaction.data)) {
          throw new Error("Sent execution transaction is missing required calldata");
        }
        emit({ stage: "transaction_sent", kind: "execute", case_id: item.id, variant: variant.name, transaction_hash: transaction.hash });
        const receipt = await transaction.wait();
        if (!receipt) throw new Error("Encrypted execution did not return a mined receipt");
        const paid = consumeReceipt(receipt);
        const confirmationMs = Date.now() - confirmedAt;
        const handle = await contract.getFunction((item.operation === "gt" ? "readBoolU" : "readValueU") + item.width).staticCall(signerAddress);
        const decryptAt = Date.now();
        let actual: unknown;
        let finalError: unknown;
        for (let attempt = 0; attempt < 12; attempt += 1) {
          try {
            actual = await client.decryptForView(handle, resultType(item.width, item.operation)).execute();
            break;
          } catch (error) {
            finalError = error;
            if (attempt !== 11) await new Promise((done) => setTimeout(done, 5000));
          }
        }
        if (actual === undefined || String(actual) !== String(item.expected)) {
          throw new Error("Live SDK decryption was unavailable or did not match the oracle: " + item.id + "/" + variant.name + "/" + String(finalError ? "retried" : "mismatch"));
        }
        const observation = {
          contract: address,
          actual: String(actual),
          transaction_hash: receipt.hash,
          calldata_sha256: sha256(Buffer.from(transaction.data.slice(2), "hex")),
          block_number: receipt.blockNumber,
          gas_used: receipt.gasUsed.toString(),
          effective_gas_price_wei: receipt.gasPrice.toString(),
          native_chain_fee_wei: paid.toString(),
          sdk_encrypt_ms: encryptionMs,
          confirmation_ms: confirmationMs,
          sdk_decrypt_wait_ms: Date.now() - decryptAt,
          sdk_result_matched_oracle: true,
          coprocessor_fee_wei: null,
        };
        values[variant.name] = observation;
        emit({ stage: "execution_verified_by_sdk", case_id: item.id, variant: variant.name, ...observation });
      }
      records.push({ id: item.id, width: item.width, operation: item.operation, expected: item.expected, variants: values });
    }

    if (records.length !== plan.case_count || transactionsSent !== plan.total_planned_transactions) {
      throw new Error("Live execution did not complete the approved plan");
    }
    const report = {
      schema: "fhec-cofhe-phase89-live-sdk-trace-v1",
      evidence_tier: "provisional-sdk-live-results-pending-independent-rpc-review",
      chain: "arb-sepolia",
      chain_id: 421614,
      profile: plan.profile,
      variant_mode: plan.variant_mode,
      plan_sha256: planHash,
      source_seal_sha256: sealHash,
      signer: signerAddress,
      started_at: started,
      completed_at: new Date().toISOString(),
      case_count: plan.case_count,
      transaction_count: transactionsSent,
      deployments,
      records,
      native_chain_fees_paid_wei: feesPaidWei.toString(),
      coprocessor_fees_measured: false,
      limitations: [
        "SDK view decryption is a user-observed result, not independently attested cryptographic execution.",
        "Native chain transaction fees do not establish CoFHE coprocessor charges.",
        "Selected demo vectors and execution order are deterministic, not representative of all workloads.",
      ],
    };
    writeFileSync(evidence, JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
    emit({ stage: "completed", transaction_count: transactionsSent, native_chain_fees_paid_wei: feesPaidWei.toString() });
    process.stdout.write("Phase 8 live SDK flow completed " + plan.case_count + " cases and " + transactionsSent + " transactions; independent RPC audit remains required.\n");
  } catch (error) {
    emit({ stage: "failed_or_incomplete", transactions_sent: transactionsSent, native_chain_fees_paid_wei: feesPaidWei.toString(), error_summary: String(error instanceof Error ? error.message : error).slice(0, 250) });
    throw error;
  }
}

main().catch((error) => {
  process.stderr.write("Phase 8/9 testnet run stopped: " + String(error instanceof Error ? error.message : error).slice(0, 300) + "\n");
  process.exitCode = 1;
});
