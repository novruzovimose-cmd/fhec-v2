import { test } from "node:test";
import assert from "node:assert/strict";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { readJson } from "./io.mjs";
import { independentValue, verifyStressFixture, verifyStressFixtures } from "./stress-fixtures.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "stress");

test("4608 independent 8/16/32/64-bit reference vectors across 12 deterministic seeds", () => {
  const evidence = verifyStressFixtures(root);
  assert.equal(evidence.fixtures, 12);
  assert.equal(evidence.independently_checked_reference_vectors, 4608);
  assert.equal(evidence.families_per_fixture, 24);
  assert.match(evidence.evidence_tier, /not cryptographic/);
});

test("stress corpus refuses forged modular expected result", () => {
  const corpus = readJson(join(root, "stress-00-three-way.json"));
  const bad = structuredClone(corpus);
  bad.cases[0].expected = "9";
  assert.throws(() => verifyStressFixture(bad, 0), /diverged/);
});

test("independent arithmetic correctly models modular wraparound and unsigned comparison", () => {
  assert.equal(independentValue({ width: 8, operation: "add", a: "255", b: "1" }), "0");
  assert.equal(independentValue({ width: 8, operation: "sub", a: "0", b: "1" }), "255");
  assert.equal(independentValue({ width: 64, operation: "mul", a: "18446744073709551615", b: "2" }), "18446744073709551614");
  assert.equal(independentValue({ width: 16, operation: "gt", a: "0", b: "65535" }), false);
  assert.equal(independentValue({ width: 16, operation: "max", a: "0", b: "65535" }), "65535");
  assert.equal(independentValue({ width: 8, operation: "repeated", a: "255", b: "1" }), "254");
});
