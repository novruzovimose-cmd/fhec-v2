import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { createPacket, verifyPacket } from "./packet.mjs";

const local = {
  schema: "fhec-cofhe-phase13-source-preflight-v1",
  candidate_status: "qualified-local-release-candidate-only",
  phase12_source_files_unchanged: true,
  phase12_recorded_source_files_unchanged: 327,
  phase12_source_inventory_tree_sha256: "A".repeat(64),
};

test("packet writes a source-bound offline summary and rejects tampering", () => {
  const folder = mkdtempSync(join(tmpdir(), "fhec-phase13-packet-"));
  try {
    assert.equal(createPacket(folder, local).externally_corroborated, 0);
    assert.equal(verifyPacket(folder).production_certified, false);
    const report = join(folder, "grant-summary.html");
    const original = readFileSync(report);
    writeFileSync(report, Buffer.concat([original, Buffer.from("<p>Production certified</p>")]));
    assert.throws(() => verifyPacket(folder), /Tampered/);
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
});

test("packet rejects a fabricated grant award even if attacker updates JSON but not manifest", () => {
  const folder = mkdtempSync(join(tmpdir(), "fhec-phase13-fraud-"));
  try {
    createPacket(folder, local);
    const report = join(folder, "evidence.json");
    const modified = JSON.parse(readFileSync(report, "utf8"));
    modified.grant_decision = { awarded_usd: 20000 };
    writeFileSync(report, JSON.stringify(modified));
    assert.throws(() => verifyPacket(folder));
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
});
