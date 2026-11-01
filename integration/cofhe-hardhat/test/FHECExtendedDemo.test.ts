import { loadFixture } from "@nomicfoundation/hardhat-toolbox/network-helpers";
import { Encryptable, FheTypes } from "@cofhe/sdk";
import { expect } from "chai";
import hre from "hardhat";

const TASK_COFHE_MOCKS_DEPLOY = "task:cofhe-mocks:deploy";

describe("FHEC V2 extended CoFHE execution", function () {
  async function deployFixture() {
    await hre.run(TASK_COFHE_MOCKS_DEPLOY);
    const [owner, stranger] = await hre.ethers.getSigners();
    const factory = await hre.ethers.getContractFactory("FHECExtendedDemo");
    const contract = await factory.connect(owner).deploy();
    await contract.waitForDeployment();
    const address = await contract.getAddress();
    const client = await hre.cofhe.createClientWithBatteries(owner);
    return { owner, stranger, contract, address, client };
  }

  async function encryptOne(
    client: Awaited<ReturnType<typeof deployFixture>>["client"],
    address: string,
    item:
      | ReturnType<typeof Encryptable.uint8>
      | ReturnType<typeof Encryptable.uint16>
      | ReturnType<typeof Encryptable.uint32>
      | ReturnType<typeof Encryptable.uint64>,
  ) {
    const [hash, proof] = await client
      .encryptInputs([item])
      .setConsumingContract(address)
      .execute();
    return { hash, proof };
  }

  for (const [a, b, expected] of [
    [200n, 100n, 300n],
    [0n, 65535n, 65535n],
    [65535n, 1n, 0n],
  ] as const) {
    it(`executes generated u16 addition: ${a} + ${b}`, async function () {
      const { owner, contract, address, client } = await loadFixture(deployFixture);
      const left = await encryptOne(client, address, Encryptable.uint16(a));
      const right = await encryptOne(client, address, Encryptable.uint16(b));
      await contract.connect(owner).calculateSum16(left.hash, left.proof, right.hash, right.proof);
      const ciphertext = await contract.sum16();
      expect(await client.decryptForView(ciphertext, FheTypes.Uint16).execute()).to.equal(expected);
    });
  }

  for (const [a, b, expected] of [
    [7n, 9n, 63n],
    [0n, 4294967295n, 0n],
    [65536n, 65536n, 0n],
  ] as const) {
    it(`executes generated u32 multiplication: ${a} * ${b}`, async function () {
      const { owner, contract, address, client } = await loadFixture(deployFixture);
      const left = await encryptOne(client, address, Encryptable.uint32(a));
      const right = await encryptOne(client, address, Encryptable.uint32(b));
      await contract.connect(owner).calculateProduct32(left.hash, left.proof, right.hash, right.proof);
      const ciphertext = await contract.product32();
      expect(await client.decryptForView(ciphertext, FheTypes.Uint32).execute()).to.equal(expected);
    });
  }

  for (const [a, b, expected] of [
    [9n, 2n, true],
    [2n, 9n, false],
    [18446744073709551615n, 0n, true],
    [18446744073709551615n, 18446744073709551615n, false],
  ] as const) {
    it(`executes generated u64 comparison: ${a} > ${b}`, async function () {
      const { owner, contract, address, client } = await loadFixture(deployFixture);
      const left = await encryptOne(client, address, Encryptable.uint64(a));
      const right = await encryptOne(client, address, Encryptable.uint64(b));
      await contract.connect(owner).calculateGreater64(left.hash, left.proof, right.hash, right.proof);
      const ciphertext = await contract.comparison64();
      expect(await client.decryptForView(ciphertext, FheTypes.Bool).execute()).to.equal(expected);
    });
  }

  for (const [value, expected] of [
    [0n, 0n],
    [5n, 20n],
    [63n, 252n],
    [64n, 0n],
    [255n, 252n],
  ] as const) {
    it(`executes chained generated functions: four times ${value}`, async function () {
      const { owner, contract, address, client } = await loadFixture(deployFixture);
      const input = await encryptOne(client, address, Encryptable.uint8(value));
      await contract.connect(owner).calculateFourfold8(input.hash, input.proof);
      const ciphertext = await contract.fourfold8();
      expect(await client.decryptForView(ciphertext, FheTypes.Uint8).execute()).to.equal(expected);
    });
  }

  it("rejects ciphertext proof submitted by a different account", async function () {
    const { stranger, contract, address, client } = await loadFixture(deployFixture);
    const left = await encryptOne(client, address, Encryptable.uint16(1n));
    const right = await encryptOne(client, address, Encryptable.uint16(2n));
    await expect(
      contract.connect(stranger).calculateSum16(left.hash, left.proof, right.hash, right.proof),
    ).to.be.reverted;
  });

  it("rejects a proof signed for another consuming contract", async function () {
    const { owner, stranger, contract, address, client } = await loadFixture(deployFixture);
    const left = await encryptOne(client, stranger.address, Encryptable.uint16(1n));
    const right = await encryptOne(client, address, Encryptable.uint16(2n));
    await expect(
      contract.connect(owner).calculateSum16(left.hash, left.proof, right.hash, right.proof),
    ).to.be.reverted;
  });

  it("rejects a proof paired with the wrong ciphertext", async function () {
    const { owner, contract, address, client } = await loadFixture(deployFixture);
    const left = await encryptOne(client, address, Encryptable.uint16(4n));
    const right = await encryptOne(client, address, Encryptable.uint16(5n));
    await expect(
      contract.connect(owner).calculateSum16(left.hash, right.proof, right.hash, left.proof),
    ).to.be.reverted;
  });

  it("permits the submitting account to decrypt the output but denies another account", async function () {
    const { owner, stranger, contract, address, client } = await loadFixture(deployFixture);
    const left = await encryptOne(client, address, Encryptable.uint16(5n));
    const right = await encryptOne(client, address, Encryptable.uint16(6n));
    await contract.connect(owner).calculateSum16(left.hash, left.proof, right.hash, right.proof);
    const ciphertext = await contract.sum16();
    expect(await client.decryptForView(ciphertext, FheTypes.Uint16).execute()).to.equal(11n);
    const otherClient = await hre.cofhe.createClientWithBatteries(stranger);
    let denied = false;
    try {
      await otherClient.decryptForView(ciphertext, FheTypes.Uint16).execute();
    } catch {
      denied = true;
    }
    expect(denied).to.equal(true);
  });
});
