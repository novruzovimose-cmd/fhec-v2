import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[character]);
}

function formatBasisPoints(input) {
  const number = BigInt(input);
  const sign = number > 0n ? "+" : number < 0n ? "-" : "";
  const magnitude = number < 0n ? -number : number;
  return sign + (magnitude / 100n) + "." + (magnitude % 100n).toString().padStart(2, "0") + "%";
}

function familyTable(report) {
  return report.families.map((row) => {
    const optimized = row.variants.optimized;
    const straight = row.variants.straight;
    const tuned = row.variants.tuned;
    return "<tr>" + [
      row.family,
      row.cases,
      optimized.gas_median,
      straight.gas_median,
      tuned.gas_median,
      optimized.gas_p95,
      formatBasisPoints(row.gas_saving_vs_straight_basis_points),
      formatBasisPoints(row.gas_saving_vs_tuned_basis_points),
      row.comparison_vs_tuned + " (" + row.exact_gas_difference_vs_tuned + " mock gas)",
    ].map((value) => "<td>" + escapeHtml(value) + "</td>").join("") + "</tr>";
  }).join("\n");
}

function checkFamilyTable(report) {
  const validNames = new Set();
  const validFamily = /^(add|sub|mul|gt|max|repeated)\/u(8|16|32|64)$/;
  const signed = /^-?(0|[1-9][0-9]*)$/;
  let straightWins = 0;
  let tunedWins = 0;
  let tunedRegressions = 0;
  for (const row of report.families) {
    if (!validFamily.test(row.family) || validNames.has(row.family) ||
        row.cases !== report.case_count / 24) {
      throw new Error("Duplicated or incomplete operation-width family in evidence table");
    }
    validNames.add(row.family);
    const values = Object.values(row.variants ?? {});
    if (values.length !== 3 || values.some((value) => !/^[1-9][0-9]*$/.test(value.gas_median))) {
      throw new Error("Incomplete benchmark gas values");
    }
    const optimized = BigInt(row.variants.optimized.gas_median);
    const straight = BigInt(row.variants.straight.gas_median);
    const tuned = BigInt(row.variants.tuned.gas_median);
    const straightStatus = optimized < straight ? "win" : optimized > straight ? "regression" : "tie";
    const tunedStatus = optimized < tuned ? "win" : optimized > tuned ? "regression" : "tie";
    if (row.comparison_vs_straight !== straightStatus || row.comparison_vs_tuned !== tunedStatus ||
        !signed.test(row.exact_gas_difference_vs_straight) || !signed.test(row.exact_gas_difference_vs_tuned) ||
        BigInt(row.exact_gas_difference_vs_straight) !== straight - optimized ||
        BigInt(row.exact_gas_difference_vs_tuned) !== tuned - optimized ||
        row.gas_saving_vs_straight_basis_points !== ((straight - optimized) * 10000n / straight).toString() ||
        row.gas_saving_vs_tuned_basis_points !== ((tuned - optimized) * 10000n / tuned).toString()) {
      throw new Error("Benchmark comparison direction or exact signed delta is inconsistent");
    }
    if (straightStatus === "win") straightWins += 1;
    if (tunedStatus === "win") tunedWins += 1;
    if (tunedStatus === "regression") tunedRegressions += 1;
  }
  if (straightWins !== report.win_families_vs_straight || tunedWins !== report.win_families_vs_tuned ||
      tunedRegressions !== report.regression_families_vs_tuned) {
    throw new Error("Published aggregate does not match the complete family evidence");
  }
}

