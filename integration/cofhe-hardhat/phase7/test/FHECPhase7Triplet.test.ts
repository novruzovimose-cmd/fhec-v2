import { Encryptable, FheTypes } from "@cofhe/sdk";
import { expect } from "chai";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import hre from "hardhat";

const matrixFile = process.env.FHEC_PHASE7_MATRIX_FILE || join(__dirname, "../generated/phase7-matrix.json");
const sourceDirectory = process.env.FHEC_PHASE7_SOURCES_DIR || join(__dirname, "../generated");
const contractSource = relative(resolve(__dirname, "../.."), resolve(sourceDirectory)).split(sep).join("/");
if (contractSource.startsWith("../") || contractSource === ".." || !contractSource) {
  throw new Error("Phase 7 Solidity must be generated inside the pinned Hardhat integration");
}
const matrix = JSON.parse(readFileSync(matrixFile, "utf8").replace(/^\uFEFF/, ""));
if (matrix.schema !== "fhec-cofhe-phase6-boundary-matrix-v1" ||
    !Array.isArray(matrix.cases) || matrix.family_count !== 24 ||
    matrix.cases.length !== matrix.total_cases) throw new Error("Missing or malformed Phase 7 test matrix");

const names = [
  { name: "optimized", contract: "FHECPhase6Live" },
  { name: "straight", contract: "FHECPhase7Straight" },
  { name: "tuned", contract: "FHECPhase7Tuned" },
];
const index = ["add", "sub", "mul", "gt", "max", "repeated"];
const records: Array<Record<string, unknown>> = [];
const matrixSha256 = createHash("sha256").update(JSON.stringify(matrix)).digest("hex").toUpperCase();

function input(width: number, n: bigint) {
  if (width === 8) return Encryptable.uint8(n);
  if (width === 16) return Encryptable.uint16(n);
  if (width === 32) return Encryptable.uint32(n);
  if (width === 64) return Encryptable.uint64(n);
  throw new Error("Unsupported input width");
}

function outputType(width: number, operation: string) {
  if (operation === "gt") return FheTypes.Bool;
  if (width === 8) return FheTypes.Uint8;
  if (width === 16) return FheTypes.Uint16;
  if (width === 32) return FheTypes.Uint32;
  if (width === 64) return FheTypes.Uint64;
  throw new Error("Unsupported output width");
}

