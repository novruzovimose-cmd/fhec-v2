import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const widths = [8, 16, 32, 64];
const variants = [
  { name: "Baseline", library: "FHECPhase4Baseline", key: "baseline" },
  { name: "Optimized", library: "FHECPhase4Optimized", key: "optimized" },
  { name: "Manual", library: "FHECPhase4Manual", key: "manual" },
];
const functions = ["add", "sub", "mul", "gt", "max", "repeated"];

function emitFunction(width, operation) {
  const type = `encrypted<u${width}>`;
  const signature = `fn ${operation}_u${width}(a: ${type}, b: ${type}) -> `;
  switch (operation) {
    case "add":
      return `${signature}${type} {\n    return a + b;\n}`;
    case "sub":
      return `${signature}${type} {\n    return a - b;\n}`;
    case "mul":
      return `${signature}${type} {\n    return a * b;\n}`;
    case "gt":
      return `${signature}encrypted<bool> {\n    return a > b;\n}`;
    case "max":
      return `${signature}${type} {\n    return if a > b { a } else { b };\n}`;
    case "repeated":
      return `${signature}${type} {\n    let left = a * b;\n    let right = a * b;\n    return left + right;\n}`;
    default:
      throw new Error(`Unknown operation ${operation}`);
  }
}

export function sourceText() {
  const parts = [];
  for (const width of widths) {
    for (const operation of functions) {
      parts.push(emitFunction(width, operation));
    }
  }
  parts.push("fn twice_u8(a: encrypted<u8>) -> encrypted<u8> {\n    return a + a;\n}");
  parts.push("fn four_times_u8(a: encrypted<u8>) -> encrypted<u8> {\n    let doubled = twice_u8(a);\n    return twice_u8(doubled);\n}");
  return `${parts.join("\n\n")}\n`;
}

function manualMethod(width, operation) {
  const type = `euint${width}`;
  const signature = `    function ${operation}_u${width}(${type} a, ${type} b) internal returns (`;
  switch (operation) {
    case "add":
    case "sub":
    case "mul":
    case "gt":
      return `${signature}${operation === "gt" ? "ebool" : type}) {\n        return FHE.${operation}(a, b);\n    }`;
    case "max":
      return `${signature}${type}) {\n        ebool chooseLeft = FHE.gt(a, b);\n        return FHE.select(chooseLeft, a, b);\n    }`;
    case "repeated":
      return `${signature}${type}) {\n        ${type} product = FHE.mul(a, b);\n        return FHE.add(product, product);\n    }`;
    default:
      throw new Error(`Unknown operation ${operation}`);
  }
}

export function manualText() {
  const result = [
    "// SPDX-License-Identifier: Apache-2.0",
    "pragma solidity ^0.8.25;",
    "",
    `import {FHE, ebool, ${widths.map((width) => `euint${width}`).join(", ")}} from "@fhenixprotocol/cofhe-contracts/FHE.sol";`,
    "",
    "library FHECPhase4Manual {",
  ];
  for (const width of widths) {
    for (const operation of functions) {
      result.push(manualMethod(width, operation), "");
    }
  }
  result.push("}", "");
  return result.join("\n");
}

function runMethod(width, variant) {
  const type = `euint${width}`;
  const ext = `externalEuint${width}`;
  const name = variant.name;
  const lib = variant.library;
  const output = `${variant.key}Value${width}`;
  const flag = `${variant.key}Bool${width}`;
  return [
    `    function run${name}U${width}(uint8 operation, ${ext} encryptedA, bytes calldata proofA, ${ext} encryptedB, bytes calldata proofB) external {`,
    "        require(operation < 6, \"invalid operation\");",
    `        ${type} a = FHE.asEuint${width}(encryptedA, proofA);`,
    `        ${type} b = FHE.asEuint${width}(encryptedB, proofB);`,
    "        if (operation == 3) {",
    `            ebool compared = ${lib}.gt_u${width}(a, b);`,
    `            ${flag} = compared;`,
    "            FHE.allowThis(compared);",
    "            FHE.allowSender(compared);",
    "            return;",
    "        }",
    `        ${type} result;`,
    `        if (operation == 0) result = ${lib}.add_u${width}(a, b);`,
    `        else if (operation == 1) result = ${lib}.sub_u${width}(a, b);`,
    `        else if (operation == 2) result = ${lib}.mul_u${width}(a, b);`,
    `        else if (operation == 4) result = ${lib}.max_u${width}(a, b);`,
    `        else result = ${lib}.repeated_u${width}(a, b);`,
    `        ${output} = result;`,
    "        FHE.allowThis(result);",
    "        FHE.allowSender(result);",
    "    }",
  ].join("\n");
}

export function wrapperText() {
  const result = [
    "// SPDX-License-Identifier: Apache-2.0",
    "pragma solidity ^0.8.25;",
    "",
    `import {FHE, ebool, ${widths.map((width) => `euint${width}, externalEuint${width}`).join(", ")}} from "@fhenixprotocol/cofhe-contracts/FHE.sol";`,
    ...variants.map(({ library }) => `import {${library}} from "./${library}.sol";`),
    "",
    "contract FHECPhase4Comparison {",
  ];
  for (const width of widths) {
    for (const variant of variants) {
      result.push(`    euint${width} public ${variant.key}Value${width};`);
      result.push(`    ebool public ${variant.key}Bool${width};`);
    }
  }
  result.push("");
  for (const width of widths) {
    for (const variant of variants) {
      result.push(runMethod(width, variant), "");
    }
  }
  result.push("}", "");
  return result.join("\n");
}

const direct = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (direct) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
  const hardhat = join(root, "integration/cofhe-hardhat/phase4");
  const files = [
    [join(root, "examples/cofhe_phase4.fhe"), sourceText()],
    [join(hardhat, "FHECPhase4Manual.sol"), manualText()],
    [join(hardhat, "FHECPhase4Comparison.sol"), wrapperText()],
  ];
  if (process.argv.length !== 3 || !["--check", "--create"].includes(process.argv[2])) {
    throw new Error("Usage: node tools/cofhe/generate-phase4-artifacts.mjs --check|--create");
  }
  for (const [path, expected] of files) {
    if (process.argv[2] === "--check") {
      if (readFileSync(path, "utf8") !== expected) {
        throw new Error(`Phase 4 generated source mismatch: ${path}`);
      }
    } else {
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, expected, { flag: "wx" });
    }
  }
  console.log(`Phase 4 artifacts ${process.argv[2] === "--check" ? "match" : "created"}: ${files.length}`);
}
