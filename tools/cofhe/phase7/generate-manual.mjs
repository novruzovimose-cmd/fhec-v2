import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { widths } from "../phase6/oracle.mjs";
import { generateSolidity } from "../phase6/generate-solidity.mjs";

const families = ["add", "sub", "mul", "gt", "max", "repeated"];
const variants = [
  { name: "optimized", library: "FHECPhase6Optimized", harness: "FHECPhase6Live" },
  { name: "straight", library: "FHECPhase7ManualStraight", harness: "FHECPhase7Straight" },
  { name: "tuned", library: "FHECPhase7ManualTuned", harness: "FHECPhase7Tuned" },
];

export { families, variants };

function functionText(width, operation, tuned) {
  const kind = "euint" + width;
  const ret = operation === "gt" ? "ebool" : kind;
  let body;
  if (["add", "sub", "mul", "gt"].includes(operation)) {
    body = "return FHE." + operation + "(a, b);";
  } else if (operation === "max") {
    body = "return FHE.select(FHE.gt(a, b), a, b);";
  } else if (tuned) {
    body = kind + " product = FHE.mul(a, b);\n        return FHE.add(product, product);";
  } else {
    body = "return FHE.add(FHE.mul(a, b), FHE.mul(a, b));";
  }
  return [
    "    function " + operation + "_u" + width + "(" + kind + " a, " + kind + " b) internal returns (" + ret + ") {",
    "        " + body,
    "    }",
  ].join("\n");
}

export function generateLibrary(name, tuned) {
  if (!["FHECPhase7ManualStraight", "FHECPhase7ManualTuned"].includes(name)) {
    throw new Error("Unrecognized reference library");
  }
  const imports = widths.map((width) => "euint" + width);
  const functions = widths.flatMap((width) => families.map((operation) => functionText(width, operation, tuned)));
  return [
    "// SPDX-License-Identifier: Apache-2.0",
    "pragma solidity ^0.8.25;",
    "",
    'import {FHE, ebool, ' + imports.join(", ") + '} from "@fhenixprotocol/cofhe-contracts/FHE.sol";',
    "",
    "library " + name + " {",
    functions.join("\n\n"),
    "}",
    "",
  ].join("\n");
}

export function generateHarness(library, contractName) {
  if (!variants.some((variant) => variant.library === library && variant.harness === contractName)) {
    throw new Error("Unrecognized harness/library pairing");
  }
  return generateSolidity()
    .replaceAll("FHECPhase6Optimized", library)
    .replaceAll("FHECPhase6Live", contractName);
}

export function generateArtifacts() {
  const result = new Map();
  result.set("FHECPhase6Live.sol", generateHarness("FHECPhase6Optimized", "FHECPhase6Live"));
  for (const variant of variants.slice(1)) {
    result.set(variant.library + ".sol", generateLibrary(variant.library, variant.name === "tuned"));
    result.set(variant.harness + ".sol", generateHarness(variant.library, variant.harness));
  }
  return result;
}

export function main(args) {
  if (args.length !== 2 || args[0] !== "--directory") {
    throw new Error("Usage: node generate-manual.mjs --directory <existing-empty-directory>");
  }
  const destination = resolve(args[1]);
  for (const [fileName, content] of generateArtifacts()) {
    writeFileSync(resolve(destination, fileName), content, { flag: "wx" });
  }
  process.stdout.write("Phase 7: wrote two independent Solidity reference libraries and three equivalent harnesses.\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write("Phase 7 generation failed: " + error.message + "\n"); process.exitCode = 1; }
}
