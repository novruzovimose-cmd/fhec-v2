import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { families, generateArtifacts, generateHarness, generateLibrary, variants } from "./generate-manual.mjs";
import { widths } from "../phase6/oracle.mjs";

describe("Phase 7 independently authored Solidity comparators", () => {
  it("has all four encrypted widths and six semantics", () => {
    assert.deepEqual(widths, [8, 16, 32, 64]);
    assert.deepEqual(families, ["add", "sub", "mul", "gt", "max", "repeated"]);
  });
  it("has three distinct harnesses on precisely the same dispatch structure", () => {
    assert.equal(variants.length, 3);
    for (const variant of variants) {
      const source = generateHarness(variant.library, variant.harness);
      for (const width of widths) {
        assert.match(source, new RegExp("function runU" + width + "\\("));
        assert.match(source, new RegExp("mapping\\(address => euint" + width + "\\)"));
      }
      assert.equal((source.match(/FHE\.allowSender\(/g) || []).length, 8);
      assert.equal((source.match(/FHE\.allowThis\(/g) || []).length, 8);
      assert.doesNotMatch(source, /tx\.origin|delegatecall|allowPublic|selfdestruct/);
    }
  });
  it("does not make the tuned comparison unfair by editing dispatch or access control", () => {
    const original = generateHarness(variants[0].library, variants[0].harness);
    for (const variant of variants.slice(1)) {
      const other = generateHarness(variant.library, variant.harness);
      assert.equal(other
        .replaceAll(variant.library, variants[0].library)
        .replaceAll(variant.harness, variants[0].harness), original);
    }
  });
  it("tuned and straight libraries differ only in repeated common-subexpression construction", () => {
    const straight = generateLibrary("FHECPhase7ManualStraight", false);
    const tuned = generateLibrary("FHECPhase7ManualTuned", true);
    for (const width of widths) {
      for (const family of families) {
        assert.match(straight, new RegExp("function " + family + "_u" + width + "\\("));
        assert.match(tuned, new RegExp("function " + family + "_u" + width + "\\("));
      }
    }
    assert.equal((straight.match(/FHE\.mul\(a, b\)/g) || []).length, 12);
    assert.equal((tuned.match(/FHE\.mul\(a, b\)/g) || []).length, 8);
  });
  it("generates exactly five new source files", () => {
    const files = generateArtifacts();
    assert.equal(files.size, 5);
    for (const [name, source] of files) {
      assert.ok(name.endsWith(".sol"));
      assert.ok(source.startsWith("// SPDX-License-Identifier"));
    }
  });
  it("rejects an unknown library/harness pairing", () => {
    assert.throws(() => generateHarness("Other", "FHECPhase7Straight"), /Unrecognized/);
    assert.throws(() => generateLibrary("Other", true), /Unrecognized/);
  });
});