export function renderReport(report, receiptReport = null) {
  if (!report || report.schema !== "fhec-cofhe-phase7-verified-mock-comparison-v1" ||
      report.evidence_tier !== "verified-local-mock-only" ||
      !Array.isArray(report.families) || report.families.length !== 24 ||
      !Number.isSafeInteger(report.case_count) || report.case_count < 24 ||
      report.mock_transactions_verified !== report.case_count * 3 ||
      report.neither_live_gas_nor_coprocessor_fees_measured !== true) {
    throw new Error("Only a complete independently verified Phase 7 mock summary may be published");
  }
  if (receiptReport && (
    receiptReport.schema !== "fhec-cofhe-phase7-rpc-corroboration-v1" ||
    receiptReport.evidence_tier !== "testnet-transport-receipt-verified-sdk-decryption-not-independently-attested" ||
    receiptReport.verified_transaction_count !== 3
  )) throw new Error("Unexpected testnet receipt evidence");
  checkFamilyTable(report);
  const live = receiptReport
    ? "<section><h2>Separate testnet transport corroboration</h2><p>Three SDK-reported execution transactions were matched against RPC receipts on Arbitrum Sepolia. This does not independently prove FHE computation, confidential data handling, or coprocessor cost.</p><p>Contract: <code>" +
      escapeHtml(receiptReport.contract) + "</code></p></section>"
    : "<section><h2>Live testnet status</h2><p>No live network evidence was supplied to this report. All numerical measurements below come from local mocks.</p></section>";
  const limitations = report.methodology.map((item) => "<li>" + escapeHtml(item) + "</li>").join("");
  const title = "FHEC Phase 7: CoFHE Differential Benchmark Evidence";
  return [
    "<!doctype html>",
    '<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">',
    "<title>" + escapeHtml(title) + "</title>",
    "<style>",
    "body{background:#101827;color:#ebf2fb;font:15px/1.58 system-ui,Segoe UI,sans-serif;max-width:1200px;margin:0 auto;padding:36px 22px}",
    "h1,h2{line-height:1.15}h1{font-size:2rem}h2{font-size:1.25rem;margin-top:38px}",
    ".notice{padding:16px 18px;border:1px solid #efb765;border-radius:12px;background:#352719;color:#ffe5a7;font-weight:650}",
    ".grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(175px,1fr));gap:12px;margin:25px 0}",
    ".metric{background:#1d2b40;border:1px solid #354965;border-radius:10px;padding:15px}",
    ".metric strong{display:block;font-size:1.7rem;color:#79d5a0}.metric small{color:#c4d1e6}",
    ".table{overflow-x:auto;border:1px solid #354965;border-radius:10px}",
    "table{border-collapse:collapse;min-width:930px;width:100%;font-variant-numeric:tabular-nums}",
    "th,td{border-bottom:1px solid #32445e;padding:11px 12px;text-align:right}",
    "th{background:#22344b;color:#d0e3ff}th:first-child,td:first-child{text-align:left}",
    "tr:nth-child(even){background:#18263a}tr:hover{background:#2a405c}",
    "code{overflow-wrap:anywhere;color:#a6dfbe}p,li{color:#d0dcee}",
    "</style></head><body>",
    "<header><h1>" + escapeHtml(title) + "</h1>",
    '<div class="notice">LOCAL MOCKS ONLY — These measurements are NOT real CoFHE gas, network latency, or independent cryptographic execution evidence.</div></header>',
    '<div class="grid">',
    '<div class="metric"><strong>' + report.case_count + '</strong><small>Oracle cases compared</small></div>',
    '<div class="metric"><strong>' + report.mock_transactions_verified + '</strong><small>Distinct mock transactions checked</small></div>',
    '<div class="metric"><strong>' + report.win_families_vs_straight + '/24</strong><small>Families with lower optimized median vs straightforward reference</small></div>',
    '<div class="metric"><strong>' + report.regression_families_vs_tuned + '/24</strong><small>Families regressing vs tuned reference</small></div>',
    "</div>",
    "<section><h2>All operation-width families</h2><p>Positive percentage indicates a lower median mock transaction gas for FHEC. Negative means regression. Results include wrapper overhead.</p>",
    '<div class="table"><table><thead><tr><th>Operation</th><th>Cases</th><th>FHEC median</th><th>Straight median</th><th>Tuned median</th><th>FHEC p95</th><th>Δ vs straight</th><th>Δ vs tuned</th><th>Versus tuned</th></tr></thead><tbody>',
    familyTable(report),
    "</tbody></table></div></section>",
    live,
    "<section><h2>Measurement methodology and limits</h2><ul>" + limitations + "</ul>",
    "<p>Oracle SHA-256: <code>" + escapeHtml(report.source_matrix_sha256) + "</code></p>",
    "<p>All data is static and self-contained. No trackers, remote fonts or scripts are loaded.</p></section>",
    "</body></html>\n",
  ].join("\n");
}

function parse(path) {
  return JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
}

export function main(args) {
  if (args.length !== 4 && args.length !== 6) {
    throw new Error("Usage: --summary <verified-mock.json> --out <report.html> [--receipts <verified-rpc.json>]");
  }
  if (args[0] !== "--summary" || args[2] !== "--out" ||
      (args.length === 6 && args[4] !== "--receipts")) throw new Error("Unexpected report generator options");
  const report = renderReport(parse(args[1]), args.length === 6 ? parse(args[5]) : null);
  writeFileSync(args[3], report, { flag: "wx" });
  process.stdout.write("Phase 7 self-contained HTML engineering report generated.\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 7 report rendering failed: " + error.message + "\n"); process.exitCode = 1; }
}
