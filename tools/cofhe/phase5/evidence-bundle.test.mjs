import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { assemble } from "./evidence-bundle.mjs";

const local = () => ({
  schema: "fhec-cofhe-phase5-local-attestation-v1",
  tier: "verified-local-mock-only",
  completed_mock_comparisons: 48,
  function_count: 26,
  cryptographic_claims: { live_fhe_proven: false, public_network_fhe_proven: false, production_security_audited: false },
});
const receipts = () => ({
  schema: "fhec-phase5-readonly-receipt-evidence-v1",
  tier: "read-only-public-chain-receipts",
  chain_id: 421614,
  target_contract: `0x${"1".repeat(40)}`,
  contract_code_sha256: "A".repeat(64),
  verified_transactions: [{ transaction_hash: `0x${"a".repeat(64)}` }],
  cryptographic_claims: {
    chain_transactions_verified: true,
    ciphertext_proof_verified: false,
    fhe_coprocessor_execution_verified: false,
    decrypted_outputs_verified: false,
  },
});

describe("Phase 5 funding evidence boundaries", () => {
  it("local-only report never elevates to live FHE", () => {
    const result = assemble(local());
    assert.equal(result.overall_evidence_tier, "local-mock-verification");
    assert.equal(result.claim_boundary.live_fhe_execution_evidence, false);
  });
  it("receipt verification cannot elevate to coprocessor proof", () => {
    const result = assemble(local(), receipts());
    assert.equal(result.overall_evidence_tier, "local-mocks-plus-readonly-chain-receipts");
    assert.equal(result.claim_boundary.fhe_result_decryption_evidence, false);
    assert.equal(result.claim_boundary.measured_real_fhe_coprocessor_fees, false);
  });
  it("rejects false local assertions", () => {
    const modified = local();
    modified.cryptographic_claims.live_fhe_proven = true;
    assert.throws(() => assemble(modified), /cannot claim/);
  });
  it("rejects elevated receipt assertions", () => {
    const modified = receipts();
    modified.cryptographic_claims.fhe_coprocessor_execution_verified = true;
    assert.throws(() => assemble(local(), modified), /FHE execution status/);
  });
});
