param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [Parameter(Mandatory = $true)][string]$PreparedFile,
    [ValidateRange(2,16)][int]$CasesPerFamily = 2,
    [ValidateSet("optimized-tuned", "three-way")][string]$VariantMode = "optimized-tuned",
    [ValidatePattern('^[0-9a-f]{64}$')][string]$Seed = "7a1234ef20ab719c49d50208f471338015f76a9a0be99432bfa778b6af01d5c2",
    [switch]$SkipDependencyInstall
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$PreparedFile = (Resolve-Path -LiteralPath $PreparedFile -ErrorAction Stop).Path
if ((Split-Path -Path $Project -Leaf) -ne "fhec-v2") {
    throw "Phase 10/11 only accepts the existing dedicated fhec-v2 project"
}
$prepared = Get-Content -LiteralPath $PreparedFile -Raw | ConvertFrom-Json
if ($prepared.schema -ne "fhec-cofhe-phase89-local-prepared-v1" -or
    $prepared.project -ne $Project -or $prepared.chain_id -ne 421614 -or
    $prepared.codegen_confirmed -ne $true -or $prepared.mock_preflight_passed -ne $true -or
    $prepared.live_transactions_sent -ne $false) {
    throw "Phase 10/11 requires a successful unmodified Phase 8/9 prepared source record"
}
$generated = (Resolve-Path -LiteralPath ([string]$prepared.generated) -ErrorAction Stop).Path
$sourceRoot = Join-Path $Project "integration\cofhe-hardhat\phase89\generated"
if (-not $generated.StartsWith(($sourceRoot + [IO.Path]::DirectorySeparatorChar), [StringComparison]::OrdinalIgnoreCase)) {
    throw "Prepared Solidity source is outside the reviewed Phase 8/9 generated-source boundary"
}
$seal = [string]$prepared.seal_file
$plan = [string]$prepared.plan_file
if (-not (Test-Path -LiteralPath $seal -PathType Leaf) -or -not (Test-Path -LiteralPath $plan -PathType Leaf) -or
    (Get-FileHash -LiteralPath $seal -Algorithm SHA256).Hash -ne $prepared.seal_sha256 -or
    (Get-FileHash -LiteralPath $plan -Algorithm SHA256).Hash -ne $prepared.plan_sha256) {
    throw "Phase 8/9 prepared plan or source seal was modified since its successful mock preflight"
}

$root = Join-Path $Project "tools\cofhe"
$hardhat = Join-Path $Project "integration\cofhe-hardhat"
$original = (Get-Location).Path
$previousSource = [Environment]::GetEnvironmentVariable("FHEC_PHASE89_SOURCES_DIR", "Process")
$previousSeal = [Environment]::GetEnvironmentVariable("FHEC_PHASE89_SEAL_FILE", "Process")
$previousCorpus = [Environment]::GetEnvironmentVariable("FHEC_PHASE10_CORPUS_FILE", "Process")
$previousRaw = [Environment]::GetEnvironmentVariable("FHEC_PHASE10_RAW_FILE", "Process")
$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
$out = Join-Path $Project "target\cofhe-phase1011-dossier-$runId"
$copies = Join-Path $out "sources"
$corpus = Join-Path $out "corpus.json"
$raw = Join-Path $out "mock-observations.json"
$validated = Join-Path $out "validated-mock-results.json"
$html = Join-Path $out "offline-report.html"
$readiness = Join-Path $out "readiness-gates.json"
$manifest = Join-Path $out "phase11-evidence-manifest.json"

foreach ($tool in @("node", "pnpm")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Missing Phase 10/11 prerequisite command: $tool"
    }
}
if (Test-Path -LiteralPath $out) { throw "Refusing to overwrite a prior Phase 10/11 evidence directory" }
$required = @(
    "tools\cofhe\phase89\source-seal.mjs",
    "tools\cofhe\phase10\oracle.mjs",
    "tools\cofhe\phase10\verify-fixtures.mjs",
    "tools\cofhe\phase10\validate-results.mjs",
    "tools\cofhe\phase11\report-html.mjs",
    "tools\cofhe\phase11\readiness.mjs",
    "tools\cofhe\phase11\attest.mjs",
    "integration\cofhe-hardhat\phase10\test\FHECPhase10Properties.test.ts",
    "integration\cofhe-hardhat\phase10\tsconfig.phase10.json"
)
foreach ($relative in $required) {
    if (-not (Test-Path -LiteralPath (Join-Path $Project $relative) -PathType Leaf)) {
        throw "Missing Phase 10/11 component: $relative"
    }
}
$sourceNames = @(
    "FHECPhase6Optimized.sol", "FHECPhase6Live.sol", "FHECPhase7ManualStraight.sol",
    "FHECPhase7ManualTuned.sol", "FHECPhase7Straight.sol", "FHECPhase7Tuned.sol",
    "phase89-plan.json", "phase89-source-seal.json"
)
foreach ($name in $sourceNames) {
    if (-not (Test-Path -LiteralPath (Join-Path $generated $name) -PathType Leaf)) {
        throw "Prepared Solidity source file missing: $name"
    }
}

