param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [switch]$SkipDependencyInstall,
    [switch]$SkipRustTests,
    [ValidateRange(1,8)][int]$CasesPerFamily = 2
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$hardhat = Join-Path $Project "integration\cofhe-hardhat"
$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
$runDirectory = Join-Path $Project "target\cofhe-phase4-$runId"
$originalLocation = (Get-Location).Path
$previousCases = [Environment]::GetEnvironmentVariable("FHEC_PHASE4_CASES_PER_FAMILY", "Process")
$previousEvidence = [Environment]::GetEnvironmentVariable("FHEC_PHASE4_EVIDENCE_FILE", "Process")
$corpus = Join-Path $hardhat "testdata\cofhe-vectors.jsonl"
$transcript = Join-Path $runDirectory "validation-transcript.txt"
$evidencePath = Join-Path $runDirectory "mock-samples.json"
$summaryPath = Join-Path $runDirectory "mock-gas-summary.json"
$htmlPath = Join-Path $runDirectory "mock-evidence-report.html"
$budgetPath = Join-Path $runDirectory "encrypted-operation-budget.json"
$contrastPath = Join-Path $runDirectory "encrypted-operation-budget-contrast.json"
$reportPath = Join-Path $runDirectory "optimized-static-report.json"
$validationPath = Join-Path $runDirectory "validation-report.json"
$passed = $false
$recording = $false
$staged = @()
$backups = @{}

$source = Join-Path $Project "examples\cofhe_phase4.fhe"
$phase3Source = Join-Path $Project "examples\cofhe_optimization.fhe"
$extendedSource = Join-Path $Project "examples\cofhe_extended.fhe"
$phase4 = Join-Path $hardhat "phase4"
$optimizations = Join-Path $hardhat "optimization"
$placements = [ordered]@{
    "FHECGenerated.sol" = (Join-Path $runDirectory "FHECGenerated.sol")
    "FHECOptimizationBaseline.sol" = (Join-Path $runDirectory "FHECOptimizationBaseline.sol")
    "FHECOptimizationOptimized.sol" = (Join-Path $runDirectory "FHECOptimizationOptimized.sol")
    "FHECManualReference.sol" = (Join-Path $optimizations "FHECManualReference.sol")
    "FHECThreeWayComparison.sol" = (Join-Path $optimizations "FHECThreeWayComparison.sol")
    "FHECPhase4Baseline.sol" = (Join-Path $runDirectory "FHECPhase4Baseline.sol")
    "FHECPhase4Optimized.sol" = (Join-Path $runDirectory "FHECPhase4Optimized.sol")
    "FHECPhase4Manual.sol" = (Join-Path $phase4 "FHECPhase4Manual.sol")
    "FHECPhase4Comparison.sol" = (Join-Path $phase4 "FHECPhase4Comparison.sol")
}

$requiredFiles = @(
    (Join-Path $Project "Cargo.toml"),
    (Join-Path $Project "Cargo.lock"),
    $source,
    $phase3Source,
    $extendedSource,
    $corpus,
    (Join-Path $Project "tools\cofhe\check-vectors.mjs"),
    (Join-Path $Project "tools\cofhe\generate-phase4-artifacts.mjs"),
    (Join-Path $Project "tools\cofhe\check-cost-policy.mjs"),
    (Join-Path $Project "tools\cofhe\check-cost-policy.test.mjs"),
    (Join-Path $Project "tools\cofhe\summarize-phase4.mjs"),
    (Join-Path $Project "tools\cofhe\summarize-phase4.test.mjs"),
    (Join-Path $Project "tools\cofhe\render-phase4-report.mjs"),
    (Join-Path $Project "tools\cofhe\render-phase4-report.test.mjs"),
    (Join-Path $Project "tools\cofhe\verify-phase4-contrast.mjs"),
    (Join-Path $Project "tools\cofhe\verify-phase4-contrast.test.mjs"),
    (Join-Path $hardhat "test\FHECPhase4Differential.test.ts"),
    (Join-Path $hardhat "package.json"),
    (Join-Path $optimizations "FHECManualReference.sol"),
    (Join-Path $optimizations "FHECThreeWayComparison.sol"),
    (Join-Path $phase4 "FHECPhase4Manual.sol"),
    (Join-Path $phase4 "FHECPhase4Comparison.sol")
)

foreach ($required in $requiredFiles) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Missing Phase 4 prerequisite: $required"
    }
}

foreach ($tool in @("cargo", "node", "pnpm")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Phase 4 verification requires $tool"
    }
}

$corpusHash = (Get-FileHash -LiteralPath $corpus -Algorithm SHA256).Hash
if ($corpusHash -ne "B0797E94345C4A5E85CB3DCC62B53A2BB8A21485176D9056181544D04CFAC7F1") {
    throw "Original deterministic vector corpus differs from the reviewed Phase 3 version"
}

