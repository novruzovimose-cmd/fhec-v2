import { Encryptable, FheTypes } from "@cofhe/sdk";
import { expect } from "chai";
import { readFileSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import hre from "hardhat";

const sources = process.env.FHEC_PHASE89_SOURCES_DIR || join(__dirname, "../generated");
const pathPrefix = relative(resolve(__dirname, "../.."), resolve(sources)).split(sep).join("/");
const plan = JSON.parse(readFileSync(join(sources, "phase89-plan.json"), "utf8").replace(/^\uFEFF/, ""));
if (!pathPrefix || pathPrefix.startsWith("../") || plan.schema !== "fhec-cofhe-phase89-live-plan-v1" ||
    plan.case_count !== plan.cases.length || plan.chain_id !== 421614) {
  throw new Error("Phase 8/9 mock preflight requires a correctly prepared, isolated plan");
}

function encrypted(width: number, value: string) {
  const n = BigInt(value);
  if (width === 8) return Encryptable.uint8(n);
  if (width === 16) return Encryptable.uint16(n);
  if (width === 32) return Encryptable.uint32(n);
  if (width === 64) return Encryptable.uint64(n);
  throw new Error("Unsupported width");
}

function type(width: number, operation: string) {
  if (operation === "gt") return FheTypes.Bool;
  if (width === 8) return FheTypes.Uint8;
  if (width === 16) return FheTypes.Uint16;
  if (width === 32) return FheTypes.Uint32;
  if (width === 64) return FheTypes.Uint64;
  throw new Error("Unsupported width");
}

describe("FHEC Phase 8/9 prepared source lifecycle local preflight", function () {
  this.timeout(240000);
  const contracts = new Map<string, any>();
  let owner: any;
  let client: any;

  before(async function () {
    await hre.run("task:cofhe-mocks:deploy");
    [owner] = await hre.ethers.getSigners();
    client = await hre.cofhe.createClientWithBatteries(owner);
    for (const variant of plan.variants) {
      const qualified = pathPrefix + "/" + variant.contract + ".sol:" + variant.contract;
      const contract = await (await hre.ethers.getContractFactory(qualified)).connect(owner).deploy();
      await contract.waitForDeployment();
      contracts.set(variant.name, contract);
    }
  });

  for (const testCase of plan.cases) {
    it("preserves the encrypted oracle through prepared harnesses: " + testCase.id, async function () {
      for (const variant of plan.variants) {
        const contract = contracts.get(variant.name)!;
        const address = await contract.getAddress();
        const left = await client.encryptInputs([encrypted(testCase.width, testCase.input_a)]).setConsumingContract(address).execute();
        const right = await client.encryptInputs([encrypted(testCase.width, testCase.input_b)]).setConsumingContract(address).execute();
        const transaction = await contract.connect(owner).getFunction("runU" + testCase.width).send(
          testCase.operation_selector, left[0], left[1], right[0], right[1],
        );
        const receipt = await transaction.wait();
        expect(receipt).to.not.equal(null);
        expect(receipt!.status).to.equal(1);
        const handle = await contract.getFunction((testCase.operation === "gt" ? "readBoolU" : "readValueU") + testCase.width).staticCall(owner.address);
        const actual = await client.decryptForView(handle, type(testCase.width, testCase.operation)).execute();
        expect(String(actual)).to.equal(String(testCase.expected));
      }
    });
  }

  it("rejects a proof prepared for another consuming contract", async function () {
    const leftContract = contracts.get(plan.variants[0].name)!;
    const rightContract = contracts.get(plan.variants[1].name)!;
    const wrong = await client.encryptInputs([Encryptable.uint8(7n)]).setConsumingContract(await rightContract.getAddress()).execute();
    const correct = await client.encryptInputs([Encryptable.uint8(9n)]).setConsumingContract(await leftContract.getAddress()).execute();
    await expect(leftContract.connect(owner).getFunction("runU8").send(0, wrong[0], wrong[1], correct[0], correct[1])).to.be.reverted;
  });
});
