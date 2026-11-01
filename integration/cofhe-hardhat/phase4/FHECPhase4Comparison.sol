// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.25;

import {FHE, ebool, euint8, externalEuint8, euint16, externalEuint16, euint32, externalEuint32, euint64, externalEuint64} from "@fhenixprotocol/cofhe-contracts/FHE.sol";
import {FHECPhase4Baseline} from "./FHECPhase4Baseline.sol";
import {FHECPhase4Optimized} from "./FHECPhase4Optimized.sol";
import {FHECPhase4Manual} from "./FHECPhase4Manual.sol";

contract FHECPhase4Comparison {
    euint8 public baselineValue8;
    ebool public baselineBool8;
    euint8 public optimizedValue8;
    ebool public optimizedBool8;
    euint8 public manualValue8;
    ebool public manualBool8;
    euint16 public baselineValue16;
    ebool public baselineBool16;
    euint16 public optimizedValue16;
    ebool public optimizedBool16;
    euint16 public manualValue16;
    ebool public manualBool16;
    euint32 public baselineValue32;
    ebool public baselineBool32;
    euint32 public optimizedValue32;
    ebool public optimizedBool32;
    euint32 public manualValue32;
    ebool public manualBool32;
    euint64 public baselineValue64;
    ebool public baselineBool64;
    euint64 public optimizedValue64;
    ebool public optimizedBool64;
    euint64 public manualValue64;
    ebool public manualBool64;

    function runBaselineU8(uint8 operation, externalEuint8 encryptedA, bytes calldata proofA, externalEuint8 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint8 a = FHE.asEuint8(encryptedA, proofA);
        euint8 b = FHE.asEuint8(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Baseline.gt_u8(a, b);
            baselineBool8 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint8 result;
        if (operation == 0) result = FHECPhase4Baseline.add_u8(a, b);
        else if (operation == 1) result = FHECPhase4Baseline.sub_u8(a, b);
        else if (operation == 2) result = FHECPhase4Baseline.mul_u8(a, b);
        else if (operation == 4) result = FHECPhase4Baseline.max_u8(a, b);
        else result = FHECPhase4Baseline.repeated_u8(a, b);
        baselineValue8 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runOptimizedU8(uint8 operation, externalEuint8 encryptedA, bytes calldata proofA, externalEuint8 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint8 a = FHE.asEuint8(encryptedA, proofA);
        euint8 b = FHE.asEuint8(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Optimized.gt_u8(a, b);
            optimizedBool8 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint8 result;
        if (operation == 0) result = FHECPhase4Optimized.add_u8(a, b);
        else if (operation == 1) result = FHECPhase4Optimized.sub_u8(a, b);
        else if (operation == 2) result = FHECPhase4Optimized.mul_u8(a, b);
        else if (operation == 4) result = FHECPhase4Optimized.max_u8(a, b);
        else result = FHECPhase4Optimized.repeated_u8(a, b);
        optimizedValue8 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runManualU8(uint8 operation, externalEuint8 encryptedA, bytes calldata proofA, externalEuint8 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint8 a = FHE.asEuint8(encryptedA, proofA);
        euint8 b = FHE.asEuint8(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Manual.gt_u8(a, b);
            manualBool8 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint8 result;
        if (operation == 0) result = FHECPhase4Manual.add_u8(a, b);
        else if (operation == 1) result = FHECPhase4Manual.sub_u8(a, b);
        else if (operation == 2) result = FHECPhase4Manual.mul_u8(a, b);
        else if (operation == 4) result = FHECPhase4Manual.max_u8(a, b);
        else result = FHECPhase4Manual.repeated_u8(a, b);
        manualValue8 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runBaselineU16(uint8 operation, externalEuint16 encryptedA, bytes calldata proofA, externalEuint16 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint16 a = FHE.asEuint16(encryptedA, proofA);
        euint16 b = FHE.asEuint16(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Baseline.gt_u16(a, b);
            baselineBool16 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint16 result;
        if (operation == 0) result = FHECPhase4Baseline.add_u16(a, b);
        else if (operation == 1) result = FHECPhase4Baseline.sub_u16(a, b);
        else if (operation == 2) result = FHECPhase4Baseline.mul_u16(a, b);
        else if (operation == 4) result = FHECPhase4Baseline.max_u16(a, b);
        else result = FHECPhase4Baseline.repeated_u16(a, b);
        baselineValue16 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runOptimizedU16(uint8 operation, externalEuint16 encryptedA, bytes calldata proofA, externalEuint16 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint16 a = FHE.asEuint16(encryptedA, proofA);
        euint16 b = FHE.asEuint16(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Optimized.gt_u16(a, b);
            optimizedBool16 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint16 result;
        if (operation == 0) result = FHECPhase4Optimized.add_u16(a, b);
        else if (operation == 1) result = FHECPhase4Optimized.sub_u16(a, b);
        else if (operation == 2) result = FHECPhase4Optimized.mul_u16(a, b);
        else if (operation == 4) result = FHECPhase4Optimized.max_u16(a, b);
        else result = FHECPhase4Optimized.repeated_u16(a, b);
        optimizedValue16 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runManualU16(uint8 operation, externalEuint16 encryptedA, bytes calldata proofA, externalEuint16 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint16 a = FHE.asEuint16(encryptedA, proofA);
        euint16 b = FHE.asEuint16(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Manual.gt_u16(a, b);
            manualBool16 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint16 result;
        if (operation == 0) result = FHECPhase4Manual.add_u16(a, b);
        else if (operation == 1) result = FHECPhase4Manual.sub_u16(a, b);
        else if (operation == 2) result = FHECPhase4Manual.mul_u16(a, b);
        else if (operation == 4) result = FHECPhase4Manual.max_u16(a, b);
        else result = FHECPhase4Manual.repeated_u16(a, b);
        manualValue16 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runBaselineU32(uint8 operation, externalEuint32 encryptedA, bytes calldata proofA, externalEuint32 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint32 a = FHE.asEuint32(encryptedA, proofA);
        euint32 b = FHE.asEuint32(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Baseline.gt_u32(a, b);
            baselineBool32 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint32 result;
        if (operation == 0) result = FHECPhase4Baseline.add_u32(a, b);
        else if (operation == 1) result = FHECPhase4Baseline.sub_u32(a, b);
        else if (operation == 2) result = FHECPhase4Baseline.mul_u32(a, b);
        else if (operation == 4) result = FHECPhase4Baseline.max_u32(a, b);
        else result = FHECPhase4Baseline.repeated_u32(a, b);
        baselineValue32 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runOptimizedU32(uint8 operation, externalEuint32 encryptedA, bytes calldata proofA, externalEuint32 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint32 a = FHE.asEuint32(encryptedA, proofA);
        euint32 b = FHE.asEuint32(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Optimized.gt_u32(a, b);
            optimizedBool32 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint32 result;
        if (operation == 0) result = FHECPhase4Optimized.add_u32(a, b);
        else if (operation == 1) result = FHECPhase4Optimized.sub_u32(a, b);
        else if (operation == 2) result = FHECPhase4Optimized.mul_u32(a, b);
        else if (operation == 4) result = FHECPhase4Optimized.max_u32(a, b);
        else result = FHECPhase4Optimized.repeated_u32(a, b);
        optimizedValue32 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runManualU32(uint8 operation, externalEuint32 encryptedA, bytes calldata proofA, externalEuint32 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint32 a = FHE.asEuint32(encryptedA, proofA);
        euint32 b = FHE.asEuint32(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Manual.gt_u32(a, b);
            manualBool32 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint32 result;
        if (operation == 0) result = FHECPhase4Manual.add_u32(a, b);
        else if (operation == 1) result = FHECPhase4Manual.sub_u32(a, b);
        else if (operation == 2) result = FHECPhase4Manual.mul_u32(a, b);
        else if (operation == 4) result = FHECPhase4Manual.max_u32(a, b);
        else result = FHECPhase4Manual.repeated_u32(a, b);
        manualValue32 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runBaselineU64(uint8 operation, externalEuint64 encryptedA, bytes calldata proofA, externalEuint64 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint64 a = FHE.asEuint64(encryptedA, proofA);
        euint64 b = FHE.asEuint64(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Baseline.gt_u64(a, b);
            baselineBool64 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint64 result;
        if (operation == 0) result = FHECPhase4Baseline.add_u64(a, b);
        else if (operation == 1) result = FHECPhase4Baseline.sub_u64(a, b);
        else if (operation == 2) result = FHECPhase4Baseline.mul_u64(a, b);
        else if (operation == 4) result = FHECPhase4Baseline.max_u64(a, b);
        else result = FHECPhase4Baseline.repeated_u64(a, b);
        baselineValue64 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runOptimizedU64(uint8 operation, externalEuint64 encryptedA, bytes calldata proofA, externalEuint64 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint64 a = FHE.asEuint64(encryptedA, proofA);
        euint64 b = FHE.asEuint64(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Optimized.gt_u64(a, b);
            optimizedBool64 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint64 result;
        if (operation == 0) result = FHECPhase4Optimized.add_u64(a, b);
        else if (operation == 1) result = FHECPhase4Optimized.sub_u64(a, b);
        else if (operation == 2) result = FHECPhase4Optimized.mul_u64(a, b);
        else if (operation == 4) result = FHECPhase4Optimized.max_u64(a, b);
        else result = FHECPhase4Optimized.repeated_u64(a, b);
        optimizedValue64 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

    function runManualU64(uint8 operation, externalEuint64 encryptedA, bytes calldata proofA, externalEuint64 encryptedB, bytes calldata proofB) external {
        require(operation < 6, "invalid operation");
        euint64 a = FHE.asEuint64(encryptedA, proofA);
        euint64 b = FHE.asEuint64(encryptedB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase4Manual.gt_u64(a, b);
            manualBool64 = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint64 result;
        if (operation == 0) result = FHECPhase4Manual.add_u64(a, b);
        else if (operation == 1) result = FHECPhase4Manual.sub_u64(a, b);
        else if (operation == 2) result = FHECPhase4Manual.mul_u64(a, b);
        else if (operation == 4) result = FHECPhase4Manual.max_u64(a, b);
        else result = FHECPhase4Manual.repeated_u64(a, b);
        manualValue64 = result;
        FHE.allowThis(result);
        FHE.allowSender(result);
    }

}
