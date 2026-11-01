export function escapeHtml(value) {
  return String(value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

export function renderQualification(qualification, inventory) {
  const rows = qualification.full_width_coverage.matrix.map((item) => `<tr><td>${escapeHtml(item.family)}</td><td>${item.cases}</td><td>${item.mock_executions}</td><td>${escapeHtml(item.variants.join(", "))}</td></tr>`).join("\n");
  const local = qualification.verified_local_gates.map((value) => `<li><strong>Verified locally:</strong> ${escapeHtml(value)}</li>`).join("\n");
  const pending = qualification.pending_external_gates.map((value) => `<li><strong>Requires external evidence:</strong> ${escapeHtml(value)}</li>`).join("\n");
  const limitations = qualification.essential_limitations.map((value) => `<li>${escapeHtml(value)}</li>`).join("\n");
  return `<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>FHEC CoFHE Phase 12 Local Release Candidate</title>
<style>body{max-width:1100px;margin:40px auto;padding:0 20px;background:#f5f8fc;color:#18243b;font:16px/1.6 system-ui}header,section{background:#fff;padding:24px 30px;border-radius:12px;margin-bottom:16px;box-shadow:0 2px 12px #12204414}h1{margin:0}h2{margin:0 0 10px}.pass{color:#166534}.warning{color:#92400e}.badge{display:inline-block;padding:5px 11px;border-radius:99px;background:#fff0c2;font-weight:bold}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:12px}.metric{background:#ecfdf5;padding:14px;border-radius:8px}.metric strong{font-size:27px;display:block}table{border-collapse:collapse;width:100%;font-size:14px}th,td{text-align:left;border-bottom:1px solid #e1e7ef;padding:7px}th{background:#eef2fb}code{overflow-wrap:anywhere}.notice{border-left:5px solid #c2410c;padding:14px;background:#fff7ed}</style></head>
<body><header><span class="badge">Local qualification only</span><h1>FHEC V2 CoFHE — Phase 12</h1><p>Deterministic source inventory and independently replayed, source-sealed CoFHE mock evidence. No transaction was submitted by this qualification.</p><p class="notice">This is <strong>not</strong> a production security certification, real testnet benchmark or grant award.</p></header>
<section><h2>Verified local scope</h2><div class="grid"><div class="metric"><strong>${qualification.full_width_coverage.operation_families}</strong>operation/width families</div><div class="metric"><strong>${qualification.full_width_coverage.corpus_cases}</strong>oracle test vectors</div><div class="metric"><strong>${qualification.full_width_coverage.verified_mock_executions}</strong>mock ciphertext executions</div><div class="metric"><strong>${inventory.source_files}</strong>selected source files fingerprinted</div></div></section>
<section><h2>Evidence gates</h2><ul>${local}\n${pending}<li><strong>Outside qualification scope:</strong> GRANT_REVIEW</li></ul></section>
<section><h2>Covered matrix</h2><table><thead><tr><th>Family</th><th>Vectors</th><th>Mock executions</th><th>Implementations</th></tr></thead><tbody>${rows}</tbody></table></section>
<section><h2>Reproduction anchors</h2><p>Reviewed Solidity seal: <code>${escapeHtml(qualification.source_seal_sha256)}</code></p><p>Phase 11 evidence entries: <code>${escapeHtml(qualification.phase11_evidence_sha256)}</code></p><p>Selected source tree: <code>${escapeHtml(inventory.tree_sha256)}</code></p><p>This static report is verified byte-for-byte by the Phase 12 candidate verifier.</p></section>
<section><h2>Limitations and funding status</h2><ul>${limitations}</ul></section></body></html>\n`;
}
