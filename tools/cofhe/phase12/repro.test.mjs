import { test } from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { compareIndependentRuns } from "./repro.mjs";

const fixture = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "mock-dossier");

test("identical independently verifiable dossiers have identical deterministic mock results", () => {
  const directory = mkdtempSync(join(tmpdir(), "fhec-phase12-repeat-"));
  try {
    const second = join(directory, "second");
    cpSync(fixture, second, { recursive: true });
    const result = compareIndependentRuns(fixture, second);
    assert.equal(result.identical_oracle_inputs, true);
    assert.equal(result.identical_mock_decrypted_outputs, true);
    assert.equal(result.compared_mock_executions, 432);
    assert.throws(() => compareIndependentRuns(fixture, fixture), /Two independent/);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("independent run comparison rejects a forged second result", () => {
  const directory = mkdtempSync(join(tmpdir(), "fhec-phase12-repeat-"));
  try {
    const second = join(directory, "second");
    cpSync(fixture, second, { recursive: true });
    const raw = join(second, "mock-observations.json");
    writeFileSync(raw, readFileSync(raw, "utf8").replace('"actual": "0"', '"actual": "1"'));
    assert.throws(() => compareIndependentRuns(fixture, second));
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
