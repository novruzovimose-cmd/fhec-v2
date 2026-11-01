import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { it } from "node:test";
import { sourceNames, generateSeal, hashBytes } from "../phase89/source-seal.mjs";
import { createPlan } from "../phase89/plan.mjs";
import { createCorpus, corpusDigest, digest } from "../phase10/oracle.mjs";
import { checkMockResults } from "../phase10/validate-results.mjs";
import { render } from "./report-html.mjs";
import { evaluateReadiness } from "./readiness.mjs";
import { evidenceFiles, generateEvidenceManifest, verifyManifest } from "./attest.mjs";

function syntheticDossier() {
  const dir = mkdtempSync(join(tmpdir(), "fhec-phase11-test-"));
  const source = join(dir, "sources");
  mkdirSync(source);
  for (const file of sourceNames) {
    if (file === "phase89-plan.json") {
      writeFileSync(join(source, file), JSON.stringify(createPlan()) + "\n");
    } else {
      writeFileSync(join(source, file), "pragma solidity ^0.8.28;\ncontract StandaloneEvidenceFixture { }\n");
    }
  }
  const seal = generateSeal(source);
  writeFileSync(join(source, "phase89-source-seal.json"), JSON.stringify(seal, null, 2) + "\n");
  const corpus = createCorpus(2);
  const corpusFile = join(dir, "corpus.json");
  writeFileSync(corpusFile, JSON.stringify(corpus, null, 2) + "\n");
  const raw = {
    schema: "fhec-cofhe-phase10-mock-observations-v1",
    evidence_tier: "locally-executed-hardhat-cofhe-mocks-not-live-network",
    corpus_sha256: corpusDigest(corpus),
    corpus_file_sha256: digest(readFileSync(corpusFile)),
    source_seal_file_sha256: digest(readFileSync(join(source, "phase89-source-seal.json"))),
    cases_expected: corpus.case_count,
    cases_recorded: corpus.case_count,
    full_corpus_completed: true,
    variants: corpus.variants,
    records: corpus.cases.map((item) => ({
      id: item.id, width: item.width, operation: item.operation, expected: String(item.expected),
      variants: Object.fromEntries(corpus.variants.map((name) => [name, {
        actual: String(item.expected), oracle_match: true, mock_transaction_gas: "240000",
        sdk_encrypt_ms: 2, mock_execution_and_decryption_ms: 7,
      }])),
    })),
  };
  writeFileSync(join(dir, "mock-observations.json"), JSON.stringify(raw, null, 2) + "\n");
  const validation = checkMockResults(corpus, raw, readFileSync(join(source, "phase89-source-seal.json")), readFileSync(corpusFile));
  writeFileSync(join(dir, "validated-mock-results.json"), JSON.stringify(validation, null, 2) + "\n");
  writeFileSync(join(dir, "offline-report.html"), render(validation));
  writeFileSync(join(dir, "readiness-gates.json"), JSON.stringify(evaluateReadiness(validation), null, 2) + "\n");
  return dir;
}

it("self-contained dossier validates canonical plan, source bytes, mock corpus, and rendered report", () => {
  const dir = syntheticDossier();
  const result = generateEvidenceManifest(dir);
  assert.equal(result.evidence_file_count, evidenceFiles.length);
  assert.equal(result.independent_mock_executions, 96);
  assert.equal(result.live_transactions_claimed, false);
  assert.deepEqual(verifyManifest(dir, result), result);
  assert.match(result.source_and_evidence_sha256, /^[A-F0-9]{64}$/);
});

it("one-byte changed Solidity or report is rejected without using a live RPC", () => {
  const first = syntheticDossier();
  const manifest = generateEvidenceManifest(first);
  writeFileSync(join(first, "sources", "FHECPhase6Live.sol"), "tampered");
  assert.throws(() => verifyManifest(first, manifest), /modified|seal/i);
  const second = syntheticDossier();
  const old = generateEvidenceManifest(second);
  writeFileSync(join(second, "offline-report.html"), "<b>fake data</b>");
  assert.throws(() => verifyManifest(second, old), /HTML/);
});

it("refuses a fabricated live receipt claim in an otherwise valid local mock dossier", () => {
  const dir = syntheticDossier();
  const result = generateEvidenceManifest(dir);
  result.live_transactions_claimed = true;
  assert.throws(() => verifyManifest(dir, result), /unsupported live/);
});
