import { test } from "node:test";
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import { readJson } from "./io.mjs";
import { evaluateCoverage } from "./coverage.mjs";
import { qualifyDossier } from "./qualification.mjs";
import { fixtureSourceSealSha256 } from "./constants.mjs";
import { sha256 } from "./io.mjs";
import { checkMockResults } from "../phase10/validate-results.mjs";
import { render } from "../phase11/report-html.mjs";
import { evaluateReadiness } from "../phase11/readiness.mjs";
import { generateEvidenceManifest } from "../phase11/attest.mjs";
import { rehearseTampering } from "./adversarial.mjs";

const fixture = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "mock-dossier");
const corpus = readJson(join(fixture, "corpus.json"));
const validated = readJson(join(fixture, "validated-mock-results.json"));

test("Phase 12 accepts the reviewed full-width, 72-vector, 216-execution local mock dossier", () => {
  const value = qualifyDossier(fixture);
  assert.equal(value.status, "qualified-local-release-candidate-only");
  assert.equal(value.source_seal_sha256, fixtureSourceSealSha256);
  assert.equal(value.full_width_coverage.operation_families, 24);
  assert.equal(value.full_width_coverage.verified_mock_executions, 216);
  assert.equal(value.production_certified, false);
  assert.equal(value.grant_awarded, false);
  assert.equal(value.pending_external_gates.length, 4);
});

test("fails closed on a 24-family report with a missing variant", () => {
  const report = structuredClone(validated);
  delete report.families[0].variants.straight;
  assert.throws(() => evaluateCoverage(corpus, report), /Missing or undersampled/);
});

test("fails closed on duplicate corpus identifiers", () => {
  const changed = structuredClone(corpus);
  changed.cases[1].id = changed.cases[0].id;
  assert.throws(() => evaluateCoverage(changed, validated), /duplicate/);
});

test("fails closed when any family loses its third boundary case", () => {
  const changed = structuredClone(corpus);
  changed.cases = changed.cases.filter((item) => item.id !== "p10-add-u8-02");
  changed.case_count -= 1;
  const summary = structuredClone(validated);
  summary.cases -= 1;
  summary.successful_mock_executions -= 3;
  assert.throws(() => evaluateCoverage(changed, summary), /Insufficient|Missing or undersampled/);
});

test("fails closed without the contract-domain negative proof", () => {
  const report = structuredClone(validated);
  report.negative_consuming_contract_test_required = false;
  assert.throws(() => evaluateCoverage(corpus, report), /domain-negative/);
});

test("fails closed on tuned-only claim replacing the full three-way record", () => {
  const changed = structuredClone(corpus);
  changed.variant_mode = "optimized-tuned";
  changed.variants = ["optimized", "tuned"];
  assert.throws(() => evaluateCoverage(changed, validated), /three-way/);
});

test("a different source seal cannot silently inherit a reviewed Phase 12 release qualification", () => {
  assert.throws(() => qualifyDossier(fixture, "A".repeat(64)), /source seal/);
});

test("a fully valid independently replayed dossier with a different source-seal file hash is accepted without a foreign fixture pin", () => {
  const root = mkdtempSync(join(tmpdir(), "fhec-phase12-other-reviewed-dossier-"));
  const altered = join(root, "alternate");
  try {
    cpSync(fixture, altered, { recursive: true });
    const sealFile = join(altered, "sources", "phase89-source-seal.json");
    writeFileSync(sealFile, readFileSync(sealFile, "utf8") + "\n");
    const newSealDigest = sha256(readFileSync(sealFile));
    assert.notEqual(newSealDigest, fixtureSourceSealSha256);
    const rawFile = join(altered, "mock-observations.json");
    const raw = readJson(rawFile);
    raw.source_seal_file_sha256 = newSealDigest;
    writeFileSync(rawFile, JSON.stringify(raw, null, 2) + "\n");
    const newValidated = checkMockResults(readJson(join(altered, "corpus.json")), raw,
      readFileSync(sealFile), readFileSync(join(altered, "corpus.json")));
    writeFileSync(join(altered, "validated-mock-results.json"), JSON.stringify(newValidated, null, 2) + "\n");
    writeFileSync(join(altered, "offline-report.html"), render(newValidated));
    writeFileSync(join(altered, "readiness-gates.json"), JSON.stringify(evaluateReadiness(newValidated), null, 2) + "\n");
    writeFileSync(join(altered, "phase11-evidence-manifest.json"), JSON.stringify(generateEvidenceManifest(altered), null, 2) + "\n");
    const qualified = qualifyDossier(altered);
    assert.equal(qualified.source_seal_sha256, newSealDigest);
    assert.equal(rehearseTampering(altered).verified_mutations_rejected, 12);
    assert.throws(() => qualifyDossier(altered, fixtureSourceSealSha256), /source seal/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
