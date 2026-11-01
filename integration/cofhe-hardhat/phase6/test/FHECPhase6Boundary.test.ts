import { Encryptable, FheTypes } from "@cofhe/sdk";
import { expect } from "chai";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import hre from "hardhat";

const matrix = JSON.parse(readFileSync(process.env.FHEC_PHASE6_MATRIX_FILE || join(__dirname, "../generated/phase6-matrix.json"), "utf8"));
if (matrix.schema !== "fhec-cofhe-phase6-boundary-matrix-v1" || !Array.isArray(matrix.cases) || matrix.family_count !== 24 || matrix.cases.length !== matrix.total_cases) {
  throw new Error("Phase 6 boundary matrix is malformed");
}

const operationIndex = ["add", "sub", "mul", "gt", "max", "repeated"];
const records: Array<Record<string, unknown>> = [];

function encrypted(width: number, value: bigint) {
  switch (width) {
    case 8: return Encryptable.uint8(value);
    case 16: return Encryptable.uint16(value);
    case 32: return Encryptable.uint32(value);
    case 64: return Encryptable.uint64(value);
    default: throw new Error("Unsupported encrypted width");
  }
}

function resultType(width: number, operation: string) {
  if (operation === "gt") return FheTypes.Bool;
  switch (width) {
    case 8: return FheTypes.Uint8;
    case 16: return FheTypes.Uint16;
    case 32: return FheTypes.Uint32;
    case 64: return FheTypes.Uint64;
    default: throw new Error("Unsupported result width");
  }
}

describe("FHEC Phase 6 boundary, overflow and ACL matrix", function () {
  this.timeout(180000);

  async function fixture() {
    await hre.run("task:cofhe-mocks:deploy");
    const [owner, stranger] = await hre.ethers.getSigners();
    const factory = await hre.ethers.getContractFactory("FHECPhase6Live");
    const contract = await factory.connect(owner).deploy();
    await contract.waitForDeployment();
    const address = await contract.getAddress();
    const client = await hre.cofhe.createClientWithBatteries(owner);
    return { owner, stranger, contract, address, client };
  }

  for (const row of matrix.cases) {
    it(`matches independent unsigned oracle for ${row.id}`, async function () {
      const { owner, contract, address, client } = await fixture();
      const a = BigInt(row.a);
      const b = BigInt(row.b);
      const [left, right] = await Promise.all([
        client.encryptInputs([encrypted(row.width, a)]).setConsumingContract(address).execute(),
        client.encryptInputs([encrypted(row.width, b)]).setConsumingContract(address).execute(),
      ]);
      const started = process.hrtime.bigint();
      const submit = contract.connect(owner).getFunction(`runU${row.width}`);
      const receipt = await (await submit.send(
        operationIndex.indexOf(row.operation), left[0], left[1], right[0], right[1],
      )).wait();
      const elapsedMs = (process.hrtime.bigint() - started) / 1000000n;
      expect(receipt).not.to.equal(null);
      const ciphertext = await contract.getFunction(
        row.operation === "gt" ? `readBoolU${row.width}` : `readValueU${row.width}`,
      ).staticCall(owner.address);
      const decrypted = await client.decryptForView(ciphertext, resultType(row.width, row.operation)).execute();
      expect(decrypted).to.equal(typeof row.expected === "boolean" ? row.expected : BigInt(row.expected));
      records.push({ id: row.id, width: row.width, operation: row.operation,
        expected: row.expected, measured_mock_gas: receipt!.gasUsed.toString(),
        elapsed_wall_ms_including_mock_receipt: elapsedMs.toString() });
    });
  }

  it("rejects wrong consuming-contract ciphertext", async function () {
    const { owner, stranger, contract, address, client } = await fixture();
    const left = await client.encryptInputs([Encryptable.uint8(7n)]).setConsumingContract(stranger.address).execute();
    const right = await client.encryptInputs([Encryptable.uint8(9n)]).setConsumingContract(address).execute();
    await expect(contract.connect(owner).runU8(0, left[0], left[1], right[0], right[1])).to.be.reverted;
  });

  it("rejects a different submitting signer", async function () {
    const { stranger, contract, address, client } = await fixture();
    const left = await client.encryptInputs([Encryptable.uint16(5n)]).setConsumingContract(address).execute();
    const right = await client.encryptInputs([Encryptable.uint16(2n)]).setConsumingContract(address).execute();
    await expect(contract.connect(stranger).runU16(0, left[0], left[1], right[0], right[1])).to.be.reverted;
  });

  it("rejects ciphertext/proof substitution", async function () {
    const { owner, contract, address, client } = await fixture();
    const left = await client.encryptInputs([Encryptable.uint32(5n)]).setConsumingContract(address).execute();
    const right = await client.encryptInputs([Encryptable.uint32(2n)]).setConsumingContract(address).execute();
    await expect(contract.connect(owner).runU32(0, left[0], right[1], right[0], left[1])).to.be.reverted;
  });

  it("rejects unsupported plaintext operation indexes", async function () {
    const { owner, contract, address, client } = await fixture();
    const left = await client.encryptInputs([Encryptable.uint8(5n)]).setConsumingContract(address).execute();
    const right = await client.encryptInputs([Encryptable.uint8(2n)]).setConsumingContract(address).execute();
    await expect(contract.connect(owner).runU8(6, left[0], left[1], right[0], right[1])).to.be.reverted;
  });

  it("denies decryption to another account", async function () {
    const { owner, stranger, contract, address, client } = await fixture();
    const left = await client.encryptInputs([Encryptable.uint8(5n)]).setConsumingContract(address).execute();
    const right = await client.encryptInputs([Encryptable.uint8(7n)]).setConsumingContract(address).execute();
    await contract.connect(owner).runU8(4, left[0], left[1], right[0], right[1]);
    const ciphertext = await contract.readValueU8(owner.address);
    expect(await client.decryptForView(ciphertext, FheTypes.Uint8).execute()).to.equal(7n);
    const other = await hre.cofhe.createClientWithBatteries(stranger);
    let failed = false;
    try { await other.decryptForView(ciphertext, FheTypes.Uint8).execute(); }
    catch { failed = true; }
    expect(failed).to.equal(true);
  });

  after(function () {
    const path = process.env.FHEC_PHASE6_MOCK_EVIDENCE_FILE;
    if (!path) return;
    const payload = {
      schema: "fhec-cofhe-phase6-mock-coverage-v1",
      environment: "Fhenix Hardhat mock, NOT actual FHE coprocessor",
      cases_per_family: matrix.cases_per_family,
      completed_cases: records.length,
      family_count: 24,
      records,
      limitations: ["Mock gas is not live gas", "Elapsed wall time includes local simulation overhead", "No live encrypted computation is proven"],
    };
    writeFileSync(path, `${JSON.stringify(payload, null, 2)}\n`, { flag: "wx" });
  });
});
