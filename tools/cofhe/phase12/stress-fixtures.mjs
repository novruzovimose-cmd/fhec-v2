import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createCorpus } from "../phase10/oracle.mjs";
import { requireSame, sha256 } from "./io.mjs";
import { operations, widths } from "./constants.mjs";

export const stressSeeds = Object.freeze(Array.from({ length: 12 }, (_, index) =>
  createHash("sha256").update("fhec-cofhe-phase12-stress-seed-v1:" + String(index).padStart(2, "0")).digest("hex")));

export function independentValue(testcase) {
  const bits = BigInt(testcase.width);
  const ring = 2n ** bits;
  const x = BigInt(testcase.a);
  const y = BigInt(testcase.b);
  if (x < 0n || y < 0n || x >= ring || y >= ring) throw new Error("Stress operand exceeds declared encrypted width");
  switch (testcase.operation) {
    case "add": return ((x + y) & (ring - 1n)).toString();
    case "sub": return (((x - y) % ring + ring) % ring).toString();
    case "mul": return ((x * y) & (ring - 1n)).toString();
    case "gt": return x > y;
    case "max": return (x > y ? x : y).toString();
    case "repeated": return (((x * y) << 1n) & (ring - 1n)).toString();
    default: throw new Error("Unknown independent Phase 12 operation");
  }
}

export function verifyStressFixture(corpus, index) {
  const seed = stressSeeds[index];
  const expected = createCorpus(16, "three-way", seed);
  requireSame(corpus, expected, "Stress test fixture diverged from canonical deterministic full-width corpus: " + index);
  if (corpus.cases.length !== 384 || corpus.case_count !== 384 || corpus.families !== 24) {
    throw new Error("Stress fixture is missing one of its 24 operation/width families");
  }
  for (const testcase of corpus.cases) {
    if (!widths.includes(testcase.width) || !operations.includes(testcase.operation) ||
        independentValue(testcase) !== testcase.expected) {
      throw new Error("Independent modular arithmetic oracle refutes stress fixture: " + testcase.id);
    }
  }
  return { cases: corpus.cases.length, sha256: sha256(JSON.stringify(corpus)) };
}

export function verifyStressFixtures(directory) {
  const fixtures = [];
  for (let index = 0; index < stressSeeds.length; index++) {
    const name = `stress-${String(index).padStart(2, "0")}-three-way.json`;
    const corpus = JSON.parse(readFileSync(join(directory, name), "utf8").replace(/^\uFEFF/, ""));
    fixtures.push({ name, ...verifyStressFixture(corpus, index) });
  }
  return {
    schema: "fhec-cofhe-phase12-independent-oracle-stress-v1",
    evidence_tier: "deterministic plaintext reference vectors only; not cryptographic or live testnet executions",
    fixtures: fixtures.length,
    independently_checked_reference_vectors: fixtures.reduce((sum, item) => sum + item.cases, 0),
    families_per_fixture: 24,
    records: fixtures,
  };
}

export function createFixtures(directory) {
  for (let index = 0; index < stressSeeds.length; index++) {
    const file = join(directory, `stress-${String(index).padStart(2, "0")}-three-way.json`);
    writeFileSync(file, JSON.stringify(createCorpus(16, "three-way", stressSeeds[index]), null, 2) + "\n", { flag: "wx" });
  }
  return verifyStressFixtures(directory);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv[2] === "--generate" && process.argv.length === 4) {
      process.stdout.write(JSON.stringify(createFixtures(resolve(process.argv[3])), null, 2) + "\n");
    } else if (process.argv[2] === "--verify" && process.argv.length === 4) {
      process.stdout.write(JSON.stringify(verifyStressFixtures(resolve(process.argv[3])), null, 2) + "\n");
    } else throw new Error("Usage: --generate <new empty fixtures directory> | --verify <fixtures directory>");
  } catch (error) { process.stderr.write("Phase 12 stress fixture validation failed: " + error.message + "\n"); process.exitCode = 1; }
}
