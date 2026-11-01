// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.25;

import {FHE, ebool, euint8, euint16, euint32, euint64} from "@fhenixprotocol/cofhe-contracts/FHE.sol";

library FHECPhase4Manual {
    function add_u8(euint8 a, euint8 b) internal returns (euint8) {
        return FHE.add(a, b);
    }

    function sub_u8(euint8 a, euint8 b) internal returns (euint8) {
        return FHE.sub(a, b);
    }

    function mul_u8(euint8 a, euint8 b) internal returns (euint8) {
        return FHE.mul(a, b);
    }

    function gt_u8(euint8 a, euint8 b) internal returns (ebool) {
        return FHE.gt(a, b);
    }

    function max_u8(euint8 a, euint8 b) internal returns (euint8) {
        ebool chooseLeft = FHE.gt(a, b);
        return FHE.select(chooseLeft, a, b);
    }

    function repeated_u8(euint8 a, euint8 b) internal returns (euint8) {
        euint8 product = FHE.mul(a, b);
        return FHE.add(product, product);
    }

    function add_u16(euint16 a, euint16 b) internal returns (euint16) {
        return FHE.add(a, b);
    }

    function sub_u16(euint16 a, euint16 b) internal returns (euint16) {
        return FHE.sub(a, b);
    }

    function mul_u16(euint16 a, euint16 b) internal returns (euint16) {
        return FHE.mul(a, b);
    }

    function gt_u16(euint16 a, euint16 b) internal returns (ebool) {
        return FHE.gt(a, b);
    }

    function max_u16(euint16 a, euint16 b) internal returns (euint16) {
        ebool chooseLeft = FHE.gt(a, b);
        return FHE.select(chooseLeft, a, b);
    }

    function repeated_u16(euint16 a, euint16 b) internal returns (euint16) {
        euint16 product = FHE.mul(a, b);
        return FHE.add(product, product);
    }

    function add_u32(euint32 a, euint32 b) internal returns (euint32) {
        return FHE.add(a, b);
    }

    function sub_u32(euint32 a, euint32 b) internal returns (euint32) {
        return FHE.sub(a, b);
    }

    function mul_u32(euint32 a, euint32 b) internal returns (euint32) {
        return FHE.mul(a, b);
    }

    function gt_u32(euint32 a, euint32 b) internal returns (ebool) {
        return FHE.gt(a, b);
    }

    function max_u32(euint32 a, euint32 b) internal returns (euint32) {
        ebool chooseLeft = FHE.gt(a, b);
        return FHE.select(chooseLeft, a, b);
    }

    function repeated_u32(euint32 a, euint32 b) internal returns (euint32) {
        euint32 product = FHE.mul(a, b);
        return FHE.add(product, product);
    }

    function add_u64(euint64 a, euint64 b) internal returns (euint64) {
        return FHE.add(a, b);
    }

    function sub_u64(euint64 a, euint64 b) internal returns (euint64) {
        return FHE.sub(a, b);
    }

    function mul_u64(euint64 a, euint64 b) internal returns (euint64) {
        return FHE.mul(a, b);
    }

    function gt_u64(euint64 a, euint64 b) internal returns (ebool) {
        return FHE.gt(a, b);
    }

    function max_u64(euint64 a, euint64 b) internal returns (euint64) {
        ebool chooseLeft = FHE.gt(a, b);
        return FHE.select(chooseLeft, a, b);
    }

    function repeated_u64(euint64 a, euint64 b) internal returns (euint64) {
        euint64 product = FHE.mul(a, b);
        return FHE.add(product, product);
    }

}
