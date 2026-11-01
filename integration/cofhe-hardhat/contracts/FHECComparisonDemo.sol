// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.25;

import {FHE, euint8, externalEuint8} from "@fhenixprotocol/cofhe-contracts/FHE.sol";
import {FHECGenerated} from "./FHECGenerated.sol";

contract FHECComparisonDemo {
    euint8 public result;

    function compare(
        externalEuint8 a,
        bytes calldata proofA,
        externalEuint8 b,
        bytes calldata proofB
    ) external {
        euint8 left = FHE.asEuint8(a, proofA);
        euint8 right = FHE.asEuint8(b, proofB);
        result = FHECGenerated.pick_greater(left, right);
        FHE.allowThis(result);
        FHE.allowSender(result);
    }
}
