import { loadFixture } from "@nomicfoundation/hardhat-toolbox/network-helpers";
import { Encryptable, FheTypes } from "@cofhe/sdk";
import { expect } from "chai";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import hre from "hardhat";

const MOCKS = "task:cofhe-mocks:deploy";
const requested = Number(process.env.FHEC_PHASE3_CASES ?? "24");
if (!Number.isInteger(requested) || requested < 1 || requested > 128) {
  throw new Error("FHEC_PHASE3_CASES must be an integer from 1 through 128");
}

type Vector = {
  id: string;
  operation: string;
  width: number;
  a: string;
  b: string;
  expected: string;
};

const repeatedVectors = readFileSync(join(__dirname, "../testdata/cofhe-vectors.jsonl"), "utf8")
  .trimEnd()
  .split("\n")
  .map((line) => JSON.parse(line) as Vector)
  .filter((vector) => vector.width === 8 && vector.operation === "repeated");

if (repeatedVectors.length !== 256) throw new Error("Incomplete u8 encrypted vector partition");
const vectors = Array.from(
  { length: requested },
  (_, index) => repeatedVectors[(index * 73) % repeatedVectors.length],
);

describe("FHEC V2 3-way encrypted differential suite", function () {
  async function deployFixture() {
    await hre.run(MOCKS);
    const [owner] = await hre.ethers.getSigners();
    const factory = await hre.ethers.getContractFactory("FHECThreeWayComparison");
    const contract = await factory.connect(owner).deploy();
    await contract.waitForDeployment();
    const address = await contract.getAddress();
    const client = await hre.cofhe.createClientWithBatteries(owner);
    return { owner, contract, address, client };
  }

  for (const vector of vectors) {
    it(`agrees with the u8 modular oracle: ${vector.id}`, async function () {
      const { owner, contract, address, client } = await loadFixture(deployFixture);
      const a = BigInt(vector.a);
      const b = BigInt(vector.b);
      const result = BigInt(vector.expected);
      const gas: bigint[] = [];
      for (const [method, getter] of [
        ["runBaseline", "baselineValue"],
        ["runOptimized", "optimizedValue"],
        ["runManual", "manualValue"],
      ] as const) {
        const encryptedA = await client.encryptInputs([Encryptable.uint8(a)])
          .setConsumingContract(address).execute();
        const encryptedB = await client.encryptInputs([Encryptable.uint8(b)])
          .setConsumingContract(address).execute();
        const receipt = await (await contract.connect(owner)[method](
          encryptedA[0], encryptedA[1], encryptedB[0], encryptedB[1],
        )).wait();
        expect(receipt).not.to.equal(null);
        gas.push(receipt!.gasUsed);
        const handle = await contract[getter]();
        expect(await client.decryptForView(handle, FheTypes.Uint8).execute()).to.equal(result);
      }
      console.log(
        `MOCK ONLY ${vector.id}: baseline=${gas[0]} optimized=${gas[1]} manual=${gas[2]}`,
      );
    });
  }
});