New-Item -ItemType Directory -Path $copies -Force -ErrorAction Stop | Out-Null
try {
    Set-Location -LiteralPath $Project
    node tools/cofhe/phase89/source-seal.mjs --directory $generated --verify $seal
    if ($LASTEXITCODE -ne 0) { throw "Phase 8/9 source identity no longer matches its original seal" }

    $testFiles = @(
        "tools/cofhe/phase10/oracle.test.mjs",
        "tools/cofhe/phase10/verify-fixtures.test.mjs",
        "tools/cofhe/phase10/validate-results.test.mjs",
        "tools/cofhe/phase11/report-html.test.mjs",
        "tools/cofhe/phase11/readiness.test.mjs",
        "tools/cofhe/phase11/attest.test.mjs"
    )
    node --test @testFiles
    if ($LASTEXITCODE -ne 0) { throw "Phase 10/11 independent offline unit and adversarial tests failed" }
    node tools/cofhe/phase10/verify-fixtures.mjs
    if ($LASTEXITCODE -ne 0) { throw "Six golden full-width independent reference corpus fixtures differ" }

    node tools/cofhe/phase10/oracle.mjs --cases $CasesPerFamily --mode $VariantMode --seed $Seed --out $corpus
    if ($LASTEXITCODE -ne 0) { throw "Independent BigInt oracle corpus generation failed" }

    [Environment]::SetEnvironmentVariable("FHEC_PHASE89_SOURCES_DIR", $generated, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE89_SEAL_FILE", $seal, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE10_CORPUS_FILE", $corpus, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE10_RAW_FILE", $raw, "Process")

    Set-Location -LiteralPath $hardhat
    if (-not $SkipDependencyInstall) {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "Pinned CoFHE Hardhat dependency installation failed" }
    }
    pnpm exec hardhat compile --config phase89/hardhat.phase89.config.ts
    if ($LASTEXITCODE -ne 0) { throw "Exact prepared CoFHE source compilation failed" }
    pnpm exec tsc --noEmit --project phase10/tsconfig.phase10.json
    if ($LASTEXITCODE -ne 0) { throw "Phase 10 TypeScript scoped typecheck failed" }
    pnpm exec hardhat test --config phase89/hardhat.phase89.config.ts phase10/test/FHECPhase10Properties.test.ts
    if ($LASTEXITCODE -ne 0) { throw "Phase 10 full-width property and contract-domain mock tests failed" }
    Set-Location -LiteralPath $Project

    node tools/cofhe/phase89/source-seal.mjs --directory $generated --verify $seal
    if ($LASTEXITCODE -ne 0) { throw "Generated Solidity changed while mocks were executing" }
    foreach ($name in $sourceNames) {
        Copy-Item -LiteralPath (Join-Path $generated $name) -Destination (Join-Path $copies $name) -ErrorAction Stop
    }
    node tools/cofhe/phase89/source-seal.mjs --directory $copies --verify (Join-Path $copies "phase89-source-seal.json")
    if ($LASTEXITCODE -ne 0) { throw "Portable source snapshot does not match the original prepared source seal" }

    node tools/cofhe/phase10/validate-results.mjs --corpus $corpus --raw $raw --seal (Join-Path $copies "phase89-source-seal.json") --out $validated --negative-confirmed true
    if ($LASTEXITCODE -ne 0) { throw "Independent mock result reconstruction or negative contract-domain test failed" }
    node tools/cofhe/phase11/report-html.mjs --validated $validated --out $html
    if ($LASTEXITCODE -ne 0) { throw "Phase 11 offline evidence rendering failed" }
    node tools/cofhe/phase11/readiness.mjs --validated $validated --out $readiness
    if ($LASTEXITCODE -ne 0) { throw "Phase 11 release and grant evidence gate assessment failed" }
    node tools/cofhe/phase11/attest.mjs --directory $out --write $manifest
    if ($LASTEXITCODE -ne 0) { throw "Portable evidence source and output manifest generation failed" }
    node tools/cofhe/phase11/attest.mjs --directory $out --verify $manifest
    if ($LASTEXITCODE -ne 0) { throw "Independent offline evidence manifest verification failed" }

    Write-Host "PHASES 10 + 11 LOCAL FHEC COFHE PROPERTY AND REPRODUCIBILITY SUITE PASSED" -ForegroundColor Green
    Write-Host "Self-contained evidence directory: $out"
    Write-Host "Offline HTML report: $html"
    Write-Host "Replayable sealed evidence: $manifest"
    Write-Host "No real network or wallet transactions were performed by this local verifier." -ForegroundColor Yellow
}
finally {
    Set-Location -LiteralPath $original
    [Environment]::SetEnvironmentVariable("FHEC_PHASE89_SOURCES_DIR", $previousSource, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE89_SEAL_FILE", $previousSeal, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE10_CORPUS_FILE", $previousCorpus, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE10_RAW_FILE", $previousRaw, "Process")
}
