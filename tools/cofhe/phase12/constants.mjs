export const fixtureSourceSealSha256 = "721BB33FB4183356CC66351CD210B3CACA201BB9CAB6E964FD526890679E6D11";
export const operations = Object.freeze(["add", "sub", "mul", "gt", "max", "repeated"]);
export const widths = Object.freeze([8, 16, 32, 64]);
export const variants = Object.freeze(["optimized", "tuned", "straight"]);
export const localEvidenceFiles = Object.freeze([
  "corpus.json",
  "mock-observations.json",
  "validated-mock-results.json",
  "offline-report.html",
  "readiness-gates.json",
  "sources/FHECPhase6Optimized.sol",
  "sources/FHECPhase6Live.sol",
  "sources/FHECPhase7ManualStraight.sol",
  "sources/FHECPhase7ManualTuned.sol",
  "sources/FHECPhase7Straight.sol",
  "sources/FHECPhase7Tuned.sol",
  "sources/phase89-plan.json",
  "sources/phase89-source-seal.json",
  "phase11-evidence-manifest.json",
]);
export const candidateFiles = Object.freeze([
  "phase11/" + localEvidenceFiles[0],
  ...localEvidenceFiles.slice(1).map((name) => "phase11/" + name),
  "source-inventory.json",
  "qualification.json",
  "qualification-report.html",
  "RELEASE-CANDIDATE.md",
]);
