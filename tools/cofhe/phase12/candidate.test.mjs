import { test } from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { createCandidate } from "./candidate.mjs";
import { verifyCandidate } from "./verify.mjs";
import { mockProject } from "./inventory.test.mjs";

const fixture = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "mock-dossier");

function withCandidate(action) {
  const project = mkdtempSync(join(tmpdir(), "fhec-phase12-rc-"));
  try {
    mockProject(project);
    const output = join(project, "target", "new-local-candidate");
    const created = createCandidate({ project, dossier: fixture, output });
    return action({ project, output, created });
  } finally { rmSync(project, { recursive: true, force: true }); }
}

test("creates a self-contained source-digested local release candidate with atomic rename", () => {
  withCandidate(({ project, output, created }) => {
    assert.equal(created.mock_executions, 216);
    const result = verifyCandidate(output, project);
    assert.equal(result.files, 18);
    assert.equal(result.status, "qualified-local-release-candidate-only");
    assert.throws(() => createCandidate({ project, dossier: fixture, output }), /overwrite/);
  });
});

test("verified release candidate refuses any extra files", () => {
  withCandidate(({ output }) => {
    writeFileSync(join(output, "unapproved-metadata.json"), "{}\n");
    assert.throws(() => verifyCandidate(output), /unexpected/);
  });
});

test("verified candidate refuses cosmetic HTML tampering", () => {
  withCandidate(({ output }) => {
    writeFileSync(join(output, "qualification-report.html"), "<html>fake production audit</html>");
    assert.throws(() => verifyCandidate(output), /digest/);
  });
});

test("candidate verifier rejects a rewritten grant-award claim even if a file is otherwise valid JSON", () => {
  withCandidate(({ output }) => {
    const path = join(output, "qualification.json");
    const report = JSON.parse(readFileSync(path, "utf8"));
    report.grant_awarded = true;
    writeFileSync(path, JSON.stringify(report, null, 2) + "\n");
    assert.throws(() => verifyCandidate(output), /digest/);
  });
});

test("candidate remains independently checkable after copying away from project workspace", () => {
  withCandidate(({ output }) => {
    const destination = mkdtempSync(join(tmpdir(), "fhec-phase12-offline-"));
    try {
      cpSync(output, join(destination, "copied-candidate"), { recursive: true });
      const checked = verifyCandidate(join(destination, "copied-candidate"));
      assert.equal(checked.corpus_cases, 72);
    } finally { rmSync(destination, { recursive: true, force: true }); }
  });
});
