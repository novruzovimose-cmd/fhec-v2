export function handoffMarkdown(qualification, inventory) {
  return `# FHEC V2 CoFHE — Phase 12 Local Release Candidate

Status: **qualified local release candidate only**. Neither production certified nor grant awarded.

## Reproduction anchors

- Reviewed generated Solidity source seal: \`${qualification.source_seal_sha256}\`
- Independently revalidated Phase 11 evidence entries: \`${qualification.phase11_evidence_sha256}\`
- SHA-256 of selected source file listing: \`${inventory.tree_sha256}\`
- Selected source files: ${inventory.source_files} (${inventory.source_bytes} bytes).
- Corpus vectors: ${qualification.full_width_coverage.corpus_cases}.
- Independent local mock executions: ${qualification.full_width_coverage.verified_mock_executions}.
- Families: ${qualification.full_width_coverage.operation_families}; variants: ${qualification.full_width_coverage.variants.join(", ")}.

## What this capsule proves locally

The Phase 11 manifest, source seal, independently recomputed BigInt arithmetic oracle, ciphertext mock observation summary, three-way differential coverage, and offline report survived read-only independent replay. Phase 12 also checks its own exact file list and a source inventory. The candidate is deliberately not signed or published.

## Explicitly outstanding evidence

${qualification.pending_external_gates.map((value) => `- ${value}`).join("\n")}

## Funding / release constraints

The desired grant is $20,000. This file records technical readiness evidence, not a promise of funding or eligibility. Security review, network-side encrypted task costs, fresh hosted CI, and live real-chain records are separately required. An internal hash manifest is not an authenticated third-party signature.

## Independent offline replay

From the FHEC V2 repository root, with the Phase 12 scripts installed:

\`\`\`powershell
cd "$env:USERPROFILE\\Documents\\fhec-v2"
node tools/cofhe/phase12/cli.mjs verify --candidate "<path-to-candidate>"
\`\`\`
`;
}
