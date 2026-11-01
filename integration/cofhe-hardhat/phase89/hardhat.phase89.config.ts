import { resolve } from "node:path";
import type { HardhatUserConfig } from "hardhat/config";
import phase6 from "../phase6/hardhat.phase6.config";

const root = resolve(__dirname, "..");
const config: HardhatUserConfig = {
  ...phase6,
  defaultNetwork: "hardhat",
  paths: {
    root,
    sources: process.env.FHEC_PHASE89_SOURCES_DIR || resolve(__dirname, "generated"),
    tests: resolve(__dirname, "test"),
    cache: resolve(root, "cache", "phase89"),
    artifacts: resolve(root, "artifacts", "phase89"),
  },
};

export default config;
