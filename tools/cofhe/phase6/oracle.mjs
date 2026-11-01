import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { writeFileSync } from "node:fs";

export const widths = Object.freeze([8, 16, 32, 64]);
export const operations = Object.freeze(["add", "sub", "mul", "gt", "max", "repeated"]);

export function maxUnsigned(width) {
  if (!widths.includes(width)) throw new Error(`Unsupported width ${width}`);
  return (1n << BigInt(width)) - 1n;
}

export function expected(width, operation, left, right) {
  const max = maxUnsigned(width);
  if (!operations.includes(operation)) throw new Error(`Unknown operation ${operation}`);
  if (typeof left !== "bigint" || typeof right !== "bigint" || left < 0n || right < 0n || left > max || right > max) {
    throw new Error("Unsigned encrypted operand is outside its declared width");
  }
  switch (operation) {
    case "add": return (left + right) & max;
    case "sub": return (left - right) & max;
    case "mul": return (left * right) & max;
    case "gt": return left > right;
    case "max": return left > right ? left : right;
    case "repeated": return (left * right * 2n) & max;
  }
}

export function boundaryPairs(width) {
  const max = maxUnsigned(width);
  const half = 1n << (BigInt(width) - 1n);
  return [
    [0n, 0n],
    [0n, max],
    [max, 0n],
    [max, max],
    [1n, max],
    [max, 1n],
    [max - 1n, 2n],
    [half, half - 1n],
    [half - 1n, half],
    [half, half],
    [1n, 0n],
    [0n, 1n],
    [max - 2n, 3n],
    [max, 2n],
    [2n, max],
    [2n, 2n],
  ];
}

export function deterministicPairs(width, count, seed = 0x5A17n) {
  const max = maxUnsigned(width);
  if (!Number.isSafeInteger(count) || count < 0 || count > 1024) throw new Error("Invalid pair count");
  const result = [...boundaryPairs(width)];
  let state = (seed ^ BigInt(width)) & ((1n << 64n) - 1n);
  for (let n = 0; n < count; n += 1) {
    state = (state * 6364136223846793005n + 1442695040888963407n) & ((1n << 64n) - 1n);
    const left = state & max;
    state = (state * 6364136223846793005n + 1442695040888963407n) & ((1n << 64n) - 1n);
    const right = state & max;
    result.push([left, right]);
  }
  return result;
}

export function createMatrix(perFamily = 16, randomPairs = 12) {
  if (!Number.isSafeInteger(perFamily) || perFamily < 1 || perFamily > 28) {
    throw new Error("perFamily must be 1-28");
  }
  if (!Number.isSafeInteger(randomPairs) || randomPairs < 0 || randomPairs > 1024) {
    throw new Error("randomPairs must be 0-1024");
  }
  const cases = [];
  for (const width of widths) {
    const pairs = deterministicPairs(width, randomPairs);
    if (perFamily > pairs.length) throw new Error("Requested cases exceed deterministic corpus size");
    for (const operation of operations) {
      for (let index = 0; index < perFamily; index += 1) {
        const [left, right] = pairs[index];
        const answer = expected(width, operation, left, right);
        cases.push({
          id: `${operation}-u${width}-${String(index).padStart(2, "0")}`,
          width,
          operation,
          a: left.toString(),
          b: right.toString(),
          expected: typeof answer === "boolean" ? answer : answer.toString(),
          boundary: index < 16,
        });
      }
    }
  }
  return { schema: "fhec-cofhe-phase6-boundary-matrix-v1", seed: "0x5A17", cases_per_family: perFamily,
    family_count: widths.length * operations.length, total_cases: cases.length, cases,
    scope: "deterministic plaintext oracle; does not prove encrypted runtime correctness" };
}

export function assertMatrix(matrix) {
  if (!matrix || matrix.schema !== "fhec-cofhe-phase6-boundary-matrix-v1" || !Array.isArray(matrix.cases)) {
    throw new Error("Invalid Phase 6 matrix schema");
  }
  if (!Number.isSafeInteger(matrix.cases_per_family) || matrix.cases_per_family < 1 || matrix.cases_per_family > 28 ||
      matrix.family_count !== 24 || matrix.total_cases !== 24 * matrix.cases_per_family ||
      matrix.cases.length !== matrix.total_cases) throw new Error("Incomplete matrix coverage");
  const expectedMatrix = createMatrix(matrix.cases_per_family);
  if (JSON.stringify(expectedMatrix) !== JSON.stringify(matrix)) {
    throw new Error("Matrix does not match deterministic independent oracle");
  }
  return matrix;
}

export function main(args) {
  if (args.length !== 4 || args[0] !== "--per-family" || args[2] !== "--out") {
    throw new Error("Usage: node oracle.mjs --per-family <1-28> --out <matrix.json>");
  }
  const count = Number(args[1]);
  const matrix = assertMatrix(createMatrix(count));
  writeFileSync(args[3], `${JSON.stringify(matrix, null, 2)}\n`, { flag: "wx" });
  process.stdout.write(`Generated ${matrix.total_cases} deterministic plaintext oracle cases.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write(`Phase 6 oracle error: ${error.message}\n`); process.exitCode = 1; }
}