New-Item -ItemType Directory -Path $runDirectory -ErrorAction Stop | Out-Null

foreach ($name in $placements.Keys) {
    $destination = Join-Path $hardhat "contracts\$name"
    if ((Test-Path -LiteralPath $destination) -and
        -not (Test-Path -LiteralPath $destination -PathType Leaf)) {
        throw "Expected a Solidity file but found another object: $destination"
    }
    if (Test-Path -LiteralPath $destination -PathType Leaf) {
        $backup = Join-Path $runDirectory "previous-$name"
        Copy-Item -LiteralPath $destination -Destination $backup -ErrorAction Stop
        $backups[$name] = @{
            Path = $backup
            Hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
        }
    }
}

try {
    Start-Transcript -LiteralPath $transcript -ErrorAction Stop | Out-Null
    $recording = $true
    Set-Location -LiteralPath $Project

    node tools/cofhe/generate-phase4-artifacts.mjs --check
    if ($LASTEXITCODE -ne 0) { throw "Generated Phase 4 source integrity check failed" }

    node tools/cofhe/check-vectors.mjs $corpus
    if ($LASTEXITCODE -ne 0) { throw "Offline vector oracle validation failed" }

    node --test tools/cofhe/check-cost-policy.test.mjs tools/cofhe/summarize-phase4.test.mjs tools/cofhe/render-phase4-report.test.mjs tools/cofhe/verify-phase4-contrast.test.mjs
    if ($LASTEXITCODE -ne 0) { throw "Cost-policy or benchmark-summary tests failed" }

    if (-not $SkipRustTests) {
        cargo fmt --all -- --check
        if ($LASTEXITCODE -ne 0) { throw "Rustfmt validation failed" }

        cargo test --locked -p fhec-backend-cofhe -p fhec-cli
        if ($LASTEXITCODE -ne 0) { throw "Rust backend or CLI regression tests failed" }

        cargo clippy --locked -p fhec-backend-cofhe -p fhec-cli --all-targets -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE clippy validation failed" }
    }

    cargo run --locked -p fhec-cli -- cofhe $source --out (Join-Path $runDirectory "FHECPhase4Baseline.sol") --library FHECPhase4Baseline
    if ($LASTEXITCODE -ne 0) { throw "Phase 4 baseline code generation failed" }

    cargo run --locked -p fhec-cli -- cofhe $source --out (Join-Path $runDirectory "FHECPhase4Optimized.sol") --library FHECPhase4Optimized --optimize --report $reportPath
    if ($LASTEXITCODE -ne 0) { throw "Phase 4 optimized code generation failed" }

    cargo run --locked -p fhec-cli -- cofhe $extendedSource --out (Join-Path $runDirectory "FHECGenerated.sol")
    if ($LASTEXITCODE -ne 0) { throw "Extended library generation failed" }

    cargo run --locked -p fhec-cli -- cofhe $phase3Source --out (Join-Path $runDirectory "FHECOptimizationBaseline.sol") --library FHECOptimizationBaseline
    if ($LASTEXITCODE -ne 0) { throw "Phase 3 baseline regeneration failed" }

    cargo run --locked -p fhec-cli -- cofhe $phase3Source --out (Join-Path $runDirectory "FHECOptimizationOptimized.sol") --library FHECOptimizationOptimized --optimize
    if ($LASTEXITCODE -ne 0) { throw "Phase 3 optimized library regeneration failed" }

    $report = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
    if ($report.schema -ne "fhec-cofhe-static-optimization-v1" -or
        @($report.call_graph).Count -ne 26) {
        throw "Phase 4 static report schema or function count differs from the intended 26-function suite"
    }

    node tools/cofhe/check-cost-policy.mjs --report $reportPath --entry repeated_u8 --mode optimized --max-ops 2 --max-calls 0 --max-depth 0 --out $budgetPath
    if ($LASTEXITCODE -ne 0) { throw "Expected optimizer budget acceptance failed" }

    node tools/cofhe/verify-phase4-contrast.mjs --report $reportPath --out $contrastPath
    if ($LASTEXITCODE -ne 0) { throw "Baseline-versus-optimized budget contrast failed" }

    foreach ($name in $placements.Keys) {
        $destination = Join-Path $hardhat "contracts\$name"
        $staged += $name
        Copy-Item -LiteralPath $placements[$name] -Destination $destination -Force -ErrorAction Stop
    }

    Set-Location -LiteralPath $hardhat
    if (-not $SkipDependencyInstall) {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "Pinned Hardhat dependency installation failed" }
    }

    pnpm compile
    if ($LASTEXITCODE -ne 0) { throw "Phase 4 Solidity compilation failed" }

    [Environment]::SetEnvironmentVariable("FHEC_PHASE4_CASES_PER_FAMILY", $CasesPerFamily.ToString(), "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE4_EVIDENCE_FILE", $evidencePath, "Process")

    pnpm exec hardhat test test/FHECPhase4Differential.test.ts
    if ($LASTEXITCODE -ne 0) { throw "Phase 4 differential Hardhat mock tests failed" }

    Set-Location -LiteralPath $Project
    node tools/cofhe/summarize-phase4.mjs --input $evidencePath --out $summaryPath
    if ($LASTEXITCODE -ne 0) { throw "Phase 4 independent gas evidence aggregation failed" }

    $summary = Get-Content -LiteralPath $summaryPath -Raw | ConvertFrom-Json
    if ($summary.total_cases -ne (24 * $CasesPerFamily) -or @($summary.families).Count -ne 24) {
        throw "Phase 4 mock summary is incomplete"
    }

    node tools/cofhe/render-phase4-report.mjs --input $summaryPath --out $htmlPath
    if ($LASTEXITCODE -ne 0) { throw "Phase 4 self-contained evidence HTML generation failed" }

    $passed = $true
}
finally {
    Set-Location -LiteralPath $originalLocation
    [Environment]::SetEnvironmentVariable("FHEC_PHASE4_CASES_PER_FAMILY", $previousCases, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE4_EVIDENCE_FILE", $previousEvidence, "Process")
    try {
        foreach ($name in $staged) {
            $destination = Join-Path $hardhat "contracts\$name"
            if ($backups.ContainsKey($name)) {
                Copy-Item -LiteralPath $backups[$name].Path -Destination $destination -Force -ErrorAction Stop
                if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -ne $backups[$name].Hash) {
                    throw "Failed to restore original Solidity source: $name"
                }
            } elseif (Test-Path -LiteralPath $destination -PathType Leaf) {
                Remove-Item -LiteralPath $destination -Force -ErrorAction Stop
            }
        }
    }
    finally {
        if ($recording) { Stop-Transcript | Out-Null }
    }
}

