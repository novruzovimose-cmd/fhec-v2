import test from "node:test";
import assert from "node:assert/strict";
import { compareRpcAttestations } from "./chain.mjs";

const receipt = {
  kind: "execute",
  transaction_hash: "0x" + "a".repeat(64),
  block_hash: "0x" + "b".repeat(64),
  block_number: 123,
  confirmations: "10",
  native_chain_fee_wei: "100",
  calldata_sha256: "C".repeat(64),
};

const record = {
  schema: "fhec-cofhe-phase89-rpc-attestation-v1",
  chain_id: 421614,
  source_seal_sha256: "A".repeat(64),
  plan_sha256: "D".repeat(64),
  raw_trace_sha256: "E".repeat(64),
  native_chain_fees_paid_wei: "100",
  verified_transaction_count: 1,
  deployment_count: 0,
  execution_count: 1,
  minimum_confirmations: 2,
  coprocessor_fees_measured: false,
  receipts: [receipt],
};

test("two providers agree even if current confirmation depths differ", () => {
  const result = compareRpcAttestations(record, { ...record, receipts: [{ ...receipt, confirmations: "12" }] });
  assert.equal(result.independent_rpc_agreement, true);
  assert.equal(result.transaction_count, 1);
  assert.equal(result.coprocessor_fees_measured, false);
});

test("two providers reject changed block, gas fee, source or absent receipt", () => {
  assert.throws(() => compareRpcAttestations(record, { ...record, receipts: [{ ...receipt, block_hash: "0x" + "1".repeat(64) }] }));
  assert.throws(() => compareRpcAttestations(record, { ...record, receipts: [{ ...receipt, native_chain_fee_wei: "101" }] }));
  assert.throws(() => compareRpcAttestations(record, { ...record, source_seal_sha256: "1".repeat(64) }));
  assert.throws(() => compareRpcAttestations(record, { ...record, receipts: [] }));
});

test("two providers reject invented measurement of off-chain FHE task fees", () => {
  assert.throws(() => compareRpcAttestations(record, { ...record, coprocessor_fees_measured: true }));
});
