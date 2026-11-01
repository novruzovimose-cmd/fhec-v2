import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function htmlEscape(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[character]);
}

function signedPercent(value) {
  if (value === null) return "n/a";
  if (typeof value !== "string" || !/^-?[0-9]+\.[0-9][0-9]$/.test(value)) {
    throw new Error("Invalid percentage in mock summary");
  }
  const numeric = Number(value);
  return `${numeric > 0 ? "+" : ""}${value}%`;
}

export function render(summary) {
  if (!summary || summary.schema !== "fhec-cofhe-phase4-gas-summary-v1" ||
      !Array.isArray(summary.families) || summary.families.length !== 24 ||
      !Number.isInteger(summary.total_cases) || summary.total_cases < 24) {
    throw new Error("Invalid or incomplete Phase 4 gas summary");
  }
  const rows = [];
  for (const item of summary.families) {
    const values = item.variants;
    if (!values || !["baseline", "optimized", "manual"].every(
      (variant) => values[variant] && /^[0-9]+(\.5)?$/.test(values[variant].median_mock_gas) &&
      /^[0-9]+$/.test(values[variant].p95_mock_gas),
    )) {
      throw new Error("Malformed gas measurement in Phase 4 summary");
    }
    const improvement = signedPercent(item.optimized_vs_baseline_median_reduction_percent);
    const manualDifference = signedPercent(item.optimized_vs_manual_median_reduction_percent);
    const cells = [
      item.width, item.operation, item.sample_count,
      values.baseline.median_mock_gas,
      values.optimized.median_mock_gas,
      values.manual.median_mock_gas,
      improvement,
      manualDifference,
    ].map((value) => `<td>${htmlEscape(value)}</td>`).join("");
    rows.push(`<tr>${cells}<td><details><summary>p95</summary><span class="mono">B ${htmlEscape(values.baseline.p95_mock_gas)} / O ${htmlEscape(values.optimized.p95_mock_gas)} / M ${htmlEscape(values.manual.p95_mock_gas)}</span></details></td></tr>`);
  }

  const disclaimer = "LOCAL HARDHAT MOCK ONLY — NOT LIVE CoFHE EXECUTION OR NETWORK GAS";
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>FHEC Phase 4 — CoFHE Mock Differential Evidence</title>
  <style>
    :root { color-scheme: dark; --bg: #0b1220; --card: #111c2f; --line: #28405d; --text: #e8effb; --muted: #a5b4cb; --blue: #63b3ff; --amber: #f0bb75; }
    * { box-sizing: border-box; }
    body { margin: 0; background: var(--bg); color: var(--text); font: 15px/1.5 ui-sans-serif, system-ui, sans-serif; }
    main { max-width: 1220px; margin: auto; padding: 34px 22px 60px; }
    h1 { font-size: clamp(26px, 4vw, 42px); line-height: 1.12; margin: 0 0 12px; }
    h2 { margin-top: 34px; font-size: 21px; }
    .eyebrow { color: var(--blue); font-size: 12px; font-weight: 750; letter-spacing: 0.13em; text-transform: uppercase; }
    p { color: var(--muted); max-width: 900px; }
    .warning { background: #3a2817; border: 1px solid #9b652e; padding: 16px 19px; border-radius: 12px; color: var(--amber); font-weight: 800; margin: 26px 0; }
    .cards { display: grid; grid-template-columns: repeat(3, minmax(0,1fr)); gap: 14px; }
    .card { padding: 20px; background: var(--card); border: 1px solid var(--line); border-radius: 14px; }
    .card strong { font-size: 29px; font-variant-numeric: tabular-nums; display: block; }
    .card span { color: var(--muted); font-size: 12px; }
    .table-scroll { overflow-x: auto; background: var(--card); border: 1px solid var(--line); border-radius: 14px; }
    table { border-collapse: collapse; width: 100%; white-space: nowrap; font-variant-numeric: tabular-nums; }
    th, td { padding: 12px 10px; border-bottom: 1px solid var(--line); text-align: right; }
    th { color: var(--muted); background: #17273e; font-size: 12px; }
    th:nth-child(-n+3), td:nth-child(-n+3) { text-align: left; }
    tr:last-child td { border-bottom: 0; }
    tr:hover td { background: #17273e; }
    .mono { font-family: ui-monospace, Consolas, monospace; font-size: 12px; }
    details summary { color: var(--blue); cursor: pointer; }
    li { margin: 7px 0; color: var(--muted); }
    @media (max-width: 640px) { .cards { grid-template-columns: 1fr; } main { padding: 22px 14px; } }
  </style>
</head>
<body>
<main>
  <div class="eyebrow">FHEC V2 / Phase 4 / Engineering evidence</div>
  <h1>CoFHE encrypted computation: three-way differential suite</h1>
  <p>This document summarizes repeatable local Hardhat mock results for FHEC baseline, FHEC optimized, and independent reference Solidity. All figures are labeled by encrypted operation family and bit width.</p>
  <div class="warning">${disclaimer}</div>
  <section class="cards">
    <div class="card"><strong>${htmlEscape(summary.total_cases)}</strong><span>Three-way comparison cases</span></div>
    <div class="card"><strong>24</strong><span>Operation / bit-width families</span></div>
    <div class="card"><strong>3</strong><span>Solidity implementations per case</span></div>
  </section>
  <h2>Per-family transaction gas</h2>
  <p>All numbers include mock proof processing and wrapper overhead. Positive reduction means optimized generated Solidity used fewer gas units than the named baseline; negative means it used more. Neither is real-network gas.</p>
  <div class="table-scroll"><table>
    <thead><tr><th>Width</th><th>Operation</th><th>Cases</th><th>Baseline median</th><th>Optimized median</th><th>Manual median</th><th>Opt vs baseline</th><th>Opt vs manual</th><th>p95 detail</th></tr></thead>
    <tbody>${rows.join("\n")}</tbody>
  </table></div>
  <h2>What these results establish</h2>
  <ul>
    <li>Encrypted inputs were submitted and permissioned results were checked against a separate modular-integer oracle under Fhenix's Hardhat mock execution.</li>
    <li>Baseline and optimized compiler variants were tested alongside an explicit independently authored Solidity implementation.</li>
    <li>These mock measurements are diagnostic evidence only. They do not measure coprocessor execution, live-network fees, protocol security, or throughput.</li>
    <li>For public funding or performance claims, pair this page with compiler commit hashes, generated Solidity hashes, source-limited examples, and a legitimate network deployment receipt.</li>
  </ul>
</main>
</body>
</html>
`;
}

export function main(args) {
  if (args.length !== 4 || args[0] !== "--input" || args[2] !== "--out") {
    throw new Error("Usage: node render-phase4-report.mjs --input <mock-summary.json> --out <report.html>");
  }
  if (resolve(args[1]) === resolve(args[3])) throw new Error("Report must not overwrite summary input");
  const summary = JSON.parse(readFileSync(args[1], "utf8"));
  writeFileSync(args[3], render(summary), { flag: "wx" });
  console.log(`Wrote self-contained CoFHE mock report: ${args[3]}`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`Phase 4 HTML report error: ${error.message}\n`);
    process.exitCode = 1;
  }
}
