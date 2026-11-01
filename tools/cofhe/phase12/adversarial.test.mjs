import { test } from "node:test";
import assert from "node:assert/strict";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { rehearseTampering, attackNames } from "./adversarial.mjs";

const fixture = join(dirname(fileURLToPath(import.meta.url)), "fixtures", "mock-dossier");

test("12 independent evidence forgery classes must be refused in isolated scratch copies", () => {
  const result = rehearseTampering(fixture);
  assert.equal(result.verified_mutations_rejected, 12);
  assert.equal(result.all_rejected, true);
  assert.deepEqual(result.attacks, attackNames());
});
