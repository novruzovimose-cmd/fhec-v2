// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.25;

import {FHE, ebool, euint8, externalEuint8, euint16, externalEuint16, euint32, externalEuint32, euint64, externalEuint64} from "@fhenixprotocol/cofhe-contracts/FHE.sol";
import {FHECPhase7ManualStraight} from "./FHECPhase7ManualStraight.sol";

contract FHECPhase7Straight {
    mapping(address => euint8) private values8;
    mapping(address => ebool) private comparisons8;
    mapping(address => euint16) private values16;
    mapping(address => ebool) private comparisons16;
    mapping(address => euint32) private values32;
    mapping(address => ebool) private comparisons32;
    mapping(address => euint64) private values64;
    mapping(address => ebool) private comparisons64;

    function runU8(
        uint8 operation,
        externalEuint8 ciphertextA,
        bytes calldata proofA,
        externalEuint8 ciphertextB,
        bytes calldata proofB
    ) external {
        require(operation < 6, "invalid operation");
        euint8 a = FHE.asEuint8(ciphertextA, proofA);
        euint8 b = FHE.asEuint8(ciphertextB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase7ManualStraight.gt_u8(a, b);
            comparisons8[msg.sender] = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint8 output;
        if (operation == 0) output = FHECPhase7ManualStraight.add_u8(a, b);
        else if (operation == 1) output = FHECPhase7ManualStraight.sub_u8(a, b);
        else if (operation == 2) output = FHECPhase7ManualStraight.mul_u8(a, b);
        else if (operation == 4) output = FHECPhase7ManualStraight.max_u8(a, b);
        else output = FHECPhase7ManualStraight.repeated_u8(a, b);
        values8[msg.sender] = output;
        FHE.allowThis(output);
        FHE.allowSender(output);
    }

    function readValueU8(address account) external view returns (euint8) {
        return values8[account];
    }

    function readBoolU8(address account) external view returns (ebool) {
        return comparisons8[account];
    }


    function runU16(
        uint8 operation,
        externalEuint16 ciphertextA,
        bytes calldata proofA,
        externalEuint16 ciphertextB,
        bytes calldata proofB
    ) external {
        require(operation < 6, "invalid operation");
        euint16 a = FHE.asEuint16(ciphertextA, proofA);
        euint16 b = FHE.asEuint16(ciphertextB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase7ManualStraight.gt_u16(a, b);
            comparisons16[msg.sender] = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint16 output;
        if (operation == 0) output = FHECPhase7ManualStraight.add_u16(a, b);
        else if (operation == 1) output = FHECPhase7ManualStraight.sub_u16(a, b);
        else if (operation == 2) output = FHECPhase7ManualStraight.mul_u16(a, b);
        else if (operation == 4) output = FHECPhase7ManualStraight.max_u16(a, b);
        else output = FHECPhase7ManualStraight.repeated_u16(a, b);
        values16[msg.sender] = output;
        FHE.allowThis(output);
        FHE.allowSender(output);
    }

    function readValueU16(address account) external view returns (euint16) {
        return values16[account];
    }

    function readBoolU16(address account) external view returns (ebool) {
        return comparisons16[account];
    }


    function runU32(
        uint8 operation,
        externalEuint32 ciphertextA,
        bytes calldata proofA,
        externalEuint32 ciphertextB,
        bytes calldata proofB
    ) external {
        require(operation < 6, "invalid operation");
        euint32 a = FHE.asEuint32(ciphertextA, proofA);
        euint32 b = FHE.asEuint32(ciphertextB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase7ManualStraight.gt_u32(a, b);
            comparisons32[msg.sender] = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint32 output;
        if (operation == 0) output = FHECPhase7ManualStraight.add_u32(a, b);
        else if (operation == 1) output = FHECPhase7ManualStraight.sub_u32(a, b);
        else if (operation == 2) output = FHECPhase7ManualStraight.mul_u32(a, b);
        else if (operation == 4) output = FHECPhase7ManualStraight.max_u32(a, b);
        else output = FHECPhase7ManualStraight.repeated_u32(a, b);
        values32[msg.sender] = output;
        FHE.allowThis(output);
        FHE.allowSender(output);
    }

    function readValueU32(address account) external view returns (euint32) {
        return values32[account];
    }

    function readBoolU32(address account) external view returns (ebool) {
        return comparisons32[account];
    }


    function runU64(
        uint8 operation,
        externalEuint64 ciphertextA,
        bytes calldata proofA,
        externalEuint64 ciphertextB,
        bytes calldata proofB
    ) external {
        require(operation < 6, "invalid operation");
        euint64 a = FHE.asEuint64(ciphertextA, proofA);
        euint64 b = FHE.asEuint64(ciphertextB, proofB);
        if (operation == 3) {
            ebool compared = FHECPhase7ManualStraight.gt_u64(a, b);
            comparisons64[msg.sender] = compared;
            FHE.allowThis(compared);
            FHE.allowSender(compared);
            return;
        }
        euint64 output;
        if (operation == 0) output = FHECPhase7ManualStraight.add_u64(a, b);
        else if (operation == 1) output = FHECPhase7ManualStraight.sub_u64(a, b);
        else if (operation == 2) output = FHECPhase7ManualStraight.mul_u64(a, b);
        else if (operation == 4) output = FHECPhase7ManualStraight.max_u64(a, b);
        else output = FHECPhase7ManualStraight.repeated_u64(a, b);
        values64[msg.sender] = output;
        FHE.allowThis(output);
        FHE.allowSender(output);
    }

    function readValueU64(address account) external view returns (euint64) {
        return values64[account];
    }

    function readBoolU64(address account) external view returns (ebool) {
        return comparisons64[account];
    }

}
