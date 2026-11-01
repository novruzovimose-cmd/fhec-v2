import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { sourceInventory, verifySourceInventory, validateSourceInventory } from "./inventory.mjs";

export function mockProject(root) {
  const names = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "crates/unit/src/lib.rs",
    "integration/cofhe-hardhat/pnpm-lock.yaml", "integration/cofhe-hardhat/package.json",
    "tools/cofhe/phase12/synthetic.mjs", ...Array.from({ length: 13 }, (_, index) => "docs/contract-" + index + ".md")];
  for (const name of names) {
    const full = join(root, ...name.split("/"));
    mkdirSync(join(full, ".."), { recursive: true });
    writeFileSync(full, "reviewed source asset: " + name + "\n");
  }
  return names;
}

test("source inventory produces deterministic, sorted file digests", () => {
  const root = mkdtempSync(join(tmpdir(), "fhec-phase12-inventory-"));
  try {
    mockProject(root);
    const first = sourceInventory(root);
    const second = sourceInventory(root);
    assert.deepEqual(first, second);
    assert.ok(first.source_files >= 15);
    assert.equal(first.tree_sha256.length, 64);
    verifySourceInventory(root, first);
    writeFileSync(join(root, "crates/unit/src/lib.rs"), "mutated source\n");
    assert.throws(() => verifySourceInventory(root, first), /changed/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("release inventory excludes generated output, installed modules and target evidence", () => {
  const root = mkdtempSync(join(tmpdir(), "fhec-phase12-inventory-"));
  try {
    mockProject(root);
    for (const name of ["tools/cofhe/phase12/node_modules/secret.json", "target/a.json", "integration/cofhe-hardhat/phase89/generated/seed.json"]) {
      const path = join(root, name);
      mkdirSync(join(path, ".."), { recursive: true });
      writeFileSync(path, "fixture\n");
    }
    const inventory = sourceInventory(root);
    assert.ok(inventory.files.every((value) => !value.file.includes("secret.json") && !value.file.includes("generated/") && !value.file.includes("target/")));
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("release inventory fails closed on source-like private keys", () => {
  const root = mkdtempSync(join(tmpdir(), "fhec-phase12-inventory-"));
  try {
    mockProject(root);
    writeFileSync(join(root, "docs/private-key.json"), '{"privateKey":"mock"}\n');
    assert.throws(() => sourceInventory(root), /Credential-like/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test("offline manifest refuses a fabricated file digest even when no source checkout exists", () => {
  const root = mkdtempSync(join(tmpdir(), "fhec-phase12-inventory-"));
  try {
    mockProject(root);
    const changed = structuredClone(sourceInventory(root));
    changed.files[0].sha256 = "0".repeat(64);
    assert.throws(() => validateSourceInventory(changed), /digest/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
