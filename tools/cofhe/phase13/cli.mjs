import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkLive } from "./chain.mjs";
import { checkPublicCi } from "./github.mjs";
import { newOutput, redactError, writeNewJson } from "./io.mjs";
import { createPacket, verifyPacket } from "./packet.mjs";
import { verifyLocal } from "./source.mjs";

const commands = {
  preflight: ["project", "candidate"],
  ci: ["repo", "run", "sha"],
  live: ["plan", "raw", "confirmations"],
  collect: ["project", "candidate", "repo", "run", "sha", "plan", "raw", "confirmations"],
  verify: ["directory"],
};

export function parseArgs(argv) {
  const [command, ...values] = argv;
  if (!Object.hasOwn(commands, command)) throw new Error("Unknown Phase 13 command. Use preflight, ci, live, collect or verify");
  if (values.length % 2) throw new Error("Arguments must be supplied as --name value pairs");
  const flags = {};
  for (let index = 0; index < values.length; index += 2) {
    const flag = values[index];
    if (!/^--[a-z-]+$/.test(flag)) throw new Error("Invalid flag name");
    const key = flag.slice(2);
    if (!commands[command].includes(key) || Object.hasOwn(flags, key) ||
        typeof values[index + 1] !== "string" || !values[index + 1].trim()) {
      throw new Error("Duplicate, unknown or empty Phase 13 parameter: " + flag);
    }
    flags[key] = values[index + 1];
  }
  if (command === "preflight" || command === "collect") {
    if (!flags.candidate) throw new Error("--candidate is required");
  }
  if (command === "ci" && (!flags.repo || !flags.run || !flags.sha)) throw new Error("--repo, --run and --sha are mandatory together");
  if (command === "live" && (!flags.plan || !flags.raw)) throw new Error("--plan and --raw are mandatory together");
  if (command === "verify" && !flags.directory) throw new Error("--directory is mandatory");
  if (command === "collect") {
    if ([flags.repo, flags.run, flags.sha].some(Boolean) && ![flags.repo, flags.run, flags.sha].every(Boolean)) {
      throw new Error("Public CI verification needs --repo, --run and --sha together");
    }
    if (!!flags.plan !== !!flags.raw) throw new Error("Live verification needs both --plan and --raw");
  }
  if (flags.confirmations && (!/^[0-9]{1,2}$/.test(flags.confirmations) || Number(flags.confirmations) < 2 || Number(flags.confirmations) > 64)) {
    throw new Error("--confirmations must be between 2 and 64");
  }
  return { command, flags };
}

function rpcSettings() {
  const a = process.env.FHEC_PHASE13_RPC_A;
  const b = process.env.FHEC_PHASE13_RPC_B;
  if (!a || !b) throw new Error("Two distinct HTTPS Arbitrum Sepolia RPC endpoints are required in FHEC_PHASE13_RPC_A/B");
  return [a, b];
}

export async function run(argv) {
  const { command, flags } = parseArgs(argv);
  const project = resolve(flags.project || process.cwd());
  if (command === "verify") {
    const verified = verifyPacket(flags.directory);
    process.stdout.write(JSON.stringify(verified, null, 2) + "\n");
    return;
  }
  const local = command === "collect" || command === "preflight" ? verifyLocal(project, flags.candidate) : null;
  const ci = flags.repo ? await checkPublicCi(flags.repo, flags.run, flags.sha) : null;
  let live = null;
  if (flags.plan) {
    const [a, b] = rpcSettings();
    live = await checkLive(flags.plan, flags.raw, a, b, Number(flags.confirmations || "2"));
  }
  const directory = newOutput(project, command);
  if (command === "preflight") {
    writeNewJson(resolve(directory, "source-preflight.json"), local);
    process.stdout.write(`PHASE 13 OFFLINE SOURCE AND CANDIDATE VERIFICATION PASSED\nOutput: ${directory}\n`);
  } else if (command === "ci") {
    writeNewJson(resolve(directory, "github-public-ci.json"), ci);
    process.stdout.write(`PUBLIC HOSTED CI METADATA VERIFIED\nOutput: ${directory}\n`);
  } else if (command === "live") {
    writeNewJson(resolve(directory, "two-rpc-consensus.json"), live.consensus);
    writeNewJson(resolve(directory, "real-native-gas-comparison.json"), live.analysis);
    writeNewJson(resolve(directory, "public-transaction-ids.json"), live.public_receipt_reference);
    process.stdout.write(`PHASE 13 READ-ONLY TWO-PROVIDER TESTNET VERIFICATION PASSED\nOutput: ${directory}\nNo transactions submitted; coprocessor fees are unmeasured.\n`);
  } else if (command === "collect") {
    const result = createPacket(directory, local, live, ci);
    process.stdout.write(`PHASE 13 GRANT EVIDENCE PACKET VERIFIED\nOutput: ${directory}\nExternal gates corroborated: ${result.externally_corroborated}/4\nNo grant or security certification asserted.\n`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  run(process.argv.slice(2)).catch((error) => {
    process.stderr.write("Phase 13 failed closed: " + redactError(error) + "\n");
    process.exitCode = 1;
  });
}
