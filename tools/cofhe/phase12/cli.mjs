import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createCandidate } from "./candidate.mjs";
import { verifyCandidate } from "./verify.mjs";
import { qualifyDossier } from "./qualification.mjs";
import { rehearseTampering } from "./adversarial.mjs";
import { compareIndependentRuns } from "./repro.mjs";
import { writeJsonNew } from "./io.mjs";

function parseOptions(args) {
  const options = new Map();
  for (let index = 0; index < args.length; index += 2) {
    if (!/^--[a-z-]+$/.test(args[index]) || typeof args[index + 1] !== "string" || options.has(args[index])) {
      throw new Error("Repeated, missing or malformed Phase 12 command-line option");
    }
    options.set(args[index], args[index + 1]);
  }
  return options;
}

function required(options, key) {
  const value = options.get(key);
  if (!value || !value.trim()) throw new Error("Missing required Phase 12 option " + key);
  return value;
}

function only(options, allowed) {
  for (const key of options.keys()) if (!allowed.includes(key)) throw new Error("Unknown Phase 12 option: " + key);
}

export function run(args) {
  const [command, ...rest] = args;
  const opts = parseOptions(rest);
  let result;
  switch (command) {
    case "qualify":
      only(opts, ["--dossier", "--out", "--expected-seal"]);
      result = qualifyDossier(required(opts, "--dossier"), opts.get("--expected-seal") ?? null);
      if (opts.has("--out")) writeJsonNew(opts.get("--out"), result);
      break;
    case "create":
      only(opts, ["--project", "--dossier", "--out", "--expected-seal"]);
      result = createCandidate({ project: required(opts, "--project"), dossier: required(opts, "--dossier"), output: required(opts, "--out"), expectedSeal: opts.get("--expected-seal") ?? null });
      break;
    case "verify":
      only(opts, ["--candidate", "--project", "--expected-seal"]);
      result = verifyCandidate(required(opts, "--candidate"), opts.get("--project"), opts.get("--expected-seal") ?? null);
      break;
    case "attack":
      only(opts, ["--dossier", "--out", "--expected-seal"]);
      result = rehearseTampering(required(opts, "--dossier"), opts.get("--expected-seal") ?? null);
      if (opts.has("--out")) writeJsonNew(opts.get("--out"), result);
      break;
    case "compare":
      only(opts, ["--first", "--second", "--out"]);
      result = compareIndependentRuns(required(opts, "--first"), required(opts, "--second"));
      if (opts.has("--out")) writeJsonNew(opts.get("--out"), result);
      break;
    default:
      throw new Error("Usage: qualify --dossier DIR [--expected-seal SHA256] | create --project DIR --dossier DIR --out NEW_DIR [--expected-seal SHA256] | verify --candidate DIR [--project DIR] [--expected-seal SHA256] | attack --dossier DIR [--expected-seal SHA256] | compare --first DIR --second DIR");
  }
  return result;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { process.stdout.write(JSON.stringify(run(process.argv.slice(2)), null, 2) + "\n"); }
  catch (error) { process.stderr.write("Phase 12 failed closed: " + error.message + "\n"); process.exitCode = 1; }
}
