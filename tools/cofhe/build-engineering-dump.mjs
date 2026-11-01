import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(process.argv[2] ?? ".");
const summary = await readFile(resolve(root, "docs/cofhe-phase3-engineering-plan.md"), "utf8");
const vectors = (await readFile(resolve(root, "integration/cofhe-hardhat/testdata/cofhe-vectors.jsonl"), "utf8"))
  .trimEnd()
  .split("\n")
  .map((line) => JSON.parse(line));

if (vectors.length !== 9472) throw new Error("The acceptance corpus is incomplete");

const appendix = [
  "",
  "## Appendix: sampled, machine-checkable acceptance ledger",
  "",
  "This appendix contains every eighth row of the pinned full corpus. These are proposed and existing regression vectors, not proof that all 9,472 rows have executed under Fhenix mocks or on a live network. The source JSONL supplies the full set. Each row supplies a future encrypted test case with an independent unsigned mathematical oracle. Every `pending` entry requires its corresponding width/operator adapter and execution verification before a passing claim is appropriate.",
  "",
  "| Scenario | Width | Encrypted operation | Left operand | Right operand | Oracle output | Execution status |",
  "|---|---:|---|---:|---:|---:|---|",
];

vectors.forEach((vector, index) => {
  if (index % 8 !== 0) return;
  appendix.push(`| ${vector.id} | ${vector.width} | ${vector.operation} | ${vector.a} | ${vector.b} | ${vector.expected} | pending per-vector live FHE check |`);
});

const closing = [
  "",
  "## Completion records the next patch must produce",
  "",
  "- A Rust backend test report containing all direct and transitive call-graph checks.",
  "- A pinned corpus checksum and a successful independent recomputation of all mathematical oracles.",
  "- A transcript of the three-way local CoFHE mock run with per-case values and gas labels.",
  "- A generation record linking FHEC input source to baseline and optimized Solidity SHA-256 values.",
  "- A statement explicitly distinguishing current local mocks from future testnet execution.",
  "- A future live-network result with chain ID, transaction hashes, deployment addresses, and exact gas provenance.",
  "- A future test matrix for u16/u32/u64 widths, all corpus operators, ACL negative paths, signer binding, and SDK compatibility.",
  "- A future public review packet whose content exposes no private source code or wallet secrets.",
  "",
];

const output = `${summary}\n${appendix.join("\n")}\n${closing.join("\n")}`;
if (Buffer.byteLength(output) < 100_000) throw new Error("Engineering dump is smaller than 100 KB");
const destination = resolve(root, "FHEC_V2_NEXT_MAJOR_STEPS_100KB.md");
await writeFile(destination, output, { encoding: "utf8", flag: "w" });
process.stdout.write(`${destination} ${Buffer.byteLength(output)} bytes\n`);
