import { cpSync, existsSync, lstatSync, mkdirSync, mkdtempSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { localEvidenceFiles } from "./constants.mjs";
import { assertRegularFile, fileEntry, jsonBytes, sha256, writeJsonNew } from "./io.mjs";
import { sourceInventory } from "./inventory.mjs";
import { qualifyDossier } from "./qualification.mjs";
import { renderQualification } from "./report.mjs";
import { handoffMarkdown } from "./handoff.mjs";

export const metadataFiles = Object.freeze([
  "qualification.json", "source-inventory.json", "qualification-report.html", "RELEASE-CANDIDATE.md",
]);

export function candidateManifest(directory, qualification, inventory) {
  const files = [
    ...localEvidenceFiles.map((name) => "phase11/" + name),
    ...metadataFiles,
  ].map((name) => fileEntry(directory, name));
  return {
    schema: "fhec-cofhe-phase12-candidate-manifest-v1",
    release_status: "local-release-candidate-only",
    published: false,
    production_certified: false,
    grant_awarded: false,
    source_seal_sha256: qualification.source_seal_sha256,
    evidence_entries_sha256: qualification.phase11_evidence_sha256,
    selected_source_tree_sha256: inventory.tree_sha256,
    evidence_tier: "independently-replayed-local-cofhe-hardhat-mocks",
    files_sha256: sha256(JSON.stringify(files)),
    files,
  };
}

export function createCandidate({ project, dossier, output, expectedSeal = null }) {
  const projectRoot = resolve(project);
  const dossierRoot = resolve(dossier);
  const outputDirectory = resolve(output);
  if (existsSync(outputDirectory)) throw new Error("Refusing to overwrite an existing release candidate directory");
  if (!outputDirectory.startsWith(join(projectRoot, "target") + "/") &&
      !outputDirectory.startsWith(join(projectRoot, "target") + "\\")) {
    throw new Error("Release candidate output must be a fresh directory under the project's target directory");
  }
  for (const candidatePath of [projectRoot, join(projectRoot, "target"), dirname(outputDirectory)]) {
    if (existsSync(candidatePath)) {
      const stat = lstatSync(candidatePath);
      if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error("Refusing release candidate output through a symlink or special directory");
    }
  }
  const qualification = qualifyDossier(dossierRoot, expectedSeal);
  const inventory = sourceInventory(projectRoot);
  mkdirSync(dirname(outputDirectory), { recursive: true });
  const stage = mkdtempSync(join(dirname(outputDirectory), ".cofhe-phase12-staging-"));
  try {
    for (const name of localEvidenceFiles) {
      const destination = join(stage, "phase11", ...name.split("/"));
      const source = join(dossierRoot, ...name.split("/"));
      assertRegularFile(source);
      mkdirSync(dirname(destination), { recursive: true });
      cpSync(source, destination, { errorOnExist: true, force: false });
    }
    writeJsonNew(join(stage, "qualification.json"), qualification);
    writeJsonNew(join(stage, "source-inventory.json"), inventory);
    writeFileSync(join(stage, "qualification-report.html"), renderQualification(qualification, inventory), { flag: "wx" });
    writeFileSync(join(stage, "RELEASE-CANDIDATE.md"), handoffMarkdown(qualification, inventory), { flag: "wx" });
    const manifest = candidateManifest(stage, qualification, inventory);
    writeJsonNew(join(stage, "phase12-release-manifest.json"), manifest);
    if (existsSync(outputDirectory)) throw new Error("Release candidate destination appeared during generation");
    renameSync(stage, outputDirectory);
  } catch (error) {
    if (existsSync(stage)) rmSync(stage, { recursive: true, force: true });
    throw error;
  }
  return { path: outputDirectory, source_files: inventory.source_files, mock_executions: qualification.full_width_coverage.verified_mock_executions };
}
