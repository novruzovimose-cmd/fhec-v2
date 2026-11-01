import { lstatSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { sourceNames, hashBytes, readSeal, validateSeal } from "../phase89/source-seal.mjs";
import { corpusDigest, loadCorpus } from "../phase10/oracle.mjs";
import { checkMockResults } from "../phase10/validate-results.mjs";
import { render } from "./report-html.mjs";
import { evaluateReadiness } from "./readiness.mjs";

export const evidenceFiles = Object.freeze([
  "corpus.json",
  "mock-observations.json",
  "validated-mock-results.json",
  "offline-report.html",
  "readiness-gates.json",
  ...sourceNames.map((name) => "sources/" + name),
  "sources/phase89-source-seal.json",
]);

export function verifyEvidenceContents(directory) {
  const sources = join(directory, "sources");
  const sealPath = join(sources, "phase89-source-seal.json");
  validateSeal(sources, readSeal(sealPath));
  const corpusPath = join(directory, "corpus.json");
  const corpus = loadCorpus(corpusPath);
  const raw = JSON.parse(readFileSync(join(directory, "mock-observations.json"), "utf8").replace(/^\uFEFF/, ""));
  const actual = JSON.parse(readFileSync(join(directory, "validated-mock-results.json"), "utf8").replace(/^\uFEFF/, ""));
  const recomputed = checkMockResults(corpus, raw, readFileSync(sealPath), readFileSync(corpusPath));
  if (JSON.stringify(actual) !== JSON.stringify(recomputed)) {
    throw new Error("Phase 11 bundle result summary differs from the independently recomputed oracle check");
  }
  const displayed = readFileSync(join(directory, "offline-report.html"), "utf8");
  if (displayed !== render(actual)) throw new Error("Phase 11 HTML has been modified after independent validation");
  const readiness = JSON.parse(readFileSync(join(directory, "readiness-gates.json"), "utf8").replace(/^\uFEFF/, ""));
  if (JSON.stringify(readiness) !== JSON.stringify(evaluateReadiness(actual))) {
    throw new Error("Readiness gates were altered to claim unverified external tests or grant funding");
  }
  return actual;
}

export function generateEvidenceManifest(directory) {
  const validated = verifyEvidenceContents(directory);
  const entries = evidenceFiles.map((name) => {
    const path = join(directory, name);
    const stat = lstatSync(path);
    if (!stat.isFile() || stat.isSymbolicLink()) throw new Error("Evidence bundle file must be a regular, non-symlink file: " + name);
    const bytes = readFileSync(path);
    return { file: name, bytes: bytes.length, sha256: hashBytes(bytes) };
  });
  return {
    schema: "fhec-cofhe-phase11-offline-evidence-dossier-v1",
    evidence_tier: "locally-executed-cofhe-mocks-with-independent-arithmetic-replay",
    live_transactions_claimed: false,
    coprocessor_fees_measured: false,
    source_file_count: sourceNames.length + 1,
    evidence_file_count: entries.length,
    corpus_sha256: validated.corpus_sha256,
    source_seal_file_sha256: validated.source_seal_file_sha256,
    independent_mock_executions: validated.successful_mock_executions,
    source_and_evidence_sha256: hashBytes(JSON.stringify(entries)),
    files: entries,
    limitations: [
      "No real network transactions are sent by this dossier generator.",
      "Replay validates mock-observed values against an independent reference function; this is not a formal proof.",
      "Code fingerprints and local test results cannot substitute for independent live protocol metering.",
    ],
  };
}

export function verifyManifest(directory, manifest) {
  if (!manifest || manifest.schema !== "fhec-cofhe-phase11-offline-evidence-dossier-v1" ||
      manifest.live_transactions_claimed !== false || manifest.coprocessor_fees_measured !== false ||
      JSON.stringify(manifest) !== JSON.stringify(generateEvidenceManifest(directory))) {
    throw new Error("Phase 11 offline bundle changed, omitted required evidence or claimed unsupported live results");
  }
  return manifest;
}

export function main(args) {
  if (args.length !== 4 || args[0] !== "--directory" || !["--write", "--verify"].includes(args[2])) {
    throw new Error("Usage: --directory <evidence-folder> --write <new-manifest> | --directory <folder> --verify <manifest>");
  }
  const directory = resolve(args[1]);
  if (args[2] === "--write") {
    const manifest = generateEvidenceManifest(directory);
    writeFileSync(args[3], JSON.stringify(manifest, null, 2) + "\n", { flag: "wx" });
    process.stdout.write(`Phase 11 dossier sealed: ${manifest.evidence_file_count} files, ${manifest.independent_mock_executions} validated mock executions, ${manifest.source_and_evidence_sha256}\n`);
  } else {
    const manifest = verifyManifest(directory, JSON.parse(readFileSync(args[3], "utf8").replace(/^\uFEFF/, "")));
    process.stdout.write(`Phase 11 offline evidence independently verified: ${manifest.evidence_file_count} files and ${manifest.independent_mock_executions} mock executions.\n`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 11 evidence dossier failed: " + error.message + "\n"); process.exitCode = 1; }
}
