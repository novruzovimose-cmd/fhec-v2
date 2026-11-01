import { loadFixture } from "@nomicfoundation/hardhat-toolbox/network-helpers";
import { Encryptable, FheTypes } from "@cofhe/sdk";
import { expect } from "chai";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import hre from "hardhat";

const MOCK_DEPLOY_TASK = "task:cofhe-mocks:deploy";
const widthValues = [8, 16, 32, 64] as const;
const operations = ["add", "sub", "mul", "gt", "max", "repeated"] as const;
const variants = [
  { method: "Baseline", storage: "baseline", name: "baseline" },
  { method: "Optimized", storage: "optimized", name: "optimized" },
  { method: "Manual", storage: "manual", name: "manual" },
] as const;

type Width = (typeof widthValues)[number];
type Operation = (typeof operations)[number];
type Vector = {
  schema: string;
  id: string;
  width: Width;
  operation: string;
  a: string;
  b: string;
  expected: string;
};
type Sample = {
  width: Width;
  operation: Operation;
  vector_id: string;
  operands: { a: string; b: string };
  expected: string | boolean;
  mock_gas: { baseline: string; optimized: string; manual: string };
};

const perFamily = Number(process.env.FHEC_PHASE4_CASES_PER_FAMILY ?? "2");
if (!Number.isInteger(perFamily) || perFamily < 1 || perFamily > 8) {
  throw new Error("FHEC_PHASE4_CASES_PER_FAMILY must be an integer between 1 and 8");
}

const corpusPath = join(__dirname, "../testdata/cofhe-vectors.jsonl");
const corpus = readFileSync(corpusPath, "utf8")
  .trimEnd()
  .split("\n")
  .map((line) => JSON.parse(line) as Vector);
const vectorsByWidth = new Map<Width, Vector[]>();
for (const width of widthValues) {
  const vectors = corpus.filter((row) => row.width === width && row.operation === "add");
  if (vectors.length !== 256 || vectors.some((row) => row.schema !== "fhec-cofhe-test-vector-v1")) {
    throw new Error(`Missing or incompatible ${width}-bit differential input partition`);
  }
  vectorsByWidth.set(width, vectors);
}

function encryptable(width: Width, value: bigint) {
  switch (width) {
    case 8: return Encryptable.uint8(value);
    case 16: return Encryptable.uint16(value);
    case 32: return Encryptable.uint32(value);
    case 64: return Encryptable.uint64(value);
  }
}

function resultType(width: Width, operation: Operation) {
  if (operation === "gt") return FheTypes.Bool;
  switch (width) {
    case 8: return FheTypes.Uint8;
    case 16: return FheTypes.Uint16;
    case 32: return FheTypes.Uint32;
    case 64: return FheTypes.Uint64;
  }
}

function independentOracle(width: Width, operation: Operation, a: bigint, b: bigint): bigint | boolean {
  const mask = (1n << BigInt(width)) - 1n;
  switch (operation) {
    case "add": return (a + b) & mask;
    case "sub": return (a - b) & mask;
    case "mul": return (a * b) & mask;
    case "gt": return a > b;
    case "max": return a > b ? a : b;
    case "repeated": return (a * b * 2n) & mask;
  }
}

const samples: Sample[] = [];

