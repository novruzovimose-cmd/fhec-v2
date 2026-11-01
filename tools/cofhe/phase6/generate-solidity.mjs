import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { widths } from "./oracle.mjs";

export function generateSolidity() {
  const types = ["FHE", "ebool"];
  for (const width of widths) types.push(`euint${width}`, `externalEuint${width}`);
  const imports = `import {${types.join(", ")}} from "@fhenixprotocol/cofhe-contracts/FHE.sol";`;
  const stores = widths.map((width) => `    mapping(address => euint${width}) private values${width};\n    mapping(address => ebool) private comparisons${width};`).join("\n");
  const functions = widths.map((width) => `
    function runU${width}(
        uint8 operation,
        externalEuint${width} ciphertextA,
        bytes calldata proofA,
        externalEuint${width} ciphertextB,
        bytes calldata proofB
    ) external {
        require(operation < 6, "invalid operation");
        euint${width} a = FHE.asEuint${width}(ciphertextA, proofA);
        euint${width} b = FHE.asEuint${width}(ciphertextB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase6Optimized.gt_u${width}(a, b);
            comparisons${width}[msg.sender] = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint${width} output;
        if (operation == 0) output = FHECPhase6Optimized.add_u${width}(a, b);
        else if (operation == 1) output = FHECPhase6Optimized.sub_u${width}(a, b);
        else if (operation == 2) output = FHECPhase6Optimized.mul_u${width}(a, b);
        else if (operation == 4) output = FHECPhase6Optimized.max_u${width}(a, b);
        else output = FHECPhase6Optimized.repeated_u${width}(a, b);
        values${width}[msg.sender] = output;
        FHE.allowThis(output);
        FHE.allowSender(output);
    }

    function readValueU${width}(address account) external view returns (euint${width}) {
        return values${width}[account];
    }

    function readBoolU${width}(address account) external view returns (ebool) {
        return comparisons${width}[account];
    }
`).join("\n");
  return `// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.25;

${imports}
import {FHECPhase6Optimized} from "./FHECPhase6Optimized.sol";

contract FHECPhase6Live {
${stores}
${functions}
}
`;
}

export function main(args) {
  if (args.length !== 2 || args[0] !== "--out") throw new Error("Usage: node generate-solidity.mjs --out <FHECPhase6Live.sol>");
  writeFileSync(args[1], generateSolidity(), { flag: "wx" });
  process.stdout.write("Generated Phase 6 harness for four encrypted widths and six operation families.\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) { process.stderr.write(`Phase 6 Solidity generator error: ${error.message}\n`); process.exitCode = 1; }
}
