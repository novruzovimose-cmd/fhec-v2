import { loadFixture } from "@nomicfoundation/hardhat-toolbox/network-helpers";
import { Encryptable, FheTypes } from "@cofhe/sdk";
import { expect } from "chai";
import hre from "hardhat";

const TASK_COFHE_MOCKS_DEPLOY = "task:cofhe-mocks:deploy";

describe("FHEC CoFHE baseline versus optimized", function () {
  async function deployFixture() {
    await hre.run(TASK_COFHE_MOCKS_DEPLOY);
    const [owner] = await hre.ethers.getSigners();
    const factory = await hre.ethers.getContractFactory("FHECOptimizationComparison");
    const contract = await factory.connect(owner).deploy();
    await contract.waitForDeployment();
    const address = await contract.getAddress();
    const client = await hre.cofhe.createClientWithBatteries(owner);
    return { owner, contract, address, client };
  }

  for (const [a, b] of [
    [0n, 255n],
    [2n, 3n],
    [7n, 9n],
    [128n, 2n],
    [255n, 255n],
  ] as const) {
    it(`produces equal encrypted results for inputs ${a}, ${b}`, async function () {
      const { owner, contract, address, client } = await loadFixture(deployFixture);
      const expected = (2n * a * b) % 256n;

      const baselineLeft = await client
        .encryptInputs([Encryptable.uint8(a)])
        .setConsumingContract(address)
        .execute();
      const baselineRight = await client
        .encryptInputs([Encryptable.uint8(b)])
        .setConsumingContract(address)
        .execute();

      const baselineTx = await contract.connect(owner).runBaseline(
        baselineLeft[0], baselineLeft[1], baselineRight[0], baselineRight[1],
      );
      const baselineReceipt = await baselineTx.wait();
      const baselineHandle = await contract.baselineValue();
      expect(await client.decryptForView(baselineHandle, FheTypes.Uint8).execute()).to.equal(expected);

      const optimizedLeft = await client
        .encryptInputs([Encryptable.uint8(a)])
        .setConsumingContract(address)
        .execute();
      const optimizedRight = await client
        .encryptInputs([Encryptable.uint8(b)])
        .setConsumingContract(address)
        .execute();

      const optimizedTx = await contract.connect(owner).runOptimized(
        optimizedLeft[0], optimizedLeft[1], optimizedRight[0], optimizedRight[1],
      );
      const optimizedReceipt = await optimizedTx.wait();
      const optimizedHandle = await contract.optimizedValue();
      expect(await client.decryptForView(optimizedHandle, FheTypes.Uint8).execute()).to.equal(expected);

      expect(baselineReceipt).not.to.equal(null);
      expect(optimizedReceipt).not.to.equal(null);
      console.log(
        `MOCK GAS ONLY (${a}, ${b}): baseline=${baselineReceipt!.gasUsed} optimized=${optimizedReceipt!.gasUsed}`,
      );
    });
  }
});
