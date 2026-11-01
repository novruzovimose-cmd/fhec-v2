import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createMatrix } from "../phase6/oracle.mjs";
import { analyzeFixture, fixtures } from "./verify-fixtures.mjs";

describe("Phase 7 stable high-coverage reference corpora", () => {
  for (const profile of fixtures) {
    it("regenerates and independently checks " + profile.filename, () => {
      const report = analyzeFixture(createMatrix(profile.perFamily), profile);
      assert.equal(report.cases, profile.total);
      assert.equal(report.families, 24);
      assert.equal(report.boundary_cases, Math.min(profile.perFamily, 16) * 24);
      assert.equal(report.generated_cases, Math.max(profile.perFamily - 16, 0) * 24);
      assert.match(report.canonical_sha256, /^[0-9A-F]{64}$/);
    });
  }
  it("rejects tampered corpus values", () => {
    const profile = fixtures[0];
    const changed = createMatrix(profile.perFamily);
    changed.cases[0].a = "1";
    assert.throws(() => analyzeFixture(changed, profile), /does not match/);
  });
  it("rejects a corpus pretending to have more coverage", () => {
    const changed = createMatrix(16);
    assert.throws(() => analyzeFixture(changed, fixtures[1]), /Golden fixture/);
  });
  it("rejects tampered comparison outcomes", () => {
    const changed = createMatrix(28);
    const entry = changed.cases.find((item) => item.operation === "gt");
    entry.expected = !entry.expected;
    assert.throws(() => analyzeFixture(changed, fixtures[1]), /does not match/);
  });
});
