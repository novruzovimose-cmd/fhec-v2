import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { it } from "node:test";
import { createCorpus, defaultSeed } from "./oracle.mjs";
import { checkFixtures } from "./verify-fixtures.mjs";

it("reproduces six reviewed canonical boundary and arithmetic sample corpora", () => {
  const fixtures = checkFixtures();
  assert.equal(fixtures.length, 6);
  assert.deepEqual(fixtures.map((v) => v.cases), [48, 48, 72, 72, 192, 192]);
  assert.deepEqual(fixtures.map((v) => v.contract_variants), [2, 3, 2, 3, 2, 3]);
  assert.equal(new Set(fixtures.map((v) => v.corpus_sha256)).size, 6);
});

it("rejects edited golden samples before any Hardhat deployment", () => {
  const path = mkdtempSync(join(tmpdir(), "fhec-phase10-corpus-snapshot-"));
  for (const count of [2, 3, 8]) {
    for (const mode of ["optimized-tuned", "three-way"]) {
      const corpus = createCorpus(count, mode, defaultSeed);
      if (count === 3 && mode === "three-way") corpus.cases[0].b = "1";
      writeFileSync(join(path, `golden-${count}-${mode}.json`), JSON.stringify(corpus) + "\n");
    }
  }
  assert.throws(() => checkFixtures(path), /Corrupted|differs/);
});
