import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const output = resolve(process.argv[2] ?? "integration/cofhe-hardhat/testdata/cofhe-vectors.jsonl");
const widths = [8, 16, 32, 64];
const operations = ["add", "sub", "mul", "eq", "ne", "gt", "ge", "lt", "le"];
const fullMask = (1n << 64n) - 1n;
let state = 0x72dca615e90b42f1n;

function next() {
  state ^= (state << 13n) & fullMask;
  state ^= state >> 7n;
  state ^= (state << 17n) & fullMask;
  state &= fullMask;
  return state;
}

function expected(operation, left, right, mask) {
  switch (operation) {
    case "add": return ((left + right) & mask).toString();
    case "sub": return ((left - right) & mask).toString();
    case "mul": return ((left * right) & mask).toString();
    case "repeated": return ((2n * left * right) & mask).toString();
    case "eq": return left === right;
    case "ne": return left !== right;
    case "gt": return left > right;
    case "ge": return left >= right;
    case "lt": return left < right;
    case "le": return left <= right;
    default: throw new Error(`Unsupported operation: ${operation}`);
  }
}

function boundaryValues(mask) {
  const half = (mask + 1n) / 2n;
  return [0n, 1n, 2n, 3n, half - 1n, half, half + 1n, mask - 2n, mask - 1n, mask];
}

const records = [];
for (const width of widths) {
  const mask = (1n << BigInt(width)) - 1n;
  const boundaries = boundaryValues(mask);
  const widthOperations = width === 8 ? [...operations, "repeated"] : operations;
  for (const operation of widthOperations) {
    for (let index = 0; index < 256; index += 1) {
      const left = index < 100
        ? boundaries[Math.floor(index / 10)]
        : (next() & mask);
      const right = index < 100
        ? boundaries[index % 10]
        : (next() & mask);
      records.push(JSON.stringify({
        schema: "fhec-cofhe-test-vector-v1",
        id: `${operation}-u${width}-${String(index).padStart(3, "0")}`,
        operation,
        width,
        a: left.toString(),
        b: right.toString(),
        expected: expected(operation, left, right, mask),
        type: ["eq", "ne", "gt", "ge", "lt", "le"].includes(operation) ? "encrypted<bool>" : `encrypted<u${width}>`,
      }));
    }
  }
}

const bytes = `${records.join("\n")}\n`;
await mkdir(dirname(output), { recursive: true });
await writeFile(output, bytes, { encoding: "utf8", flag: "w" });
const hash = createHash("sha256").update(bytes).digest("hex");
process.stdout.write(`vectors=${records.length} bytes=${Buffer.byteLength(bytes)} sha256=${hash}\n`);
