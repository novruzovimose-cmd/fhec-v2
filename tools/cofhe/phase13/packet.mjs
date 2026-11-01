import { lstatSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { evaluateGates } from "./gates.mjs";
import { boundedFile, hash, jsonFile, writeNewJson } from "./io.mjs";
import { reportHtml, reportMarkdown } from "./report.mjs";

const files = ["evidence.json", "gates.json", "grant-summary.md", "grant-summary.html"];
const allowed = [...files, "manifest.json"].sort();

export function createPacket(directory, local, live = null, ci = null) {
  const gates = evaluateGates(local, live, ci);
  const evidence = {
    schema: "fhec-cofhe-phase13-evidence-v1",
    created_at_utc: new Date().toISOString(),
    source: local,
    live,
    ci,
    user_facing_state: "evidence-only-not-production-certified-not-grant-awarded",
    grant_decision: null,
  };
  writeNewJson(join(directory, "evidence.json"), evidence);
  writeNewJson(join(directory, "gates.json"), gates);
  writeFileSync(join(directory, "grant-summary.md"), reportMarkdown(gates, local, live, ci), { flag: "wx" });
  writeFileSync(join(directory, "grant-summary.html"), reportHtml(gates, local, live, ci), { flag: "wx" });
  const manifest = {
    schema: "fhec-cofhe-phase13-packet-manifest-v1",
    tamper_evidence: "unsigned-content-hashes-only-not-trusted-signatures",
    files: files.map((file) => {
      const bytes = boundedFile(join(directory, file));
      return { file, bytes: bytes.length, sha256: hash(bytes) };
    }),
    production_certified: false,
    grant_awarded: false,
  };
  writeNewJson(join(directory, "manifest.json"), manifest);
  return verifyPacket(directory);
}

export function verifyPacket(directory) {
  const names = readdirSync(directory).sort();
  if (JSON.stringify(names) !== JSON.stringify(allowed)) throw new Error("Evidence packet has unexpected, missing or renamed content");
  for (const name of names) {
    const entry = lstatSync(join(directory, name));
    if (!entry.isFile() || entry.isSymbolicLink()) throw new Error("Evidence packet contains a symlink or special file");
  }
  const manifest = jsonFile(join(directory, "manifest.json"));
  if (manifest.schema !== "fhec-cofhe-phase13-packet-manifest-v1" ||
      manifest.grant_awarded !== false || manifest.production_certified !== false ||
      !Array.isArray(manifest.files) || manifest.files.length !== files.length) {
    throw new Error("Invalid or promoted Phase 13 evidence manifest");
  }
  for (let index = 0; index < files.length; index += 1) {
    const record = manifest.files[index];
    const content = boundedFile(join(directory, files[index]));
    if (record?.file !== files[index] || record.bytes !== content.length || record.sha256 !== hash(content)) {
      throw new Error("Tampered Phase 13 evidence file: " + files[index]);
    }
  }
  const evidence = jsonFile(join(directory, "evidence.json"));
  if (evidence.schema !== "fhec-cofhe-phase13-evidence-v1" || evidence.grant_decision !== null ||
      evidence.user_facing_state !== "evidence-only-not-production-certified-not-grant-awarded") {
    throw new Error("Evidence file was promoted beyond its checked scope");
  }
  const gates = evaluateGates(evidence.source, evidence.live, evidence.ci);
  if (JSON.stringify(jsonFile(join(directory, "gates.json"))) !== JSON.stringify(gates)) {
    throw new Error("External gate summary differs from its original inputs");
  }
  const markdown = readFileSync(join(directory, "grant-summary.md"), "utf8");
  const html = readFileSync(join(directory, "grant-summary.html"), "utf8");
  if (markdown !== reportMarkdown(gates, evidence.source, evidence.live, evidence.ci) ||
      html !== reportHtml(gates, evidence.source, evidence.live, evidence.ci)) {
    throw new Error("Phase 13 grant report differs from the verified evidence");
  }
  return {
    schema: "fhec-cofhe-phase13-packet-verification-v1",
    local_qualified: true,
    externally_corroborated: gates.external_gates_corroborated,
    total_external_gates: gates.external_gates_total,
    grant_awarded: false,
    production_certified: false,
    warning: "Unsigned local manifest: can detect isolated file changes but not malicious replacement of every file and its manifest",
  };
}
