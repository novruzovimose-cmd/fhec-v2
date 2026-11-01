// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.25;

import {
    FHE,
    euint8,
    euint16,
    euint32,
    euint64,
    ebool,
    externalEuint8,
    externalEuint16,
    externalEuint32,
    externalEuint64
} from "@fhenixprotocol/cofhe-contracts/FHE.sol";
import {FHECGenerated} from "./FHECGenerated.sol";

contract FHECExtendedDemo {
    euint16 public sum16;
    euint32 public product32;
    ebool public comparison64;
    euint8 public fourfold8;

    function calculateSum16(
        externalEuint16 a,
        bytes calldata proofA,
        externalEuint16 b,
        bytes calldata proofB
    ) external {
        euint16 left = FHE.asEuint16(a, proofA);
        euint16 right = FHE.asEuint16(b, proofB);
        sum16 = FHECGenerated.add_u16(left, right);
        FHE.allowThis(sum16);
        FHE.allowSender(sum16);
    }

    function calculateProduct32(
        externalEuint32 a,
        bytes calldata proofA,
        externalEuint32 b,
        bytes calldata proofB
    ) external {
        euint32 left = FHE.asEuint32(a, proofA);
        euint32 right = FHE.asEuint32(b, proofB);
        product32 = FHECGenerated.multiply_u32(left, right);
        FHE.allowThis(product32);
        FHE.allowSender(product32);
    }

    function calculateGreater64(
        externalEuint64 a,
        bytes calldata proofA,
        externalEuint64 b,
        bytes calldata proofB
    ) external {
        euint64 left = FHE.asEuint64(a, proofA);
        euint64 right = FHE.asEuint64(b, proofB);
        comparison64 = FHECGenerated.greater_u64(left, right);
        FHE.allowThis(comparison64);
        FHE.allowSender(comparison64);
    }

    function calculateFourfold8(externalEuint8 a, bytes calldata proof) external {
        euint8 value = FHE.asEuint8(a, proof);
        fourfold8 = FHECGenerated.four_times_u8(value);
        FHE.allowThis(fourfold8);
        FHE.allowSender(fourfold8);
    }
}
