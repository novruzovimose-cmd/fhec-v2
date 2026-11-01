import { writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { digest, exact, jsonFile, object, options, requireNewPath } from "./strict.mjs";

export function assemble(local, receipt = null) {
  object(local, "local attestation");
  exact(local.schema, "fhec-cofhe-phase5-local-attestation-v1", "local evidence schema");
  exact(local.tier, "verified-local-mock-only", "local evidence tier");
  if (local.cryptographic_claims?.live_fhe_proven !== false ||
      local.cryptographic_claims?.public_network_fhe_proven !== false ||
      local.cryptographic_claims?.production_security_audited !== false) {
    throw new Error("Local attestation cannot claim live/production cryptographic verification");
  }
  let verifiedNetwork = null;
  if (receipt) {
    object(receipt, "network receipt attestation");
    exact(receipt.schema, "fhec-phase5-readonly-receipt-evidence-v1", "receipt evidence schema");
    exact(receipt.tier, "read-only-public-chain-receipts", "receipt evidence tier");
    exact(receipt.cryptographic_claims?.chain_transactions_verified, true, "on-chain receipt result");
    exact(receipt.cryptographic_claims?.ciphertext_proof_verified, false, "ciphertext proof status");
    exact(receipt.cryptographic_claims?.fhe_coprocessor_execution_verified, false, "FHE execution status");
    exact(receipt.cryptographic_claims?.decrypted_outputs_verified, false, "decrypted outputs status");
    verifiedNetwork = {
      chain_id: receipt.chain_id,
      target_contract: receipt.target_contract,
      verified_transaction_count: receipt.verified_transactions?.length,
      contract_code_sha256: receipt.contract_code_sha256,
    };
  }
  return {
    schema: "fhec-cofhe-phase5-grant-evidence-v1",
    overall_evidence_tier: verifiedNetwork ? "local-mocks-plus-readonly-chain-receipts" : "local-mock-verification",
    completed_mock_comparisons: local.completed_mock_comparisons,
    function_count: local.function_count,
    onchain: verifiedNetwork,
    claim_boundary: {
      independent_local_differential_oracle: true,
      hash_integrity_of_local_generated_outputs: true,
      live_fhe_execution_evidence: false,
      fhe_result_decryption_evidence: false,
      measured_real_fhe_coprocessor_fees: false,
      external_security_audit: false,
    },
    application_guidance: [
      "Attach Phase 4 HTML report clearly identified as local Hardhat mock evidence.",
      "Attach the SHA-256 manifest, source reference, independent manual Solidity baseline and test methodology.",
      "Do not call transaction inclusion evidence a complete FHE end-to-end demonstration.",
      "Obtain wallet-authorized public testnet ciphertext proofs, processing completion, explicit ACL, and decrypted result validations before making live claims.",
      "Keep external funding requests as milestone proposals, not promised awards.",
    ],
  };
}

export function main(args) {
  if (args.length !== 4 && args.length !== 6) {
    throw new Error("Usage: node evidence-bundle.mjs --local <attestation.json> [--receipts <receipts.json>] --out <evidence.json>");
  }
  const mapping = { "--local": "local", "--out": "out" };
  if (args.length === 6) mapping["--receipts"] = "receipts";
  const parsed = options(args, mapping);
  const local = jsonFile(parsed.local, "local attestation");
  const receipts = parsed.receipts ? jsonFile(parsed.receipts, "receipt attestation") : null;
  const bundled = assemble(local, receipts);
  writeFileSync(requireNewPath(parsed.out), `${JSON.stringify(bundled, null, 2)}\n`, { flag: "wx" });
  process.stdout.write(`Wrote ${bundled.overall_evidence_tier} grant evidence.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write(`Phase 5 bundling failed: ${error.message}\n`); process.exitCode = 1; }
}