describe("FHEC Phase 7 identical-wrapper three-way differential benchmarks", function () {
  this.timeout(240000);
  let owner: any;
  let stranger: any;
  let client: any;
  const contracts = new Map<string, any>();
  const addresses = new Map<string, string>();

  before(async function () {
    await hre.run("task:cofhe-mocks:deploy");
    [owner, stranger] = await hre.ethers.getSigners();
    client = await hre.cofhe.createClientWithBatteries(owner);
    for (const variant of names) {
      const factory = await hre.ethers.getContractFactory(
        contractSource + "/" + variant.contract + ".sol:" + variant.contract,
      );
      const contract = await factory.connect(owner).deploy();
      await contract.waitForDeployment();
      contracts.set(variant.name, contract);
      addresses.set(variant.name, await contract.getAddress());
    }
  });

  for (const testCase of matrix.cases) {
    it("three-way equivalence for " + testCase.id, async function () {
      const record: Record<string, unknown> = {
        id: testCase.id,
        width: testCase.width,
        operation: testCase.operation,
        expected: testCase.expected,
        variants: {},
      };
      const triplets: Record<string, unknown> = {};
      for (const variant of names) {
        const address = addresses.get(variant.name)!;
        const contract = contracts.get(variant.name)!;
        const [left, right] = await Promise.all([
          client.encryptInputs([input(testCase.width, BigInt(testCase.a))]).setConsumingContract(address).execute(),
          client.encryptInputs([input(testCase.width, BigInt(testCase.b))]).setConsumingContract(address).execute(),
        ]);
        const started = process.hrtime.bigint();
        const transaction = await contract.connect(owner).getFunction("runU" + testCase.width).send(
          index.indexOf(testCase.operation), left[0], left[1], right[0], right[1],
        );
        const receipt = await transaction.wait();
        expect(receipt).not.to.equal(null);
        expect(receipt!.status).to.equal(1);
        const handle = await contract.getFunction(
          (testCase.operation === "gt" ? "readBoolU" : "readValueU") + testCase.width,
        ).staticCall(owner.address);
        const decrypted = await client.decryptForView(handle, outputType(testCase.width, testCase.operation)).execute();
        const elapsedMs = (process.hrtime.bigint() - started) / 1000000n;
        expect(String(decrypted)).to.equal(String(testCase.expected));
        triplets[variant.name] = {
          result: String(decrypted),
          gas_used: receipt!.gasUsed.toString(),
          elapsed_ms: elapsedMs.toString(),
          transaction_hash: receipt!.hash,
        };
      }
      record.variants = triplets;
      records.push(record);
    });
  }

  it("rejects a proof bound to the wrong comparison harness", async function () {
    const target = contracts.get("optimized")!;
    const original = addresses.get("straight")!;
    const valid = addresses.get("optimized")!;
    const left = await client.encryptInputs([Encryptable.uint8(5n)]).setConsumingContract(original).execute();
    const right = await client.encryptInputs([Encryptable.uint8(9n)]).setConsumingContract(valid).execute();
    await expect(target.connect(owner).getFunction("runU8").send(0, left[0], left[1], right[0], right[1])).to.be.reverted;
  });

  it("rejects ciphertext signed by another sender", async function () {
    const target = contracts.get("straight")!;
    const address = addresses.get("straight")!;
    const left = await client.encryptInputs([Encryptable.uint16(2n)]).setConsumingContract(address).execute();
    const right = await client.encryptInputs([Encryptable.uint16(5n)]).setConsumingContract(address).execute();
    await expect(target.connect(stranger).getFunction("runU16").send(0, left[0], left[1], right[0], right[1])).to.be.reverted;
  });

  it("keeps view-decryption permissions account-scoped", async function () {
    const target = contracts.get("tuned")!;
    const address = addresses.get("tuned")!;
    const left = await client.encryptInputs([Encryptable.uint8(11n)]).setConsumingContract(address).execute();
    const right = await client.encryptInputs([Encryptable.uint8(13n)]).setConsumingContract(address).execute();
    await target.connect(owner).getFunction("runU8").send(4, left[0], left[1], right[0], right[1]);
    const handle = await target.getFunction("readValueU8").staticCall(owner.address);
    expect(await client.decryptForView(handle, FheTypes.Uint8).execute()).to.equal(13n);
    const otherClient = await hre.cofhe.createClientWithBatteries(stranger);
    let rejected = false;
    try { await otherClient.decryptForView(handle, FheTypes.Uint8).execute(); }
    catch { rejected = true; }
    expect(rejected).to.equal(true);
  });

  after(function () {
    const output = process.env.FHEC_PHASE7_MOCK_EVIDENCE_FILE;
    if (!output) return;
    if (records.length !== matrix.total_cases) {
      throw new Error("Refusing to write partial Phase 7 mock evidence");
    }
    const raw = {
      schema: "fhec-cofhe-phase7-mock-triplet-v1",
      environment: "LOCAL_HARDHAT_MOCK_NOT_LIVE_FHE",
      matrix_sha256: matrixSha256,
      case_count: matrix.total_cases,
      records,
      limitations: [
        "These are Hardhat mock gas amounts, not actual CoFHE network gas or coprocessor fees.",
        "A handwritten Solidity reference is not automatically an optimal hand-written implementation.",
        "Local elapsed time includes simulated FHE and mock receipt handling.",
      ],
    };
    writeFileSync(output, JSON.stringify(raw, null, 2) + "\n", { flag: "wx" });
  });
});
