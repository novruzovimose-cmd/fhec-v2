import { join, resolve } from "node:path";
import { readFileSync } from "node:fs";
import { readJson, assertRegularFile, sha256 } from "./io.mjs";
import { localEvidenceFiles } from "./constants.mjs";
import { evaluateCoverage } from "./coverage.mjs";
import { verifyManifest } from "../phase11/attest.mjs";
import { evaluateReadiness } from "../phase11/readiness.mjs";

export function qualifyDossier(dossierDirectory, expectedSeal = null) {
  const dossier = resolve(dossierDirectory);
  if (expectedSeal !== null && (typeof expectedSeal !== "string" || !/^[A-Fa-f0-9]{64}$/.test(expectedSeal))) {
    throw new Error("An optional independently recorded source seal must be a 64-character SHA-256 digest");
  }
  for (const name of localEvidenceFiles) assertRegularFile(join(dossier, ...name.split("/")));
  const validated = readJson(join(dossier, "validated-mock-results.json"));
  const corpus = readJson(join(dossier, "corpus.json"));
  const manifest = readJson(join(dossier, "phase11-evidence-manifest.json"));
  verifyManifest(dossier, manifest);
  const seal = manifest.source_seal_file_sha256;
  const actualSeal = sha256(readFileSync(join(dossier, "sources", "phase89-source-seal.json")));
  if (!/^[A-F0-9]{64}$/.test(seal) || actualSeal !== seal ||
      validated.source_seal_file_sha256 !== seal ||
      (expectedSeal !== null && expectedSeal.toUpperCase() !== seal)) {
    throw new Error("Phase 12 source seal does not match independently verified Phase 11 evidence or the optional external source digest");
  }
  const coverage = evaluateCoverage(corpus, validated);
  const readiness = evaluateReadiness(validated);
  const verified = readiness.gates.filter((gate) => gate.status === "verified-local");
  const pending = readiness.gates.filter((gate) => gate.status === "requires-external-evidence");
  if (verified.length !== 4 || pending.length !== 4 ||
      readiness.gates.find((gate) => gate.id === "GRANT_REVIEW")?.status !== "outside-tool-control") {
    throw new Error("The reviewed external evidence policy was weakened or silently rewritten");
  }
  const status = {
    schema: "fhec-cofhe-phase12-local-qualification-v1",
    status: "qualified-local-release-candidate-only",
    production_certified: false,
    grant_awarded: false,
    real_network_test_performed_by_phase12: false,
    source_seal_sha256: seal,
    phase11_evidence_sha256: manifest.source_and_evidence_sha256,
    phase11_manifest_sha256: sha256(JSON.stringify(manifest)),
    full_width_coverage: coverage,
    verified_local_gates: verified.map((item) => item.id),
    pending_external_gates: pending.map((item) => item.id),
    outside_tool_control: ["GRANT_REVIEW"],
    essential_limitations: [
      "Locally executed CoFHE Hardhat mocks do not prove live protocol execution or economic efficiency.",
      "SHA-256 manifests detect accidental or isolated tampering but do not replace authenticated signatures or third-party trust.",
      "Generated code and compilation tests are not an external smart contract or cryptographic security audit.",
      "Hosted Windows/Linux CI needs immutable external run URLs before HOSTED_CI can change status.",
      "The $20,000 funding objective is a proposal, not an awarded or guaranteed grant.",
    ],
  };
  if (status.pending_external_gates.length !== 4 || status.production_certified || status.grant_awarded) {
    throw new Error("Fail-closed local release qualification policy violated");
  }
  return status;
}
