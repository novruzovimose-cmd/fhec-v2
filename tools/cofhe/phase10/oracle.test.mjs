import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { corpusDigest, createCorpus, defaultSeed, reference, validateCorpus } from "./oracle.mjs";

describe("Independent Phase 10 full-width BigInt oracle", () => {
  it("has stable deterministic boundary cases, widths and selectors", () => {
    const a = createCorpus(8);
    const b = createCorpus(8);
    assert.deepEqual(a, b);
    assert.equal(a.case_count, 192);
    assert.equal(a.families, 24);
    assert.equal(a.cases[0].id, "p10-add-u8-00");
    assert.equal(a.cases[0].expected, "0");
    assert.equal(a.cases[1].expected, "0");
    assert.equal(a.cases[2].expected, "0");
    assert.equal(a.cases.at(-1).width, 64);
    assert.match(corpusDigest(a), /^[A-F0-9]{64}$/);
    assert.deepEqual(validateCorpus(a), a);
  });
  it("enforces unsigned modular boundaries for every supported type", () => {
    for (const width of [8, 16, 32, 64]) {
      const max = (1n << BigInt(width)) - 1n;
      assert.equal(reference(width, "add", max, 1n), "0");
      assert.equal(reference(width, "sub", 0n, 1n), String(max));
      assert.equal(reference(width, "mul", max, max), "1");
      assert.equal(reference(width, "gt", max, 1n), true);
      assert.equal(reference(width, "gt", max, max), false);
      assert.equal(reference(width, "max", 0n, max), String(max));
      assert.equal(reference(width, "repeated", max, 1n), String(max - 1n));
    }
  });
  it("exercises deterministic pseudorandom values and three-way selection", () => {
    const one = createCorpus(16, "three-way");
    const two = createCorpus(16, "three-way", "a".repeat(64));
    assert.equal(one.case_count, 384);
    assert.equal(one.variants.length, 3);
    assert.notEqual(corpusDigest(one), corpusDigest(two));
    assert.deepEqual(one.cases.slice(0, 8).map((v) => [v.a, v.b]), two.cases.slice(0, 8).map((v) => [v.a, v.b]));
    assert.notDeepEqual(one.cases.filter((v) => v.id.endsWith("-10")).map((v) => v.a), two.cases.filter((v) => v.id.endsWith("-10")).map((v) => v.a));
  });
  it("rejects fabricated oracle outcomes, fields and out-of-range inputs", () => {
    const one = createCorpus(3);
    one.cases[1].expected = "255";
    assert.throws(() => validateCorpus(one), /Corrupted/);
    assert.throws(() => createCorpus(17), /2 to 16/);
    assert.throws(() => createCorpus(0), /2 to 16/);
    assert.throws(() => createCorpus(2, "bad"), /variant/);
    assert.throws(() => createCorpus(2, "optimized-tuned", defaultSeed.toUpperCase()), /seed/);
    assert.throws(() => reference(8, "add", 256n, 0n), /outside/);
    assert.throws(() => reference(64, "sub", -1n, 0n), /outside/);
  });
});
