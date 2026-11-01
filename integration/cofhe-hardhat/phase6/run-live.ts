import { Encryptable, FheTypes } from "@cofhe/sdk";
import { writeFileSync } from "node:fs";
import hre from "hardhat";

type LiveCase = { label: string; operation: number; a: bigint; b: bigint; expected: bigint | boolean; type: number };

const liveCases: LiveCase[] = [
  { label: "maximum-u8", operation: 4, a: 19n, b: 41n, expected: 41n, type: FheTypes.Uint8 },
  { label: "addition-wrap-u8", operation: 0, a: 255n, b: 1n, expected: 0n, type: FheTypes.Uint8 },
  { label: "greater-u8", operation: 3, a: 241n, b: 12n, expected: true, type: FheTypes.Bool },
];

async function main() {
  if (process.env.FHEC_PHASE6_LIVE_ALLOW_SEND !== "YES_I_ACCEPT_TESTNET_TRANSACTIONS") {
    throw new Error("Live transaction consent is missing");
  }
  if (hre.network.name !== "arb-sepolia") {
    throw new Error("Only Arbitrum Sepolia is permitted for this Phase 6 deployment");
  }
  if (!process.env.FHEC_TESTNET_RPC_URL?.startsWith("https://")) {
    throw new Error("FHEC_TESTNET_RPC_URL must be an HTTPS RPC endpoint");
  }
  if (!/^0x[a-fA-F0-9]{64}$/.test(process.env.FHEC_TESTNET_PRIVATE_KEY || "")) {
    throw new Error("FHEC_TESTNET_PRIVATE_KEY is missing or malformed");
  }
  const destination = process.env.FHEC_PHASE6_LIVE_EVIDENCE_FILE;
  if (!destination) throw new Error("Missing live evidence destination");

  const network = await hre.ethers.provider.getNetwork();
  if (network.chainId !== 421614n) throw new Error("Wrong network chain ID; no deployment attempted");
  const [owner] = await hre.ethers.getSigners();
  if (!owner) throw new Error("Dedicated testnet wallet is not configured");
  const balance = await hre.ethers.provider.getBalance(owner.address);
  if (balance === 0n) throw new Error("Test wallet has no testnet gas balance");

  const factory = await hre.ethers.getContractFactory("FHECPhase6Live");
  const contract = await factory.connect(owner).deploy();
  const deployReceipt = await contract.deploymentTransaction()?.wait();
  if (!deployReceipt || deployReceipt.status !== 1) throw new Error("Live contract deployment failed");
  await contract.waitForDeployment();
  const address = await contract.getAddress();
  const client = await hre.cofhe.createClientWithBatteries(owner);

  const samples: Array<Record<string, string | number | boolean>> = [];
  for (const testCase of liveCases) {
    const left = await client.encryptInputs([Encryptable.uint8(testCase.a)]).setConsumingContract(address).execute();
    const right = await client.encryptInputs([Encryptable.uint8(testCase.b)]).setConsumingContract(address).execute();
    const submissionStarted = Date.now();
    const transaction = await contract.connect(owner).getFunction("runU8").send(
      testCase.operation, left[0], left[1], right[0], right[1],
    );
    const receipt = await transaction.wait();
    if (!receipt || receipt.status !== 1) throw new Error(`Transaction reverted: ${testCase.label}`);
    const handle = await contract.getFunction(testCase.operation === 3 ? "readBoolU8" : "readValueU8").staticCall(owner.address);

    let decrypted: unknown = undefined;
    let lastError: unknown = undefined;
    const started = Date.now();
    for (let attempt = 0; attempt < 12; attempt += 1) {
      try {
        decrypted = await client.decryptForView(handle, testCase.type).execute();
        break;
      } catch (error) {
        lastError = error;
        if (attempt < 11) await new Promise((resolve) => setTimeout(resolve, 5000));
      }
    }
    if (decrypted === undefined) {
      throw new Error(`FHE result did not become decryptable after retries: ${testCase.label}`);
    }
    if (decrypted !== testCase.expected) {
      throw new Error(`FHE result diverged from independent oracle: ${testCase.label}`);
    }
    samples.push({
      label: testCase.label,
      operation: testCase.operation,
      input_a: testCase.a.toString(),
      input_b: testCase.b.toString(),
      expected: String(testCase.expected),
      actual: String(decrypted),
      transaction_hash: receipt.hash,
      block_number: receipt.blockNumber,
      transaction_gas_used: receipt.gasUsed.toString(),
      transaction_submission_ms: Date.now() - submissionStarted,
      result_wait_ms: Date.now() - started,
      matched_independent_oracle: true,
    });
  }

  const report = {
    schema: "fhec-cofhe-phase6-live-testnet-v1",
    environment: "Arbitrum Sepolia testnet, intended real CoFHE SDK lifecycle",
    chain_id: Number(network.chainId),
    contract: address,
    signer: owner.address,
    deployment_transaction: deployReceipt.hash,
    deployment_block: deployReceipt.blockNumber,
    completed_encrypted_cases: samples.length,
    operation_coverage: ["max-u8", "add-u8", "gt-u8"],
    records: samples,
    caveats: [
      "This evidence is valid only if the receipt hashes and SDK result logs are independently verified.",
      "Only three 8-bit operations are covered by this optional live script.",
      "The reported gas excludes FHE coprocessor execution fees and SDK encryption time.",
      "No public decryption path, third-party independent audit, or mainnet authorization is claimed.",
    ],
  };
  writeFileSync(destination, `${JSON.stringify(report, null, 2)}\n`, { flag: "wx" });
  process.stdout.write(`Phase 6 live testnet: ${samples.length} encrypted cases matched the independent oracle.\n`);
  process.stdout.write(`Deployment transaction: ${deployReceipt.hash}\n`);
}

main().catch((error) => {
  process.stderr.write(`Phase 6 live testnet stopped: ${error.message}\n`);
  process.exitCode = 1;
});
