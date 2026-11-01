import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (char) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[char]);
}

function tableRow(family) {
  const opt = family.variants.optimized;
  const tuned = family.variants.tuned;
  return "<tr>" + [
    family.family,
    family.cases_per_variant,
    opt.transaction_gas.median,
    tuned.transaction_gas.median,
    family.exact_median_gas_difference_tuned_minus_optimized,
    family.optimized_vs_tuned,
    opt.native_transaction_fee_wei.median,
    tuned.native_transaction_fee_wei.median,
  ].map((value) => "<td>" + escapeHtml(value) + "</td>").join("") + "</tr>";
}

function checked(report) {
  if (!report || report.schema !== "fhec-cofhe-phase9-real-testnet-comparison-v1" ||
      report.evidence_tier !== "arbitrum-sepolia-transaction-fees-only-sdk-results-not-cryptographically-attested" ||
      report.chain_id !== 421614 || !Array.isArray(report.families) ||
      report.families.length !== report.family_count ||
      report.coprocessor_fees_measured !== false || report.coprocessor_fee_wei !== null ||
      !Number.isSafeInteger(report.case_count) || report.case_count < 1) {
    throw new Error("Only independently verified Arbitrum Sepolia transaction evidence may be displayed");
  }
  let lower = 0;
  let higher = 0;
  const familyNames = new Set();
  for (const family of report.families) {
    if (familyNames.has(family.family) || !/^(add|sub|mul|gt|max|repeated)\/u(8|16|32|64)$/.test(family.family) ||
        !Number.isSafeInteger(family.cases_per_variant) || family.cases_per_variant < 1 ||
        !family.variants?.optimized?.transaction_gas?.median || !family.variants?.tuned?.transaction_gas?.median) {
      throw new Error("Report family contains duplicates or missing gas observations");
    }
    familyNames.add(family.family);
    const optimized = BigInt(family.variants.optimized.transaction_gas.median);
    const tuned = BigInt(family.variants.tuned.transaction_gas.median);
    const expected = optimized < tuned ? "lower-gas" : optimized > tuned ? "higher-gas" : "equal-gas";
    if (family.optimized_vs_tuned !== expected ||
        BigInt(family.exact_median_gas_difference_tuned_minus_optimized) !== tuned - optimized) {
      throw new Error("Disclosed real-chain performance comparison is inconsistent");
    }
    if (expected === "lower-gas") lower += 1;
    if (expected === "higher-gas") higher += 1;
  }
  if (lower !== report.optimized_lower_gas_families_vs_tuned || higher !== report.optimized_higher_gas_families_vs_tuned ||
      BigInt(report.total_native_chain_fees_wei) !== BigInt(report.deployment_native_chain_fees_wei) + BigInt(report.execution_native_chain_fees_wei)) {
    throw new Error("Displayed real-chain aggregate is inconsistent with family and fee totals");
  }
}

export function renderHtml(report) {
  checked(report);
  const limitations = report.caveats.map((item) => "<li>" + escapeHtml(item) + "</li>").join("");
  return [
    "<!doctype html>",
    '<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">',
    "<title>FHEC Phase 8–9: Arbitrum Sepolia Receipt-Based Benchmark</title>",
    "<style>",
    "body{font:15px/1.55 system-ui,Segoe UI,sans-serif;background:#0b1522;color:#eef4fa;max-width:1160px;margin:auto;padding:32px 22px}",
    "h1,h2{line-height:1.2}h1{font-size:2rem}h2{margin-top:30px;font-size:1.25rem}",
    ".alert{background:#352819;border:1px solid #d7ab69;color:#ffe8ad;border-radius:12px;padding:18px;font-weight:650}",
    ".metrics{display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:12px;margin:23px 0}",
    ".metric{background:#182a40;border:1px solid #35516d;border-radius:11px;padding:14px}",
    ".metric strong{display:block;font-size:1.65rem;color:#92e3ba}.metric small{color:#bdd2e5}",
    ".table{overflow-x:auto;border:1px solid #33506f;border-radius:12px}table{width:100%;min-width:940px;border-collapse:collapse;font-variant-numeric:tabular-nums}",
    "th,td{padding:11px;text-align:right;border-bottom:1px solid #32465f}th:first-child,td:first-child{text-align:left}",
    "th{color:#c6ddf6;background:#213c58}tr:nth-child(even){background:#15263a}",
    "p,li{color:#c1d2e5}code{overflow-wrap:anywhere;color:#97dbbe}",
    "</style></head><body>",
    "<h1>FHEC Phase 8–9: Real Testnet Receipt Evidence</h1>",
    '<div class="alert">REAL ARBITRUM SEPOLIA TRANSACTION FEES ONLY. This report does NOT measure FHE coprocessor service fees or independently prove decrypted FHE results. It is NOT a security audit or mainnet performance forecast.</div>',
    '<div class="metrics">',
    '<div class="metric"><strong>' + report.case_count + '</strong><small>Approved oracle cases</small></div>',
    '<div class="metric"><strong>' + report.successful_onchain_execution_receipts + '</strong><small>Execution receipts corroborated</small></div>',
    '<div class="metric"><strong>' + report.family_count + '</strong><small>Operation-width families</small></div>',
    '<div class="metric"><strong>' + report.optimized_higher_gas_families_vs_tuned + '</strong><small>Families with a higher FHEC gas median</small></div>',
    "</div>",
    "<h2>Full operation-family comparison</h2><p>Each comparison uses identical public demo inputs but separate contract-bound encryption. Signed gas difference = tuned median minus FHEC median. Negative means FHEC used more transaction gas. Small samples are exploratory.</p>",
    '<div class="table"><table><thead><tr><th>Family</th><th>Cases</th><th>FHEC gas median</th><th>Tuned gas median</th><th>Exact gas difference</th><th>Outcome</th><th>FHEC fee median (wei)</th><th>Tuned fee median (wei)</th></tr></thead><tbody>',
    report.families.map(tableRow).join("\n"),
    "</tbody></table></div>",
    "<h2>Real native-chain fee accounting</h2>",
    "<p>Total observed native-chain transaction fees: <code>" + escapeHtml(report.total_native_chain_fees_wei) + " wei</code>.</p>",
    "<p>Deployment fees: <code>" + escapeHtml(report.deployment_native_chain_fees_wei) + " wei</code>; execution fees: <code>" + escapeHtml(report.execution_native_chain_fees_wei) + " wei</code>.</p>",
    "<p>FHE coprocessor service fees: <strong>UNKNOWN / NOT MEASURED</strong>.</p>",
    "<h2>Provenance and limitations</h2><p>Arbitrum Sepolia chain ID 421614. Frozen live plan digest: <code>" + escapeHtml(report.plan_sha256) + "</code>. RPC-audited SDK trace digest: <code>" + escapeHtml(report.trace_sha256) + "</code>.</p>",
    "<ul>" + limitations + "</ul>",
    "<p>This report is static and self-contained; it fetches no scripts, fonts or network resources.</p>",
    "</body></html>\n",
  ].join("\n");
}

export function main(args) {
  if (args.length !== 4 || args[0] !== "--summary" || args[2] !== "--out") {
    throw new Error("Usage: --summary <real-chain-comparison.json> --out <report.html>");
  }
  const report = JSON.parse(readFileSync(args[1], "utf8").replace(/^\uFEFF/, ""));
  writeFileSync(args[3], renderHtml(report), { flag: "wx" });
  process.stdout.write("Generated offline Phase 9 report with real-chain native fees and explicit coprocessor-fee limitations.\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 9 HTML report failed: " + error.message + "\n"); process.exitCode = 1; }
}
