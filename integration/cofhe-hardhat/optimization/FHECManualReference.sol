// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.25;

import {FHE, euint8} from "@fhenixprotocol/cofhe-contracts/FHE.sol";

library FHECManualReference {
    function repeated(euint8 a, euint8 b) internal returns (euint8) {
        euint8 product = FHE.mul(a, b);
        return FHE.add(product, product);
    }
}
