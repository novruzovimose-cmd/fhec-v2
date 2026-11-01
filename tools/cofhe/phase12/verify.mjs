import { existsSync, lstatSync, readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { candidateManifest } from "./candidate.mjs";
import { localEvidenceFiles } from "./constants.mjs";
import { fileEntry, identical, readJson, requireSame } from "./io.mjs";
import { verifySourceInventory, validateSourceInventory } from "./inventory.mjs";
import { qualifyDossier } from "./qualification.mjs";
import { renderQualification } from "./report.mjs";
import { handoffMarkdown } from "./handoff.mjs";

export function enumerateCandidate(root) {
  const files = [];
  function visit(name) {
    const dir = join(root, ...name.split("/").filter(Boolean));
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const relative = name ? name + "/" + entry.name : entry.name;
      const path = join(dir, entry.name);
      const stat = lstatSync(path);
      if (stat.isSymbolicLink()) throw new Error("Symlink inside release capsule: " + relative);
      if (stat.isDirectory()) visit(relative);
      else if (stat.isFile()) files.push(relative);
      else throw new Error("Unexpected device or special file inside candidate: " + relative);
    }
  }
  visit("");
  return files.sort();
}

export function verifyCandidate(directory, projectRoot, expectedSeal = null) {
  const root = resolve(directory);
  const manifest = readJson(join(root, "phase12-release-manifest.json"));
  if (!manifest || manifest.schema !== "fhec-cofhe-phase12-candidate-manifest-v1" ||
      manifest.release_status !== "local-release-candidate-only" || manifest.published !== false ||
      manifest.production_certified !== false || manifest.grant_awarded !== false) {
    throw new Error("Release candidate manifest has been corrupted or promoted beyond verified evidence");
  }
  const expected = ["phase12-release-manifest.json", "qualification.json", "source-inventory.json",
    "qualification-report.html", "RELEASE-CANDIDATE.md", ...localEvidenceFiles.map((name) => "phase11/" + name)].sort();
  requireSame(enumerateCandidate(root), expected, "Candidate contains unexpected, missing, hidden, symlinked or duplicated files");
  for (const entry of manifest.files) {
    const actual = fileEntry(root, entry.file);
    requireSame(actual, entry, "Candidate file content or length does not match its recorded digest: " + entry.file);
  }
  const qualification = qualifyDossier(join(root, "phase11"), expectedSeal);
  const inventory = readJson(join(root, "source-inventory.json"));
  validateSourceInventory(inventory);
  requireSame(readJson(join(root, "qualification.json")), qualification, "Qualification was changed after independent replay");
  if (projectRoot) verifySourceInventory(projectRoot, inventory);
  if (readFileSync(join(root, "qualification-report.html"), "utf8") !== renderQualification(qualification, inventory)) {
    throw new Error("Rendered HTML report is not a faithful representation of the verified evidence");
  }
  if (readFileSync(join(root, "RELEASE-CANDIDATE.md"), "utf8") !== handoffMarkdown(qualification, inventory)) {
    throw new Error("Grant handoff text has been modified independently of the evidence");
  }
  requireSame(manifest, candidateManifest(root, qualification, inventory), "Candidate manifest contents disagree with verified file inventory");
  return { path: root, status: qualification.status, files: manifest.files.length, corpus_cases: qualification.full_width_coverage.corpus_cases };
}
