import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { corpusDigest, createCorpus, defaultSeed, validateCorpus } from "./oracle.mjs";

const directory = join(dirname(fileURLToPath(import.meta.url)), "fixtures");
const counts = Object.freeze([2, 3, 8]);
const variants = Object.freeze(["optimized-tuned", "three-way"]);

export function checkFixtures(base = directory) {
  const records = [];
  for (const count of counts) {
    for (const mode of variants) {
      const filename = `golden-${count}-${mode}.json`;
      const actual = JSON.parse(readFileSync(join(base, filename), "utf8").replace(/^\uFEFF/, ""));
      validateCorpus(actual);
      const expected = createCorpus(count, mode, defaultSeed);
      if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error("Independent reference fixture differs: " + filename);
      records.push({ file: filename, cases: expected.case_count, contract_variants: expected.variants.length, corpus_sha256: corpusDigest(actual) });
    }
  }
  return records;
}

export function main() {
  const result = checkFixtures();
  process.stdout.write(JSON.stringify({ schema: "fhec-phase10-independent-golden-corpus-v1", checked: result.length, fixtures: result }, null, 2) + "\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(); }
  catch (error) { process.stderr.write("Phase 10 canonical fixture validation failed: " + error.message + "\n"); process.exitCode = 1; }
}
