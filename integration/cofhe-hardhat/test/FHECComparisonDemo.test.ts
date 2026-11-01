import { loadFixture } from "@nomicfoundation/hardhat-toolbox/network-helpers";
import { Encryptable, FheTypes } from "@cofhe/sdk";
import { expect } from "chai";
import hre from "hardhat";

const TASK_COFHE_MOCKS_DEPLOY = "task:cofhe-mocks:deploy";

describe("FHEC-generated CoFHE comparison", function () {
  async function deployFixture() {
    await hre.run(TASK_COFHE_MOCKS_DEPLOY);
    const [owner, stranger] = await hre.ethers.getSigners();
    const factory = await hre.ethers.getContractFactory("FHECComparisonDemo");
    const contract = await factory.connect(owner).deploy();
    await contract.waitForDeployment();
    const address = await contract.getAddress();
    const client = await hre.cofhe.createClientWithBatteries(owner);
    return { owner, stranger, contract, address, client };
  }

  for (const [a, b, expected] of [
    [8n, 3n, 8n],
    [3n, 8n, 8n],
    [7n, 7n, 7n],
    [0n, 255n, 255n],
    [255n, 0n, 255n],
  ] as const) {
    it(`returns max(${a}, ${b}) = ${expected} with mock encrypted computation`, async function () {
      const { owner, contract, address, client } = await loadFixture(deployFixture);
      const [left, leftProof] = await client
        .encryptInputs([Encryptable.uint8(a)])
        .setConsumingContract(address)
        .execute();
      const [right, rightProof] = await client
        .encryptInputs([Encryptable.uint8(b)])
        .setConsumingContract(address)
        .execute();

      await contract.connect(owner).compare(left, leftProof, right, rightProof);
      const ciphertext = await contract.result();
      await hre.cofhe.mocks.expectPlaintext(ciphertext, expected);
      expect(await client.decryptForView(ciphertext, FheTypes.Uint8).execute()).to.equal(expected);
    });
  }
});
