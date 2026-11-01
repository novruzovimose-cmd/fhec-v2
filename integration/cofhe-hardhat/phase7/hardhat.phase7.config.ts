import { resolve } from "node:path";
import type { HardhatUserConfig } from "hardhat/config";
import base from "../hardhat.config";

const root = resolve(__dirname, "..");
const config: HardhatUserConfig = {
  ...base,
  defaultNetwork: "hardhat",
  paths: {
    root,
    sources: process.env.FHEC_PHASE7_SOURCES_DIR || resolve(__dirname, "generated"),
    tests: resolve(__dirname, "test"),
    cache: resolve(root, "cache", "phase7"),
    artifacts: resolve(root, "artifacts", "phase7"),
  },
};

export default config;
