import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, test } from "node:test";
import { evaluatePolicy, execute, nonnegativeInteger, parseArguments } from "./check-cost-policy.mjs";

const report = {
  schema: "fhec-cofhe-static-optimization-v1",
  call_graph: [
    {
      name: "repeated_u8",
      baseline: {
        expanded_encrypted_operations: 3,
        expanded_helper_calls: 0,
        max_helper_call_depth: 0,
      },
      optimized: {
        expanded_encrypted_operations: 2,
        expanded_helper_calls: 0,
        max_helper_call_depth: 0,
      },
    },
    {
      name: "four_times_u8",
      baseline: {
        expanded_encrypted_operations: 3,
        expanded_helper_calls: 3,
        max_helper_call_depth: 1,
      },
      optimized: {
        expanded_encrypted_operations: 3,
        expanded_helper_calls: 3,
        max_helper_call_depth: 1,
      },
    },
  ],
};

const temp = mkdtempSync(join(tmpdir(), "fhec-cost-policy-test-"));
after(() => rmSync(temp, { recursive: true, force: true }));

test("accepts an optimized program exactly on the operation budget", () => {
  const decision = evaluatePolicy(report, {
    entry: "repeated_u8",
    mode: "optimized",
    limits: { operations: 2, calls: 0, depth: 0 },
  });
  assert.equal(decision.verdict, "pass");
  assert.equal(decision.checks.length, 3);
  assert.equal(decision.checks[0].measured, 2);
});

test("rejects baseline using more encrypted operations than allowed", () => {
  const decision = evaluatePolicy(report, {
    entry: "repeated_u8",
    mode: "baseline",
    limits: { operations: 2 },
  });
  assert.equal(decision.verdict, "reject");
  assert.equal(decision.checks[0].measured, 3);
});

test("requires the named function to exist", () => {
  assert.throws(() => evaluatePolicy(report, {
    entry: "unknown", mode: "optimized", limits: { operations: 2 },
  }), /Unknown CoFHE entry/);
});

test("rejects a duplicate call graph entry", () => {
  assert.throws(() => evaluatePolicy({ ...report, call_graph: [report.call_graph[0], report.call_graph[0]] }, {
    entry: "repeated_u8", mode: "optimized", limits: { operations: 2 },
  }), /repeated call-graph/);
});

test("rejects corrupted and unsafe operation counts", () => {
  const invalid = structuredClone(report);
  invalid.call_graph[0].optimized.expanded_encrypted_operations = "2";
  assert.throws(() => evaluatePolicy(invalid, {
    entry: "repeated_u8", mode: "optimized", limits: { operations: 2 },
  }), /Invalid operations profile/);
  invalid.call_graph[0].optimized.expanded_encrypted_operations = Number.MAX_SAFE_INTEGER + 1;
  assert.throws(() => evaluatePolicy(invalid, {
    entry: "repeated_u8", mode: "optimized", limits: { operations: 2 },
  }), /Invalid operations profile/);
});

test("enforces helper expansion and nesting separately", () => {
  const result = evaluatePolicy(report, {
    entry: "four_times_u8", mode: "baseline", limits: { calls: 2, depth: 0 },
  });
  assert.equal(result.verdict, "reject");
  assert.deepEqual(result.checks.map((check) => check.passed), [false, false]);
});

test("validates the report schema", () => {
  assert.throws(() => evaluatePolicy({ ...report, schema: "other" }, {
    entry: "repeated_u8", mode: "optimized", limits: { operations: 2 },
  }), /not an FHEC/);
});

test("parses strict decimal integers including zero", () => {
  assert.equal(nonnegativeInteger("0", "limit"), 0);
  assert.equal(nonnegativeInteger("999", "limit"), 999);
  for (const invalid of ["", "01", "1.0", "-1", "+1", "1e3", "9007199254740992"]) {
    assert.throws(() => nonnegativeInteger(invalid, "limit"));
  }
});

test("rejects duplicate flags and missing required arguments", () => {
  assert.throws(() => parseArguments(["--report", "a.json", "--report", "b.json", "--entry", "one", "--max-ops", "2"]));
  assert.throws(() => parseArguments(["--report", "a.json", "--entry", "one"]));
});

test("rejects a nonnumeric metric and invalid mode", () => {
  assert.throws(() => parseArguments(["--report", "a", "--entry", "one", "--max-ops", "2a"]));
  assert.throws(() => parseArguments(["--report", "a", "--entry", "one", "--max-ops", "2", "--mode", "raw"]));
});

test("policy command writes an immutable machine-readable policy report", () => {
  const sourcePath = join(temp, "report.json");
  const policyPath = join(temp, "accepted.json");
  writeFileSync(sourcePath, JSON.stringify(report));
  const result = execute(["--report", sourcePath, "--entry", "repeated_u8", "--max-ops", "2", "--out", policyPath]);
  assert.equal(result, 0);
  assert.equal(JSON.parse(readFileSync(policyPath, "utf8")).verdict, "pass");
  assert.throws(() => execute(["--report", sourcePath, "--entry", "repeated_u8", "--max-ops", "2", "--out", policyPath]), /EEXIST/);
});

test("policy command returns a dedicated exit code for a failed budget", () => {
  const path = join(temp, "denied-report.json");
  writeFileSync(path, JSON.stringify(report));
  const status = execute(["--report", path, "--entry", "repeated_u8", "--mode", "baseline", "--max-ops", "2"]);
  assert.equal(status, 3);
});
