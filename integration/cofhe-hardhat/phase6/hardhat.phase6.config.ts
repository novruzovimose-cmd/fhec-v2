import { resolve } from "node:path";
import type { HardhatUserConfig } from "hardhat/config";
import base from "../hardhat.config";

const root = resolve(__dirname, "..");
const rawKey = process.env.FHEC_TESTNET_PRIVATE_KEY;
const accounts = rawKey && /^0x[a-fA-F0-9]{64}$/.test(rawKey) ? [rawKey] : [];

const config: HardhatUserConfig = {
  ...base,
  defaultNetwork: "hardhat",
  networks: {
    ...(base.networks ?? {}),
    "arb-sepolia": {
      url: process.env.FHEC_TESTNET_RPC_URL || "http://127.0.0.1:8545",
      chainId: 421614,
      accounts,
    },
  },
  paths: {
    root,
    sources: resolve(__dirname, "generated"),
    tests: resolve(__dirname, "test"),
    cache: resolve(root, "cache", "phase6"),
    artifacts: resolve(root, "artifacts", "phase6"),
  },
};

export default config;
