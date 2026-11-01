import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { corpusDigest, createCorpus, digest } from "./oracle.mjs";
import { checkMockResults } from "./validate-results.mjs";

function fake() {
  const corpus = createCorpus(3);
  const seal = Buffer.from("fhec-test-source-seal");
  const corpusBytes = Buffer.from(JSON.stringify(corpus, null, 2) + "\n");
  const records = corpus.cases.map((item) => ({
    id: item.id, width: item.width, operation: item.operation, expected: String(item.expected),
    variants: Object.fromEntries(corpus.variants.map((name) => [name, {
      actual: String(item.expected), oracle_match: true, mock_transaction_gas: "500000",
      sdk_encrypt_ms: 1, mock_execution_and_decryption_ms: 2,
    }])),
  }));
  const raw = {
    schema: "fhec-cofhe-phase10-mock-observations-v1",
    evidence_tier: "locally-executed-hardhat-cofhe-mocks-not-live-network",
    corpus_sha256: corpusDigest(corpus), corpus_file_sha256: digest(corpusBytes),
    source_seal_file_sha256: digest(seal), cases_expected: corpus.case_count,
    cases_recorded: corpus.case_count, full_corpus_completed: true,
    variants: corpus.variants, records,
  };
  return { corpus, seal, corpusBytes, raw };
}

describe("Independent Phase 10 mock result validator", () => {
  it("accepts only a full mock corpus and exact integer gas accounting", () => {
    const { corpus, seal, corpusBytes, raw } = fake();
    const report = checkMockResults(corpus, raw, seal, corpusBytes);
    assert.equal(report.successful_mock_executions, 144);
    assert.equal(report.operation_width_families, 24);
    assert.equal(report.families.length, 24);
    assert.equal(report.families[0].variants.optimized.total_mock_gas, "1500000");
    assert.equal(report.families[0].variants.tuned.median_mock_gas, "500000");
  });
  it("rejects swapped identity, wrong outputs and missing variants", () => {
    const first = fake();
    first.raw.records[0].variants.optimized.actual = "999";
    assert.throws(() => checkMockResults(first.corpus, first.raw, first.seal, first.corpusBytes), /differs/);
    const second = fake();
    second.raw.records.reverse();
    assert.throws(() => checkMockResults(second.corpus, second.raw, second.seal, second.corpusBytes), /ordering/);
    const third = fake();
    delete third.raw.records[2].variants.tuned;
    assert.throws(() => checkMockResults(third.corpus, third.raw, third.seal, third.corpusBytes), /ordering/);
  });
  it("rejects altered source seal, fake mock gas and incomplete evidence", () => {
    const one = fake();
    assert.throws(() => checkMockResults(one.corpus, one.raw, Buffer.from("changed"), one.corpusBytes), /cover/);
    const two = fake();
    two.raw.records[2].variants.optimized.mock_transaction_gas = "-1";
    assert.throws(() => checkMockResults(two.corpus, two.raw, two.seal, two.corpusBytes), /differs/);
    const three = fake();
    three.raw.full_corpus_completed = false;
    assert.throws(() => checkMockResults(three.corpus, three.raw, three.seal, three.corpusBytes), /cover/);
  });
});
