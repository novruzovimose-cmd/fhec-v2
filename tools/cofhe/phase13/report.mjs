import { grantBudget } from "./gates.mjs";

export function escapeHtml(value) {
  return String(value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

export function reportMarkdown(gates, local, live = null, ci = null) {
  const budget = grantBudget();
  const lines = [
    "# FHEC V2 CoFHE — Phase 13 external-evidence summary",
    "",
    "This is a review packet for an illustrative $20,000 milestone grant request. It does not demonstrate an awarded grant, full CoFHE parity, or production security.",
    "",
    `Local qualification: ${gates.local_qualification}. Phase 12 candidate: ${local.candidate_status}.`,
    `Original unchanged source files: ${local.phase12_recorded_source_files_unchanged}.`,
    `Original source inventory digest: ${local.phase12_source_inventory_tree_sha256}.`,
    "",
    "## External gates",
    "",
    "| Gate | Status | Evidence |",
    "| --- | --- | --- |",
    ...gates.gates.map((gate) => `| ${gate.id} | ${gate.status} | ${gate.proof ?? "Not provided or not independently verified"} |`),
    "",
    `Corroborated evidence gates: ${gates.external_gates_corroborated} of four. The separate grant decision is outside software control.`,
    "",
    "## Measured and unmeasured costs",
    "",
    live === null ? "No live transactions or network fees are demonstrated by this report." :
      `Observed native testnet transaction fees: ${live.consensus.native_chain_fees_paid_wei} wei over ${live.consensus.transaction_count} confirmed receipts. This is **not** a CoFHE coprocessor fee measurement.`,
    "Coprocessor service prices, sponsorships, future network prices, and mainnet costs remain unverified.",
    "",
    "## Public reproducibility",
    "",
    ci === null ? "No independently verified hosted CI run was supplied." :
      `GitHub Actions: ${ci.run_url}; revision: ${ci.head_sha}; operating systems: ${ci.verified_jobs.map((job) => job.os).join(", ")}.`,
    "The reporter checks public GitHub API status; reviewer still needs to establish the relationship between the public commit and the distributed release.",
    "",
    "## Suggested funding milestones",
    "",
    "| Milestone | Proposed USD | Acceptance evidence |",
    "| --- | ---: | --- |",
    ...budget.milestones.map((item) => `| ${item.title} | $${item.usd.toLocaleString("en-US")} | ${item.proof} |`),
    "",
    `**Total requested: $${budget.requested.toLocaleString("en-US")}. Awarded: not verified.**`,
    "",
    "## Release limitations",
    "",
    ...gates.warnings.map((warning) => `- ${warning}`),
    "",
    "## Recommended independent follow-up",
    "",
    "1. Request independent review of all source generation, ciphertext proof lifecycle and access control boundaries.",
    "2. Record provider-verified CoFHE task or coprocessor fees separately from native receipts.",
    "3. Match the published SHA and source inventory to the actual reviewed release.",
    "4. Provide a 90-second demonstration, reproduction commands, API support matrix, open issues and usage examples.",
    "5. Confirm eligibility and funding terms with Fhenix. Do not imply that the Fellowship guarantees a grant.",
    "",
  ];
  return lines.join("\n");
}

export function reportHtml(gates, local, live = null, ci = null) {
  const rows = gates.gates.map((gate) => `<tr><th>${escapeHtml(gate.id)}</th><td>${escapeHtml(gate.status)}</td><td>${escapeHtml(gate.proof ?? "Pending")}</td></tr>`).join("\n");
  const budget = grantBudget();
  const milestones = budget.milestones.map((item) => `<tr><td>${escapeHtml(item.id)}</td><td>${escapeHtml(item.title)}</td><td>$${item.usd.toLocaleString("en-US")}</td><td>${escapeHtml(item.proof)}</td></tr>`).join("\n");
  const liveText = live ? `${live.consensus.transaction_count} read-only corroborated testnet receipts, ${live.consensus.native_chain_fees_paid_wei} wei native fees` : "No live testnet evidence supplied";
  const ciText = ci ? `${ci.repository}@${ci.head_sha} — <a rel="noopener noreferrer" href="${escapeHtml(ci.run_url)}">hosted run</a>` : "No public hosted CI evidence supplied";
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>FHEC CoFHE Phase 13 Evidence</title><style>body{font:16px/1.55 system-ui,sans-serif;max-width:1050px;margin:35px auto;padding:0 22px;background:#101827;color:#e9eef6}header,section{background:#1b293e;border:1px solid #364c69;border-radius:14px;padding:22px;margin:18px 0}h1{font-size:26px}h2{font-size:21px}table{border-collapse:collapse;width:100%}th,td{padding:10px;text-align:left;border-bottom:1px solid #40526a;vertical-align:top}a{color:#9bccff}.warn{color:#ffd18f}code{overflow-wrap:anywhere}small{color:#b2c0d7}</style></head><body><header><h1>FHEC V2 — CoFHE Phase 13 external evidence</h1><p>Local candidate validated, external evidence recorded only when independently checked.</p><p class="warn"><strong>Not production-certified. No grant award verified. Full CoFHE API coverage unproven.</strong></p></header><section><h2>Source identity</h2><p>Phase 12 recorded files unchanged: ${local.phase12_recorded_source_files_unchanged}</p><p>SHA-256 source inventory: <code>${escapeHtml(local.phase12_source_inventory_tree_sha256)}</code></p></section><section><h2>External gates</h2><table><thead><tr><th>Gate</th><th>Result</th><th>Evidence</th></tr></thead><tbody>${rows}</tbody></table><p>${gates.external_gates_corroborated} of four external technical gates corroborated. Award decision is separate.</p></section><section><h2>Observed scope</h2><p>${escapeHtml(liveText)}</p><p>${ciText}</p><p class="warn">Coprocessor fees and independent security review remain unverified.</p></section><section><h2>Proposed $20,000 milestones</h2><table><thead><tr><th>ID</th><th>Deliverable</th><th>Proposed funding</th><th>Acceptance</th></tr></thead><tbody>${milestones}</tbody></table><p><strong>$20,000 requested; no grant award verified.</strong></p></section><section><h2>Limitations</h2><ul>${gates.warnings.map((s) => `<li>${escapeHtml(s)}</li>`).join("")}</ul><small>Offline evidence artifact generated by Phase 13. Do not replace third-party signatures or fee invoices with local status files.</small></section></body></html>\n`;
}
