import { resolve, join } from "node:path";
import { readJson, requireSame } from "./io.mjs";
import { qualifyDossier } from "./qualification.mjs";

function semanticRecords(dossier) {
  const raw = readJson(join(dossier, "mock-observations.json"));
  return raw.records.map((record) => ({
    id: record.id,
    operation: record.operation,
    width: record.width,
    expected: record.expected,
    actual: Object.fromEntries(Object.entries(record.variants).map(([name, value]) => [name, value.actual])),
  }));
}

export function compareIndependentRuns(firstDossier, secondDossier) {
  const left = resolve(firstDossier);
  const right = resolve(secondDossier);
  if (left === right) throw new Error("Two independent dossier directory paths must be supplied");
  const first = qualifyDossier(left);
  const second = qualifyDossier(right);
  if (first.full_width_coverage.corpus_cases !== second.full_width_coverage.corpus_cases ||
      first.phase11_evidence_sha256 === "" || second.phase11_evidence_sha256 === "") {
    throw new Error("Dossier sizes or digests are incompatible");
  }
  const a = readJson(join(left, "corpus.json"));
  const b = readJson(join(right, "corpus.json"));
  requireSame(a, b, "Independent local CoFHE runs did not use identical reviewed oracle inputs");
  requireSame(semanticRecords(left), semanticRecords(right), "Independent local CoFHE runs returned different decrypted results");
  return {
    schema: "fhec-cofhe-phase12-reproducibility-comparison-v1",
    scope: "deterministic oracle, function and decrypted mock outputs; excludes nondeterministic wall time and mock gas",
    compared_runs: 2,
    identical_oracle_inputs: true,
    identical_mock_decrypted_outputs: true,
    source_seal_sha256: first.source_seal_sha256,
    compared_mock_executions: first.full_width_coverage.verified_mock_executions + second.full_width_coverage.verified_mock_executions,
  };
}