if ($passed) {
    $validation = [ordered]@{
        schema = "fhec-cofhe-phase4-local-validation-v1"
        generated_utc = [DateTime]::UtcNow.ToString("o")
        environment = "Fhenix Hardhat CoFHE MOCK"
        source_sha256 = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
        corpus_sha256 = $corpusHash
        baseline_solidity_sha256 = (Get-FileHash -LiteralPath (Join-Path $runDirectory "FHECPhase4Baseline.sol") -Algorithm SHA256).Hash
        optimized_solidity_sha256 = (Get-FileHash -LiteralPath (Join-Path $runDirectory "FHECPhase4Optimized.sol") -Algorithm SHA256).Hash
        manual_solidity_sha256 = (Get-FileHash -LiteralPath (Join-Path $phase4 "FHECPhase4Manual.sol") -Algorithm SHA256).Hash
        cost_report_sha256 = (Get-FileHash -LiteralPath $reportPath -Algorithm SHA256).Hash
        budget_decision_sha256 = (Get-FileHash -LiteralPath $budgetPath -Algorithm SHA256).Hash
        budget_contrast_sha256 = (Get-FileHash -LiteralPath $contrastPath -Algorithm SHA256).Hash
        raw_mock_samples_sha256 = (Get-FileHash -LiteralPath $evidencePath -Algorithm SHA256).Hash
        mock_summary_sha256 = (Get-FileHash -LiteralPath $summaryPath -Algorithm SHA256).Hash
        mock_html_report_sha256 = (Get-FileHash -LiteralPath $htmlPath -Algorithm SHA256).Hash
        family_count = 24
        cases_per_family = $CasesPerFamily
        completed_mock_comparisons = 24 * $CasesPerFamily
        three_way_oracle_mock_tests = "passed"
        signer_contract_proof_acl_negative_tests = "passed"
        contract_sources_restored = $true
        real_tfhe_execution = "not checked by this script"
        live_cofhe_execution = "not tested"
        real_network_gas = "not measured"
    }
    $validation | ConvertTo-Json -Depth 5 |
        Set-Content -LiteralPath $validationPath -Encoding UTF8 -ErrorAction Stop
    Write-Host "PHASE 4 LOCAL COFHE VERIFICATION PASSED" -ForegroundColor Green
    Write-Host "Validation: $validationPath"
    Write-Host "Mock samples: $evidencePath"
    Write-Host "Mock gas summary: $summaryPath"
    Write-Host "Shareable mock report: $htmlPath"
    Write-Host "Transcript: $transcript"
}
