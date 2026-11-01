import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const widths = Object.freeze([8, 16, 32, 64]);
export const operations = Object.freeze(["add", "sub", "mul", "gt", "max", "repeated"]);
export const defaultSeed = "7a1234ef20ab719c49d50208f471338015f76a9a0be99432bfa778b6af01d5c2";
export const modes = Object.freeze(["optimized-tuned", "three-way"]);

export function digest(value) {
  return createHash("sha256").update(value).digest("hex").toUpperCase();
}

export function reference(width, operation, a, b) {
  if (!widths.includes(width) || !operations.includes(operation)) throw new Error("Unsupported independent CoFHE property type");
  const left = BigInt(a);
  const right = BigInt(b);
  const modulus = 1n << BigInt(width);
  if (left < 0n || right < 0n || left >= modulus || right >= modulus) throw new Error("Unsigned operand outside CoFHE bit width");
  if (operation === "add") return ((left + right) % modulus).toString();
  if (operation === "sub") return ((left - right + modulus) % modulus).toString();
  if (operation === "mul") return ((left * right) % modulus).toString();
  if (operation === "gt") return left > right;
  if (operation === "max") return (left > right ? left : right).toString();
  return ((left * right * 2n) % modulus).toString();
}

function deterministicPair(seed, width, operation, index) {
  const max = (1n << BigInt(width)) - 1n;
  const half = max / 2n;
  const boundaries = [
    [0n, 0n],
    [max, 1n],
    [1n, max],
    [max, max],
    [half + 1n, half],
    [0n, max],
    [1n, 1n],
    [half, half + 1n],
  ];
  if (index < boundaries.length) return boundaries[index];
  const left = BigInt("0x" + digest(`fhec-phase10-left:${seed}:${width}:${operation}:${index}`)) & max;
  const right = BigInt("0x" + digest(`fhec-phase10-right:${seed}:${width}:${operation}:${index}`)) & max;
  return [left, right];
}

export function createCorpus(casesPerFamily = 8, mode = "optimized-tuned", seed = defaultSeed) {
  if (!Number.isSafeInteger(casesPerFamily) || casesPerFamily < 2 || casesPerFamily > 16) {
    throw new Error("Phase 10 accepts 2 to 16 deterministic cases per family");
  }
  if (!modes.includes(mode) || !/^[0-9a-f]{64}$/.test(seed)) {
    throw new Error("Phase 10 requires an approved variant mode and lowercase 64-character hex seed");
  }
  const cases = [];
  for (const width of widths) {
    for (const [selector, operation] of operations.entries()) {
      for (let index = 0; index < casesPerFamily; index += 1) {
        const [a, b] = deterministicPair(seed, width, operation, index);
        cases.push({
          id: `p10-${operation}-u${width}-${String(index).padStart(2, "0")}`,
          width,
          operation,
          operation_selector: selector,
          a: a.toString(),
          b: b.toString(),
          expected: reference(width, operation, a, b),
        });
      }
    }
  }
  return {
    schema: "fhec-cofhe-phase10-independent-oracle-v1",
    oracle: "independent-bigint-unsigned-modular-v1",
    purpose: "Deterministic boundary and pseudorandom differential CoFHE mock tests; never live gas evidence",
    cases_per_family: casesPerFamily,
    families: widths.length * operations.length,
    case_count: cases.length,
    seed,
    variant_mode: mode,
    variants: mode === "three-way" ? ["optimized", "tuned", "straight"] : ["optimized", "tuned"],
    cases,
  };
}

export function validateCorpus(corpus) {
  if (!corpus || !modes.includes(corpus.variant_mode)) throw new Error("Invalid Phase 10 oracle schema");
  const exact = createCorpus(corpus.cases_per_family, corpus.variant_mode, corpus.seed);
  if (JSON.stringify(exact) !== JSON.stringify(corpus)) {
    throw new Error("Corrupted or noncanonical independently generated CoFHE oracle corpus");
  }
  return corpus;
}

export function loadCorpus(path) {
  return validateCorpus(JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, "")));
}

export function corpusDigest(corpus) {
  return digest(JSON.stringify(validateCorpus(corpus)));
}

export function main(args) {
  if (args.length !== 8 || args[0] !== "--cases" || args[2] !== "--mode" || args[4] !== "--seed" || args[6] !== "--out") {
    throw new Error("Usage: --cases 2..16 --mode optimized-tuned|three-way --seed <64-hex> --out <new-corpus.json>");
  }
  const corpus = createCorpus(Number(args[1]), args[3], args[5]);
  writeFileSync(args[7], JSON.stringify(corpus, null, 2) + "\n", { flag: "wx" });
  process.stdout.write(`Independent Phase 10 corpus: ${corpus.case_count} cases, ${corpus.variants.length} contract variants, ${corpusDigest(corpus)}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 10 oracle failed: " + error.message + "\n"); process.exitCode = 1; }
}
