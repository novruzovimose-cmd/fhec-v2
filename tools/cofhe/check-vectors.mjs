import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const input = resolve(process.argv[2] ?? "integration/cofhe-hardhat/testdata/cofhe-vectors.jsonl");
const data = await readFile(input, "utf8");
const entries = data.trimEnd().split("\n");
const known = new Set();
const counts = new Map();

for (const raw of entries) {
  const item = JSON.parse(raw);
  if (item.schema !== "fhec-cofhe-test-vector-v1") throw new Error(`Schema mismatch ${item.id}`);
  if (known.has(item.id)) throw new Error(`Duplicate test vector ${item.id}`);
  known.add(item.id);
  const width = BigInt(item.width);
  if (![8n, 16n, 32n, 64n].includes(width)) throw new Error(`Invalid width ${item.id}`);
  const mask = (1n << width) - 1n;
  const a = BigInt(item.a);
  const b = BigInt(item.b);
  if (a < 0n || b < 0n || a > mask || b > mask) throw new Error(`Invalid input range ${item.id}`);

  let computed;
  switch (item.operation) {
    case "add": computed = ((a + b) & mask).toString(); break;
    case "sub": computed = ((a - b) & mask).toString(); break;
    case "mul": computed = ((a * b) & mask).toString(); break;
    case "repeated": computed = ((2n * a * b) & mask).toString(); break;
    case "eq": computed = a === b; break;
    case "ne": computed = a !== b; break;
    case "gt": computed = a > b; break;
    case "ge": computed = a >= b; break;
    case "lt": computed = a < b; break;
    case "le": computed = a <= b; break;
    default: throw new Error(`Unknown operation ${item.id}`);
  }
  if (computed !== item.expected) throw new Error(`Oracle mismatch ${item.id}`);
  const label = `u${item.width}:${item.operation}`;
  counts.set(label, (counts.get(label) ?? 0) + 1);
}

if (entries.length !== 9472) {
  throw new Error(`Unexpected corpus size ${entries.length}`);
}
for (const [label, count] of counts) {
  if (count !== 256) throw new Error(`Incomplete corpus partition ${label} count=${count}`);
}
const sha256 = createHash("sha256").update(data).digest("hex");
process.stdout.write(`VALIDATED ${entries.length} vectors partitions=${counts.size} sha256=${sha256}\n`);
