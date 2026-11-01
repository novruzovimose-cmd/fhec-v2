export const grantMilestones = Object.freeze([
  { id: "M1", title: "Public reproducibility and supported CoFHE API matrix", usd: 4000, proof: "Reproducible source revision, Windows/Linux CI, compatibility matrix, stable CLI" },
  { id: "M2", title: "Real testnet execution and observed encrypted semantics", usd: 5000, proof: "Operator-authorized Arbitrum Sepolia trace, source-sealed cases, two read-only RPC checks" },
  { id: "M3", title: "Real-world economics and optimization measurement", usd: 4000, proof: "Actual native transaction fees, latency and a separate coprocessor task-fee review" },
  { id: "M4", title: "Developer experience and integration adoption", usd: 4000, proof: "Easy-to-run example applications, SDK contract-bound proof flows, onboarding documentation" },
  { id: "M5", title: "External security scrutiny and remediation", usd: 3000, proof: "Independent review, recorded findings, resolved high-severity issues" },
]);

export function grantBudget() {
  const total = grantMilestones.reduce((sum, milestone) => sum + milestone.usd, 0);
  if (total !== 20000) throw new Error("Reviewed illustrative grant budget no longer totals $20,000");
  return { currency: "USD", requested: total, funding_awarded: null, milestones: grantMilestones };
}

export function evaluateGates(local, live = null, ci = null) {
  if (!local || local.schema !== "fhec-cofhe-phase13-source-preflight-v1" ||
      local.candidate_status !== "qualified-local-release-candidate-only" ||
      local.phase12_source_files_unchanged !== true) throw new Error("A verified Phase 12 capsule and unmodified source anchors are mandatory");
  if (live !== null && (live.consensus?.schema !== "fhec-cofhe-phase13-two-provider-testnet-corroboration-v1" ||
      live.consensus.independent_rpc_agreement !== true || live.consensus.coprocessor_fees_measured !== false ||
      live.consensus.transaction_count !== live.consensus.deployment_count + live.consensus.execution_count ||
      live.analysis?.schema !== "fhec-cofhe-phase9-real-testnet-comparison-v1" ||
      live.analysis.coprocessor_fees_measured !== false ||
      live.analysis.total_native_chain_fees_wei !== live.consensus.native_chain_fees_paid_wei)) {
    throw new Error("Invalid or incomplete two-provider real-network evidence");
  }
  if (ci !== null && (ci.schema !== "fhec-cofhe-phase13-public-github-ci-v1" ||
      ci.evidence_tier !== "github-api-observed-hosted-success" || ci.checked_via_public_api !== true ||
      ci.verified_jobs?.length !== 2 || ci.verified_jobs.some((job) => job.conclusion !== "success"))) {
    throw new Error("Invalid public hosted CI evidence");
  }
  const gates = [
    {
      id: "LIVE_TESTNET",
      status: live === null ? "requires-external-evidence" : "rpc-reverified-on-arbitrum-sepolia",
      proof: live === null ? null : `${live.consensus.transaction_count} transaction receipts cross-checked against two providers`,
    },
    {
      id: "COPROCESSOR_FEES",
      status: "requires-provider-evidence-and-independent-review",
      proof: null,
    },
    {
      id: "HOSTED_CI",
      status: ci === null ? "requires-external-evidence" : "github-public-api-verified-for-specified-commit",
      proof: ci === null ? null : `${ci.repository}@${ci.head_sha}; ${ci.verified_jobs.length} hosted OS jobs`,
    },
    {
      id: "INDEPENDENT_SECURITY_REVIEW",
      status: "requires-external-review-and-remediation",
      proof: null,
    },
    { id: "GRANT_REVIEW", status: "outside-tool-control", proof: null },
  ];
  return {
    schema: "fhec-cofhe-phase13-external-gates-v1",
    local_qualification: "verified",
    scope: "Selected CoFHE compiler behavior and external gate evidence, not complete CoFHE API coverage",
    status: "evaluation-only-not-production-certification",
    production_certified: false,
    grant_awarded: false,
    requested_grant_usd: 20000,
    awarded_grant_usd: null,
    external_gates_corroborated: gates.filter((gate) => gate.status.startsWith("rpc-") || gate.status.startsWith("github-")).length,
    external_gates_total: 4,
    gates,
    warnings: [
      "Passing locally never implies a payment or acceptance by Fhenix or another funder.",
      "Transaction receipts establish native chain gas, not CoFHE coprocessor fees.",
      "Permission, API breadth, failure handling, and production security require separate review.",
      "A successful hosted commit must still be linked to the intended released code by publisher and reviewer.",
      "Signed or authenticated third-party reviews and protocol fee statements cannot be self-created by this tool.",
    ],
  };
}
