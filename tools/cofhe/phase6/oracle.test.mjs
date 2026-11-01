import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { readFileSync } from "node:fs";
import { assertMatrix, boundaryPairs, createMatrix, deterministicPairs, expected, maxUnsigned, operations, widths } from "./oracle.mjs";
import { generateSolidity } from "./generate-solidity.mjs";

describe("Phase 6 mathematical and generation safety", () => {
  it("defines all six operation families across four widths", () => {
    assert.deepEqual([...widths], [8, 16, 32, 64]);
    assert.deepEqual([...operations], ["add", "sub", "mul", "gt", "max", "repeated"]);
  });
  it("wraps addition exactly at every declared unsigned width", () => {
    for (const width of widths) assert.equal(expected(width, "add", maxUnsigned(width), 1n), 0n);
  });
  it("wraps subtraction exactly at every declared unsigned width", () => {
    for (const width of widths) assert.equal(expected(width, "sub", 0n, 1n), maxUnsigned(width));
  });
  it("wraps multiplication exactly at every declared unsigned width", () => {
    for (const width of widths) assert.equal(expected(width, "mul", maxUnsigned(width), maxUnsigned(width)), 1n);
  });
  it("bounds repeated subexpressions in modular arithmetic", () => {
    for (const width of widths) assert.equal(expected(width, "repeated", maxUnsigned(width), maxUnsigned(width)), 2n);
  });
  it("resolves equality and greater-than without unsigned coercion errors", () => {
    for (const width of widths) {
      const max = maxUnsigned(width);
      assert.equal(expected(width, "gt", max, max), false);
      assert.equal(expected(width, "gt", max, 0n), true);
      assert.equal(expected(width, "gt", 0n, max), false);
      assert.equal(expected(width, "max", max, 0n), max);
    }
  });
  it("rejects unsupported operations and signed or overflowing literals", () => {
    assert.throws(() => expected(8, "div", 2n, 1n), /Unknown operation/);
    assert.throws(() => expected(8, "add", -1n, 2n), /outside/);
    assert.throws(() => expected(8, "add", 256n, 2n), /outside/);
    assert.throws(() => expected(128, "add", 1n, 1n), /Unsupported/);
  });
  it("seeds an exact boundary corpus with no duplicated pairs", () => {
    for (const width of widths) {
      const list = boundaryPairs(width);
      assert.equal(list.length, 16);
      assert.equal(new Set(list.map(([a, b]) => `${a}/${b}`)).size, 16);
    }
  });
  it("generates deterministic pseudorandom coverage without changing the seed", () => {
    assert.deepEqual(deterministicPairs(64, 15), deterministicPairs(64, 15));
    assert.notDeepEqual(deterministicPairs(64, 15), deterministicPairs(32, 15));
  });
  it("generates an exact 384-case boundary matrix with independent oracle outputs", () => {
    const result = assertMatrix(createMatrix());
    assert.equal(result.total_cases, 384);
    assert.equal(result.cases.filter((entry) => entry.boundary).length, 384);
    assert.equal(result.family_count, 24);
  });
  it("reproduces the packaged 384-case immutable boundary-golden matrix", () => {
    const fixture = JSON.parse(readFileSync(new URL("./golden-boundary-v1.json", import.meta.url), "utf8"));
    assert.deepEqual(assertMatrix(fixture), createMatrix(16));
  });
  it("reproduces the packaged 672-case immutable boundary and seeded-golden matrix", () => {
    const fixture = JSON.parse(readFileSync(new URL("./golden-full-v1.json", import.meta.url), "utf8"));
    assert.equal(fixture.total_cases, 672);
    assert.equal(fixture.cases.filter((entry) => !entry.boundary).length, 288);
    assert.deepEqual(assertMatrix(fixture), createMatrix(28));
  });
  it("detects tampered expected outputs even when the case count is unchanged", () => {
    const matrix = createMatrix();
    matrix.cases[0].expected = "999";
    assert.throws(() => assertMatrix(matrix), /independent oracle/);
  });
  it("rejects incomplete coverage and manipulated identifiers", () => {
    const matrix = createMatrix();
    matrix.cases.pop();
    assert.throws(() => assertMatrix(matrix), /Incomplete/);
    const changed = createMatrix();
    changed.cases[0].id = "forged";
    assert.throws(() => assertMatrix(changed), /independent oracle/);
  });
  it("generates four independent account-bound ciphertext storage partitions", () => {
    const source = generateSolidity();
    for (const width of widths) {
      assert.match(source, new RegExp(`mapping\\(address => euint${width}\\) private values${width};`));
      assert.match(source, new RegExp(`function runU${width}\\(`));
      assert.match(source, new RegExp(`FHECPhase6Optimized\\.max_u${width}\\(`));
      assert.match(source, new RegExp(`FHE\\.asEuint${width}\\(`));
    }
    assert.equal((source.match(/FHE\.allowSender\(/g) ?? []).length, 8);
    assert.equal((source.match(/FHE\.allowThis\(/g) ?? []).length, 8);
    assert.doesNotMatch(source, /delegatecall|tx\.origin|FHE\.allowPublic|FHE\.decrypt|selfdestruct/);
  });
});