describe("FHEC V2 Phase 4 width-aware CoFHE differential suite", function () {
  this.timeout(120000);

  async function deployFixture() {
    await hre.run(MOCK_DEPLOY_TASK);
    const [owner, stranger] = await hre.ethers.getSigners();
    const factory = await hre.ethers.getContractFactory("FHECPhase4Comparison");
    const contract = await factory.connect(owner).deploy();
    await contract.waitForDeployment();
    const address = await contract.getAddress();
    const client = await hre.cofhe.createClientWithBatteries(owner);
    return { owner, stranger, contract, address, client };
  }

  async function encryptOne(
    client: Awaited<ReturnType<typeof deployFixture>>["client"],
    contractAddress: string,
    width: Width,
    number: bigint,
  ) {
    const [hash, proof] = await client.encryptInputs([encryptable(width, number)])
      .setConsumingContract(contractAddress).execute();
    return { hash, proof };
  }

  for (const width of widthValues) {
    for (const [operationIndex, operation] of operations.entries()) {
      const inputPool = vectorsByWidth.get(width)!;
      for (let caseIndex = 0; caseIndex < perFamily; caseIndex++) {
        const vector = inputPool[(caseIndex * 73 + operationIndex * 41 + width / 8 * 19) % inputPool.length];
        const a = BigInt(vector.a);
        const b = BigInt(vector.b);
        const expected = independentOracle(width, operation, a, b);

        it(`matches handwritten Solidity and modular oracle for ${operation}/u${width}/${caseIndex}`, async function () {
          const { owner, contract, address, client } = await loadFixture(deployFixture);
          const gas = { baseline: "", optimized: "", manual: "" };

          for (const variant of variants) {
            const left = await encryptOne(client, address, width, a);
            const right = await encryptOne(client, address, width, b);
            const runMethod = `run${variant.method}U${width}`;
            const receipt = await (await contract.connect(owner)[runMethod](
              operationIndex,
              left.hash, left.proof,
              right.hash, right.proof,
            )).wait();
            expect(receipt).not.to.equal(null);

            const valueGetter = operation === "gt"
              ? `${variant.storage}Bool${width}`
              : `${variant.storage}Value${width}`;
            const ciphertext = await contract[valueGetter]();
            const decrypted = await client.decryptForView(ciphertext, resultType(width, operation)).execute();
            expect(decrypted, `${variant.name} yielded an incorrect ${operation}/u${width} value`).to.equal(expected);
            gas[variant.name] = receipt!.gasUsed.toString();
          }

          samples.push({
            width,
            operation,
            vector_id: vector.id,
            operands: { a: a.toString(), b: b.toString() },
            expected: typeof expected === "bigint" ? expected.toString() : expected,
            mock_gas: gas,
          });
        });
      }
    }
  }

  it("rejects inputs encrypted for a different contract", async function () {
    const { owner, stranger, contract, address, client } = await loadFixture(deployFixture);
    const left = await encryptOne(client, stranger.address, 8, 15n);
    const right = await encryptOne(client, address, 8, 23n);
    await expect(contract.connect(owner).runBaselineU8(
      0, left.hash, left.proof, right.hash, right.proof,
    )).to.be.reverted;
  });

  it("rejects ciphertext proofs from a different signer", async function () {
    const { stranger, contract, address, client } = await loadFixture(deployFixture);
    const left = await encryptOne(client, address, 64, 15n);
    const right = await encryptOne(client, address, 64, 23n);
    await expect(contract.connect(stranger).runBaselineU64(
      0, left.hash, left.proof, right.hash, right.proof,
    )).to.be.reverted;
  });

  it("rejects a proof paired to another ciphertext", async function () {
    const { owner, contract, address, client } = await loadFixture(deployFixture);
    const left = await encryptOne(client, address, 16, 15n);
    const right = await encryptOne(client, address, 16, 23n);
    await expect(contract.connect(owner).runBaselineU16(
      0, left.hash, right.proof, right.hash, left.proof,
    )).to.be.reverted;
  });

  it("does not permit a different signer to decrypt the stored result", async function () {
    const { owner, stranger, contract, address, client } = await loadFixture(deployFixture);
    const left = await encryptOne(client, address, 32, 15n);
    const right = await encryptOne(client, address, 32, 23n);
    await contract.connect(owner).runBaselineU32(
      0, left.hash, left.proof, right.hash, right.proof,
    );
    const handle = await contract.baselineValue32();
    expect(await client.decryptForView(handle, FheTypes.Uint32).execute()).to.equal(38n);
    const strangerClient = await hre.cofhe.createClientWithBatteries(stranger);
    let blocked = false;
    try {
      await strangerClient.decryptForView(handle, FheTypes.Uint32).execute();
    } catch {
      blocked = true;
    }
    expect(blocked).to.equal(true);
  });

  after(function () {
    const output = process.env.FHEC_PHASE4_EVIDENCE_FILE;
    if (!output) return;
    const payload = {
      schema: "fhec-cofhe-phase4-mock-samples-v1",
      environment: "Fhenix Hardhat CoFHE MOCK (not live cryptographic execution)",
      cases_per_family: perFamily,
      family_count: widthValues.length * operations.length,
      expected_comparisons: widthValues.length * operations.length * perFamily,
      completed_comparisons: samples.length,
      samples,
      limitations: [
        "The recorded gas is Hardhat mock transaction gas, not live network gas or coprocessor fees.",
        "Input encryption and network deployment are excluded from the reported transaction gas.",
        "Variants have distinct method selectors and storage slots; differences also include wrapper overhead.",
      ],
    };
    writeFileSync(output, `${JSON.stringify(payload, null, 2)}\n`, { flag: "wx" });
  });
});
