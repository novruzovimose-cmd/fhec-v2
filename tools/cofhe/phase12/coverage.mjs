import { operations, variants, widths } from "./constants.mjs";
import { requireSame } from "./io.mjs";

export function evaluateCoverage(corpus, validated) {
  if (!corpus || corpus.schema !== "fhec-cofhe-phase10-independent-oracle-v1" ||
      !Array.isArray(corpus.cases) || !Array.isArray(corpus.variants) ||
      corpus.variant_mode !== "three-way" ||
      !validated || validated.schema !== "fhec-cofhe-phase10-validated-mock-results-v1") {
    throw new Error("Phase 12 requires a complete independently validated three-way corpus");
  }
  requireSame(corpus.variants, variants, "Expected optimized, tuned and handwritten straight variants");
  requireSame(validated.variants, variants, "Validated variants differ from the full three-way corpus");
  if (corpus.case_count !== corpus.cases.length || validated.cases !== corpus.case_count ||
      !Number.isSafeInteger(corpus.case_count) || corpus.case_count < 72 ||
      validated.successful_mock_executions !== corpus.case_count * variants.length ||
      validated.operation_width_families !== 24 ||
      validated.negative_consuming_contract_test_required !== true) {
    throw new Error("Insufficient corpus, executed variants or ciphertext domain-negative evidence");
  }
  if (!Array.isArray(validated.families) || validated.families.length !== 24) {
    throw new Error("Validated per-family observation matrix is incomplete");
  }
  const seen = new Set();
  const counts = new Map();
  for (const testcase of corpus.cases) {
    if (!testcase || !operations.includes(testcase.operation) || !widths.includes(testcase.width) ||
        typeof testcase.id !== "string" || seen.has(testcase.id)) {
      throw new Error("Malformed or duplicate independent oracle case");
    }
    seen.add(testcase.id);
    const family = testcase.operation + "/u" + testcase.width;
    counts.set(family, (counts.get(family) || 0) + 1);
  }
  const familySummaries = new Map(validated.families.map((record) => [record.family, record]));
  if (familySummaries.size !== 24) throw new Error("Duplicated validated family summary");
  const matrix = [];
  for (const width of widths) {
    for (const operation of operations) {
      const family = operation + "/u" + width;
      const count = counts.get(family) || 0;
      const summary = familySummaries.get(family);
      if (count < 3 || !summary || summary.cases !== count ||
          !summary.variants || Object.keys(summary.variants).sort().join() !== [...variants].sort().join()) {
        throw new Error("Missing or undersampled full-width family: " + family);
      }
      for (const variant of variants) {
        const sample = summary.variants[variant];
        if (!sample || sample.sample_count !== count ||
            !/^[0-9]+$/.test(sample.median_mock_gas) ||
            !/^[0-9]+$/.test(sample.total_mock_gas)) {
          throw new Error("Missing exact mock execution statistics for " + family + "/" + variant);
        }
      }
      matrix.push({ family, cases: count, mock_executions: count * variants.length, variants: [...variants] });
    }
  }
  if (counts.size !== 24) throw new Error("Unexpected operation/width families");
  return {
    schema: "fhec-cofhe-phase12-coverage-v1",
    evidence: "source-sealed locally executed CoFHE mock ciphertexts; never live protocol gas",
    operation_families: 24,
    unsigned_widths: widths,
    operations,
    variants,
    corpus_cases: corpus.case_count,
    verified_mock_executions: validated.successful_mock_executions,
    per_family_minimum: Math.min(...counts.values()),
    contract_domain_negative_test_confirmed: true,
    matrix,
  };
}
