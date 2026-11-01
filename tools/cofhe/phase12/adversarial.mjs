import { copyFileSync, cpSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { qualifyDossier } from "./qualification.mjs";

const attacks = Object.freeze([
  { name: "oracle_expected_value", path: "corpus.json", search: /"expected": "0"/, replacement: '"expected": "1"' },
  { name: "mock_observed_ciphertext_value", path: "mock-observations.json", search: /"actual": "0"/, replacement: '"actual": "1"' },
  { name: "mock_fee_forgery", path: "mock-observations.json", search: /"mock_transaction_gas": "[0-9]+"/, replacement: '"mock_transaction_gas": "1"' },
  { name: "false_mock_execution_total", path: "validated-mock-results.json", search: /"successful_mock_executions": [0-9]+/, replacement: '"successful_mock_executions": 900' },
  { name: "false_liveness_promotion", path: "readiness-gates.json", search: /"requires-external-evidence"/, replacement: '"verified-local"' },
  { name: "source_byte_mutation", path: "sources/FHECPhase6Optimized.sol", suffix: "\n" },
  { name: "source_seal_mutation", path: "sources/phase89-source-seal.json", search: /"files_count": 7/, replacement: '"files_count": 8' },
  { name: "manifest_digest_mutation", path: "phase11-evidence-manifest.json", search: /"source_and_evidence_sha256": "[A-F0-9]+"/, replacement: '"source_and_evidence_sha256": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"' },
  { name: "manifest_schema_mutation", path: "phase11-evidence-manifest.json", search: /"schema": "fhec-cofhe-phase11-offline-evidence-dossier-v1"/, replacement: '"schema": "fake-bundle-v1"' },
  { name: "html_rewrite", path: "offline-report.html", suffix: "\n<div>LIVE PRODUCTION VERIFIED</div>" },
  { name: "negative_domain_downgrade", path: "validated-mock-results.json", search: /"negative_consuming_contract_test_required": true/, replacement: '"negative_consuming_contract_test_required": false' },
  { name: "extra_evidence_without_manifest", path: "mock-observations.json", suffix: " " },
]);

export function attackNames() {
  return attacks.map((item) => item.name);
}

export function rehearseTampering(dossier, expectedSeal = null) {
  qualifyDossier(dossier, expectedSeal);
  const rejected = [];
  const temporary = mkdtempSync(join(tmpdir(), "fhec-phase12-attack-"));
  try {
    for (let index = 0; index < attacks.length; index++) {
      const attack = attacks[index];
      const copied = join(temporary, "attack-" + index);
      cpSync(dossier, copied, { recursive: true, force: false, errorOnExist: true });
      const file = join(copied, ...attack.path.split("/"));
      const original = readFileSync(file, "utf8");
      let altered;
      if (typeof attack.suffix === "string") {
        altered = original + attack.suffix;
      } else {
        if (!attack.search.test(original)) throw new Error("Adversarial fixture pattern no longer matches: " + attack.name);
        altered = original.replace(attack.search, attack.replacement);
      }
      if (altered === original) throw new Error("Attack was a no-op: " + attack.name);
      writeFileSync(file, altered);
      let rejectedByVerifier = false;
      try { qualifyDossier(copied, expectedSeal); }
      catch { rejectedByVerifier = true; }
      if (!rejectedByVerifier) throw new Error("FAIL OPEN: adversarial evidence was accepted: " + attack.name);
      rejected.push(attack.name);
    }
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
  return {
    schema: "fhec-cofhe-phase12-fail-closed-mutation-suite-v1",
    scope: "isolated temporary copies of existing local mock evidence only",
    verified_mutations_rejected: rejected.length,
    all_rejected: rejected.length === attacks.length,
    attacks: rejected,
  };
}
