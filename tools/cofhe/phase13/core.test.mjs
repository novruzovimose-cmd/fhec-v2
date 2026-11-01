import test from "node:test";
import assert from "node:assert/strict";
import { grantBudget, evaluateGates } from "./gates.mjs";
import { checkedUrl, hash, safeName } from "./io.mjs";
import { parseArgs } from "./cli.mjs";
import { escapeHtml, reportHtml } from "./report.mjs";

const local = {
  schema: "fhec-cofhe-phase13-source-preflight-v1",
  candidate_status: "qualified-local-release-candidate-only",
  phase12_source_files_unchanged: true,
  phase12_recorded_source_files_unchanged: 327,
  phase12_source_inventory_tree_sha256: "A".repeat(64),
};

test("budget has five measurable milestones and totals exactly $20,000", () => {
  const value = grantBudget();
  assert.equal(value.milestones.length, 5);
  assert.equal(value.requested, 20000);
  assert.equal(value.funding_awarded, null);
});

test("offline-only candidate never upgrades any external gate", () => {
  const gates = evaluateGates(local);
  assert.equal(gates.external_gates_corroborated, 0);
  assert.equal(gates.external_gates_total, 4);
  assert.equal(gates.production_certified, false);
  assert.equal(gates.grant_awarded, false);
  assert.equal(gates.gates.find((item) => item.id === "GRANT_REVIEW").status, "outside-tool-control");
});

test("a locally unverified or promoted capsule fails closed", () => {
  assert.throws(() => evaluateGates({ ...local, candidate_status: "production-certified" }));
  assert.throws(() => evaluateGates({ ...local, phase12_source_files_unchanged: false }));
});

test("a fabricated live proof with coprocessor fees silently marked measured is rejected", () => {
  const live = {
    consensus: {
      schema: "fhec-cofhe-phase13-two-provider-testnet-corroboration-v1",
      independent_rpc_agreement: true,
      coprocessor_fees_measured: true,
      transaction_count: 3,
      deployment_count: 1,
      execution_count: 2,
      native_chain_fees_paid_wei: "100",
    },
    analysis: { schema: "fhec-cofhe-phase9-real-testnet-comparison-v1", coprocessor_fees_measured: false, total_native_chain_fees_wei: "100" },
  };
  assert.throws(() => evaluateGates(local, live));
});

test("valid RPC URLs require two distinct public HTTPS endpoints at execution", () => {
  assert.equal(checkedUrl("https://rpc.example.org/eth").hostname, "rpc.example.org");
  assert.throws(() => checkedUrl("http://rpc.example.org"));
  assert.throws(() => checkedUrl("https://name:password@rpc.example.org"));
  assert.throws(() => checkedUrl("https://localhost:8545"));
});

test("safe relative path parser blocks traversal and platform separators", () => {
  assert.equal(safeName("tools/cofhe/check.mjs"), "tools/cofhe/check.mjs");
  for (const candidate of ["../x", "c:\\windows", "/root", "x/./z", "a//b"]) {
    assert.throws(() => safeName(candidate));
  }
});

test("CLI rejects mixed partial evidence and duplicate flags", () => {
  assert.throws(() => parseArgs(["collect", "--candidate", "p", "--repo", "a/b"]));
  assert.throws(() => parseArgs(["collect", "--candidate", "p", "--raw", "r"]));
  assert.throws(() => parseArgs(["live", "--plan", "p", "--raw", "r", "--confirmations", "1"]));
  assert.throws(() => parseArgs(["ci", "--repo", "a/b", "--run", "1", "--sha", "A".repeat(40), "--repo", "x/y"]));
  assert.deepEqual(parseArgs(["collect", "--candidate", "candidate-path"]).flags, { candidate: "candidate-path" });
});

test("HTML report escapes untrusted values", () => {
  assert.equal(escapeHtml("<script>x&y</script>"), "&lt;script&gt;x&amp;y&lt;/script&gt;");
  const gates = evaluateGates(local);
  const html = reportHtml(gates, local);
  assert.match(html, /Not production-certified/);
  assert.match(html, /Coprocessor fees/);
  assert.ok(!html.includes("grant awarded: true"));
});

test("source hashes use uppercase canonical SHA-256", () => {
  assert.equal(hash("abc"), "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD");
});
