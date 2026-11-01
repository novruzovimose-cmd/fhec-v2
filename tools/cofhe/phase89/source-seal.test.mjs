import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { generateSeal, sourceNames, validateSeal } from "./source-seal.mjs";

describe("Phase 8/9 immutable source preflight seal", () => {
  it("enumerates the entire six-source deployable comparison bundle", () => {
    assert.equal(sourceNames.length, 7);
    assert.equal(new Set(sourceNames).size, 7);
    assert.equal(sourceNames.filter((item) => item.endsWith(".sol")).length, 6);
    assert.equal(sourceNames.at(-1), "phase89-plan.json");
  });
  it("does not accept forged seals or nonexistent generation directory", () => {
    assert.throws(() => generateSeal("/this-phase89-path-must-not-exist"));
    assert.throws(() => validateSeal("/this-phase89-path-must-not-exist", { schema: "fake" }), /modified/);
  });
});
