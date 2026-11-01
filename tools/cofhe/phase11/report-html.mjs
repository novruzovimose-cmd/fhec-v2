import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const escape = (value) => String(value).replace(/[&<>"']/g, (v) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[v]);

export function render(report) {
  if (!report || report.schema !== "fhec-cofhe-phase10-validated-mock-results-v1" ||
      report.evidence_tier !== "independently-recomputed-local-mock-differential-results" ||
      !Number.isSafeInteger(report.cases) || report.cases <= 0 ||
      report.operation_width_families !== 24 || !Array.isArray(report.families) || report.families.length !== 24 ||
      !Array.isArray(report.variants) || !report.variants.includes("optimized") || !report.variants.includes("tuned")) {
    throw new Error("Phase 11 refuses an incomplete or non-mock evidence report");
  }
  const rows = report.families.map((item) => {
    const cells = report.variants.map((variant) => {
      const data = item.variants[variant];
      if (!data || !/^[0-9]+$/.test(String(data.median_mock_gas))) throw new Error("Missing complete family/variant evidence");
      return `<td><strong>${escape(data.median_mock_gas)}</strong><small>${escape(data.sample_count)} samples</small></td>`;
    }).join("");
    return `<tr><th scope="row">${escape(item.family)}</th>${cells}</tr>`;
  }).join("\n");
  const head = report.variants.map((name) => `<th scope="col">${escape(name)}<small>median MOCK gas</small></th>`).join("");
  const caveats = (report.limitations || []).map((item) => `<li>${escape(item)}</li>`).join("");
  return `<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>FHEC CoFHE Phase 10–11 — Reproducible Local Mock Evidence</title>
<style>
:root{color-scheme:dark;background:#07101e;color:#eff6ff;font:16px/1.6 system-ui,Segoe UI,sans-serif}*{box-sizing:border-box}body{margin:0 auto;max-width:1120px;padding:36px 20px 80px}h1{font-size:clamp(2rem,5vw,3.3rem);line-height:1.1;letter-spacing:-.035em}h2{margin-top:38px}.eyebrow{color:#8ceccc;letter-spacing:.13em;text-transform:uppercase;font-size:.79rem;font-weight:800}p{color:#c5d7e8}strong{color:#fff}.warning{border:1px solid #d7a747;background:#2d251a;color:#fff3ca;padding:16px;border-radius:12px}.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:12px;margin:26px 0}.card{background:#122238;border:1px solid #2a4161;border-radius:14px;padding:20px}.number{font-size:2rem;color:#94f2d5;font-weight:800;display:block}.card small{color:#afc6dc}.table{overflow:auto;border:1px solid #293e56;border-radius:13px}table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}th,td{padding:12px 16px;border-bottom:1px solid #233d55;text-align:left}thead{background:#10293e}tbody tr:nth-child(even){background:#0f1b2e}td small,th small{display:block;color:#97b5d2;font-size:.74rem}li{padding:4px 0}code{word-break:break-all;color:#8ceccc}.foot{font-size:.8rem;color:#90a9bc;margin-top:34px}
</style></head>
<body><p class="eyebrow">FHEC V2 / independent test corpus / offline evidence</p>
<h1>Encrypted operations, sampled across all supported unsigned widths</h1>
<p>Phase 10 compiles source-sealed CoFHE Solidity wrappers against a deterministic independent BigInt oracle. Phase 11 independently checks the saved mock outputs and packages them for reproducibility.</p>
<div class="warning"><strong>LOCAL MOCKS ONLY.</strong> These gas readings are from a Hardhat CoFHE mock environment. They are not live Arbitrum Sepolia transaction gas, real network costs, FHE coprocessor billing, production latency or cryptographic proof of correctness.</div>
<div class="cards"><div class="card"><span class="number">${escape(report.cases)}</span><small>Independent oracle input cases</small></div><div class="card"><span class="number">${escape(report.successful_mock_executions)}</span><small>Oracle-matched mocked executions</small></div><div class="card"><span class="number">${escape(report.operation_width_families)}</span><small>Operation / width families</small></div><div class="card"><span class="number">${escape(report.variants.length)}</span><small>Solidity implementations compared</small></div></div>
<h2>Observed mocked operation gas</h2><p>Each table cell is a median from the same deterministic sample IDs. All numbers are <strong>mock transaction gas units</strong>, not an estimate of on-chain CoFHE task fees.</p>
<div class="table"><table><thead><tr><th scope="col">Operation / width</th>${head}</tr></thead><tbody>${rows}</tbody></table></div>
<h2>Verification fingerprints</h2><p>Corpus SHA-256: <code>${escape(report.corpus_sha256)}</code></p><p>Recorded evidence SHA-256: <code>${escape(report.raw_evidence_sha256)}</code></p><p>Compiled source-seal file SHA-256: <code>${escape(report.source_seal_file_sha256)}</code></p>
<h2>Limits and next evidence gate</h2><ul>${caveats}</ul><p>The proper next funding-grade experiment is a separately authorized and independently audited live Phase 8/9 run. The offline dossier does not assert that such a run has happened.</p>
<p class="foot">Reproduce via the Phase 11 evidence verifier and the accompanying canonical sample corpus; data and source hashes are provided in the bundle.</p></body></html>\n`;
}

export function main(args) {
  if (args.length !== 4 || args[0] !== "--validated" || args[2] !== "--out") throw new Error("Usage: --validated <report.json> --out <new-report.html>");
  const report = JSON.parse(readFileSync(args[1], "utf8").replace(/^\uFEFF/, ""));
  writeFileSync(args[3], render(report), { flag: "wx" });
  process.stdout.write("Phase 11 offline-only evidence report generated; no mock costs misrepresented as live gas.\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 11 HTML refused input: " + error.message + "\n"); process.exitCode = 1; }
}
