import { Encryptable, FheTypes } from "@cofhe/sdk";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { expect } from "chai";
import hre from "hardhat";

type Value = { id: string; width: number; operation: string; operation_selector: number; a: string; b: string; expected: string | boolean };
type Corpus = { schema: string; variant_mode: string; variants: string[]; case_count: number; cases: Value[] };

const sourceRoot = process.env.FHEC_PHASE89_SOURCES_DIR;
const corpusPath = process.env.FHEC_PHASE10_CORPUS_FILE;
const rawPath = process.env.FHEC_PHASE10_RAW_FILE;
const sealPath = process.env.FHEC_PHASE89_SEAL_FILE;
if (!sourceRoot || !corpusPath || !rawPath || !sealPath) {
  throw new Error("Phase 10 requires a prepared source, canonical independent corpus, fresh result path and source seal");
}

const sourcePrefix = relative(resolve(__dirname, "../.."), resolve(sourceRoot)).split(sep).join("/");
const parsed = JSON.parse(readFileSync(corpusPath, "utf8").replace(/^\uFEFF/, "")) as Corpus;
const sealBytes = readFileSync(sealPath);
const corpusBytes = readFileSync(corpusPath);
const expectedModes = parsed.variant_mode === "three-way" ? ["optimized", "tuned", "straight"] : ["optimized", "tuned"];
if (!sourcePrefix || sourcePrefix.startsWith("../") || parsed.schema !== "fhec-cofhe-phase10-independent-oracle-v1" ||
    JSON.stringify(expectedModes) !== JSON.stringify(parsed.variants) || parsed.case_count !== parsed.cases.length) {
  throw new Error("Phase 10 mock test received an invalid or out-of-tree oracle corpus");
}

function hash(bytes: Buffer | string) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

function encrypted(width: number, value: string) {
  const n = BigInt(value);
  if (width === 8) return Encryptable.uint8(n);
  if (width === 16) return Encryptable.uint16(n);
  if (width === 32) return Encryptable.uint32(n);
  if (width === 64) return Encryptable.uint64(n);
  throw new Error("Phase 10 oracle specifies an unsupported unsigned width");
}

function fheType(width: number, operation: string) {
  if (operation === "gt") return FheTypes.Bool;
  if (width === 8) return FheTypes.Uint8;
  if (width === 16) return FheTypes.Uint16;
  if (width === 32) return FheTypes.Uint32;
  if (width === 64) return FheTypes.Uint64;
  throw new Error("Phase 10 result width must be unsigned and supported");
}

const names: Record<string, string> = {
  optimized: "FHECPhase6Live",
  tuned: "FHECPhase7Tuned",
  straight: "FHECPhase7Straight",
};

describe("FHEC Phase 10 independent property corpus against source-sealed CoFHE mocks", function () {
  this.timeout(600000);
  const contracts = new Map<string, any>();
  const observed: any[] = [];
  let owner: any;
  let client: any;

  before(async function () {
    await hre.run("task:cofhe-mocks:deploy");
    [owner] = await hre.ethers.getSigners();
    client = await hre.cofhe.createClientWithBatteries(owner);
    for (const name of parsed.variants) {
      const solidityName = names[name];
      if (!solidityName) throw new Error("Unknown source-sealed Solidity variant");
      const qualified = `${sourcePrefix}/${solidityName}.sol:${solidityName}`;
      const factory = await hre.ethers.getContractFactory(qualified);
      const contract = await factory.connect(owner).deploy();
      await contract.waitForDeployment();
      contracts.set(name, contract);
    }
  });

  for (const item of parsed.cases) {
    it("preserves independent boundary/fuzz oracle: " + item.id, async function () {
      const measurements: Record<string, unknown> = {};
      for (const name of parsed.variants) {
        const contract = contracts.get(name)!;
        const address = await contract.getAddress();
        const startedAt = Date.now();
        const left = await client.encryptInputs([encrypted(item.width, item.a)]).setConsumingContract(address).execute();
        const right = await client.encryptInputs([encrypted(item.width, item.b)]).setConsumingContract(address).execute();
        const encryptedMs = Date.now() - startedAt;
        const call = contract.connect(owner).getFunction("runU" + item.width);
        const transaction = await call.send(item.operation_selector, left[0], left[1], right[0], right[1]);
        const receipt = await transaction.wait();
        expect(receipt).to.not.equal(null);
        expect(receipt!.status).to.equal(1);
        const handle = await contract.getFunction((item.operation === "gt" ? "readBoolU" : "readValueU") + item.width).staticCall(owner.address);
        const actual = await client.decryptForView(handle, fheType(item.width, item.operation)).execute();
        expect(String(actual)).to.equal(String(item.expected));
        measurements[name] = {
          actual: String(actual),
          oracle_match: true,
          mock_transaction_gas: receipt!.gasUsed.toString(),
          sdk_encrypt_ms: encryptedMs,
          mock_execution_and_decryption_ms: Date.now() - startedAt,
        };
      }
      observed.push({ id: item.id, width: item.width, operation: item.operation, expected: String(item.expected), variants: measurements });
    });
  }

  it("rejects an encrypted input produced for another consuming contract", async function () {
    const first = contracts.get(parsed.variants[0])!;
    const second = contracts.get(parsed.variants[1])!;
    const wrong = await client.encryptInputs([Encryptable.uint8(7n)]).setConsumingContract(await second.getAddress()).execute();
    const right = await client.encryptInputs([Encryptable.uint8(9n)]).setConsumingContract(await first.getAddress()).execute();
    await expect(first.connect(owner).getFunction("runU8").send(0, wrong[0], wrong[1], right[0], right[1])).to.be.reverted;
  });

  after(function () {
    const output = {
      schema: "fhec-cofhe-phase10-mock-observations-v1",
      evidence_tier: "locally-executed-hardhat-cofhe-mocks-not-live-network",
      corpus_sha256: hash(JSON.stringify(parsed)),
      source_seal_file_sha256: hash(sealBytes),
      corpus_file_sha256: hash(corpusBytes),
      cases_expected: parsed.case_count,
      cases_recorded: observed.length,
      variants: parsed.variants,
      full_corpus_completed: observed.length === parsed.case_count,
      records: observed,
      limitations: ["Mock gas is not production or testnet gas", "SDK mock decryption is not independent cryptographic verification", "Boundary and pseudorandom sampling is not exhaustive formal verification"],
    };
    writeFileSync(rawPath!, JSON.stringify(output, null, 2) + "\n", { flag: "wx" });
  });
});
