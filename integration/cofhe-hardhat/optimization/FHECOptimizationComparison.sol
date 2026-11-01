// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.25;

import {
    FHE,
    euint8,
    externalEuint8
} from "@fhenixprotocol/cofhe-contracts/FHE.sol";
import {FHECOptimizationBaseline} from "./FHECOptimizationBaseline.sol";
import {FHECOptimizationOptimized} from "./FHECOptimizationOptimized.sol";

contract FHECOptimizationComparison {
    euint8 public baselineValue;
    euint8 public optimizedValue;

    function runBaseline(
        externalEuint8 a,
        bytes calldata proofA,
        externalEuint8 b,
        bytes calldata proofB
    ) external {
        euint8 left = FHE.asEuint8(a, proofA);
        euint8 right = FHE.asEuint8(b, proofB);
        baselineValue = FHECOptimizationBaseline.repeated(left, right);
        FHE.allowThis(baselineValue);
        FHE.allowSender(baselineValue);
    }

    function runOptimized(
        externalEuint8 a,
        bytes calldata proofA,
        externalEuint8 b,
        bytes calldata proofB
    ) external {
        euint8 left = FHE.asEuint8(a, proofA);
        euint8 right = FHE.asEuint8(b, proofB);
        optimizedValue = FHECOptimizationOptimized.repeated(left, right);
        FHE.allowThis(optimizedValue);
        FHE.allowSender(optimizedValue);
    }
}
