param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [switch]$SkipDependencyInstall,
    [switch]$FullWorkspace,
    [ValidateRange(1,128)][int]$CaseCount = 24
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$hardhat = Join-Path $Project "integration\cofhe-hardhat"
$source = Join-Path $Project "examples\cofhe_optimization.fhe"
$extendedSource = Join-Path $Project "examples\cofhe_extended.fhe"
$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
$runDirectory = Join-Path $Project "target\cofhe-phase3-$runId"
$corpus = Join-Path $hardhat "testdata\cofhe-vectors.jsonl"
$recording = $false
$originalLocation = (Get-Location).Path
$previousCaseCount = [Environment]::GetEnvironmentVariable("FHEC_PHASE3_CASES", "Process")
$passed = $false

foreach ($file in @(
    $source,
    $extendedSource,
    $corpus,
    (Join-Path $Project "Cargo.lock"),
    (Join-Path $Project "tools\cofhe\check-vectors.mjs"),
    (Join-Path $hardhat "optimization\FHECManualReference.sol"),
    (Join-Path $hardhat "optimization\FHECThreeWayComparison.sol"),
    (Join-Path $hardhat "test\FHECThreeWayComparison.test.ts"),
    (Join-Path $hardhat "package.json")
)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Required Phase 3 file is missing: $file"
    }
}

foreach ($tool in @("cargo", "pnpm", "node")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Phase 3 validation requires $tool"
    }
}

$vectorsHash = (Get-FileHash -LiteralPath $corpus -Algorithm SHA256).Hash
if ($vectorsHash -ne "B0797E94345C4A5E85CB3DCC62B53A2BB8A21485176D9056181544D04CFAC7F1") {
    throw "Corpus changed from the reviewed deterministic vectors; no source files changed"
}

New-Item -ItemType Directory -Path $runDirectory -ErrorAction Stop | Out-Null

$baseline = Join-Path $runDirectory "FHECOptimizationBaseline.sol"
$optimized = Join-Path $runDirectory "FHECOptimizationOptimized.sol"
$generated = Join-Path $runDirectory "FHECGenerated.sol"
$report = Join-Path $runDirectory "optimized-static-report.json"
$evidence = Join-Path $runDirectory "validation-report.json"
$transcript = Join-Path $runDirectory "validation-transcript.txt"
$placements = [ordered]@{
    "FHECGenerated.sol" = $generated
    "FHECOptimizationBaseline.sol" = $baseline
    "FHECOptimizationOptimized.sol" = $optimized
    "FHECManualReference.sol" = (Join-Path $hardhat "optimization\FHECManualReference.sol")
    "FHECThreeWayComparison.sol" = (Join-Path $hardhat "optimization\FHECThreeWayComparison.sol")
}
$backups = @{}
$staged = @()

foreach ($name in $placements.Keys) {
    $path = Join-Path $hardhat "contracts\$name"
    if ((Test-Path -LiteralPath $path) -and -not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Expected a contract source file: $path"
    }
    if (Test-Path -LiteralPath $path -PathType Leaf) {
        $backup = Join-Path $runDirectory "previous-$name"
        Copy-Item -LiteralPath $path -Destination $backup -ErrorAction Stop
        $backups[$name] = @{
            Path = $backup
            Hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
        }
    }
}

try {
    Start-Transcript -LiteralPath $transcript -ErrorAction Stop | Out-Null
    $recording = $true
    Set-Location -LiteralPath $Project

    node tools/cofhe/check-vectors.mjs $corpus
    if ($LASTEXITCODE -ne 0) { throw "Offline test-vector oracle validation failed" }

    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "Rust source format check failed" }

    cargo test --locked -p fhec-backend-cofhe -p fhec-cli
    if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE and CLI tests failed" }

    cargo clippy --locked -p fhec-backend-cofhe -p fhec-cli --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE and CLI lint failed" }

    if ($FullWorkspace) {
        cargo test --locked --workspace
        if ($LASTEXITCODE -ne 0) { throw "Full workspace Rust tests failed" }
    }

    cargo run --locked -p fhec-cli -- cofhe $source --out $baseline --library FHECOptimizationBaseline
    if ($LASTEXITCODE -ne 0) { throw "Baseline Solidity generation failed" }

    cargo run --locked -p fhec-cli -- cofhe $source --out $optimized --library FHECOptimizationOptimized --optimize --report $report
    if ($LASTEXITCODE -ne 0) { throw "Optimized Solidity generation failed" }

    cargo run --locked -p fhec-cli -- cofhe $extendedSource --out $generated
    if ($LASTEXITCODE -ne 0) { throw "Shared reference Solidity generation failed" }

    $parsed = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
    if ($parsed.schema -ne "fhec-cofhe-static-optimization-v1" -or
        $parsed.baseline_totals.encrypted_operations -ne 6 -or
        $parsed.optimized_totals.encrypted_operations -ne 2 -or
        @($parsed.call_graph).Count -ne 4) {
        throw "Unexpected optimization and call-graph report structure"
    }

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
    if ($LASTEXITCODE -ne 0) { throw "Three-way Solidity compilation failed" }

    [Environment]::SetEnvironmentVariable("FHEC_PHASE3_CASES", $CaseCount.ToString(), "Process")
    pnpm exec hardhat test test/FHECThreeWayComparison.test.ts
    if ($LASTEXITCODE -ne 0) { throw "Three-way Hardhat mock differential tests failed" }

    $passed = $true
}
finally {
    Set-Location -LiteralPath $originalLocation
    [Environment]::SetEnvironmentVariable("FHEC_PHASE3_CASES", $previousCaseCount, "Process")
    try {
        foreach ($name in $staged) {
            $destination = Join-Path $hardhat "contracts\$name"
            if ($backups.ContainsKey($name)) {
                Copy-Item -LiteralPath $backups[$name].Path -Destination $destination -Force -ErrorAction Stop
                $actual = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
                if ($actual -ne $backups[$name].Hash) {
                    throw "Failed to restore original contract source hash: $name"
                }
            } else {
                if (Test-Path -LiteralPath $destination -PathType Leaf) {
                    Remove-Item -LiteralPath $destination -Force -ErrorAction Stop
                }
            }
        }
    }
    finally {
        if ($recording) { Stop-Transcript | Out-Null }
    }
}

if ($passed) {
    $validation = [ordered]@{
        schema = "fhec-cofhe-phase3-local-validation-v1"
        generated_utc = [DateTime]::UtcNow.ToString("o")
        environment = "Fhenix Hardhat CoFHE mocks"
        source_sha256 = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
        test_vector_sha256 = $vectorsHash
        baseline_solidity_sha256 = (Get-FileHash -LiteralPath $baseline -Algorithm SHA256).Hash
        optimized_solidity_sha256 = (Get-FileHash -LiteralPath $optimized -Algorithm SHA256).Hash
        optimized_static_report_sha256 = (Get-FileHash -LiteralPath $report -Algorithm SHA256).Hash
        baseline_static_operations = 6
        optimized_static_operations = 2
        function_call_graph_report = "generated"
        cases = $CaseCount
        three_way_mock_oracle_tests = "passed"
        baseline_and_reference_contracts_restored = $true
        rust_tests_and_clippy = "passed"
        real_testnet_execution = "not run"
        live_network_gas = "not measured"
    }
    $validation | ConvertTo-Json -Depth 5 |
        Set-Content -LiteralPath $evidence -Encoding UTF8 -ErrorAction Stop
    Write-Host "FHEC PHASE 3 LOCAL VALIDATION PASSED" -ForegroundColor Green
    Write-Host "Evidence: $evidence"
    Write-Host "Transcript: $transcript"
    Write-Host "Optimized call-graph report: $report"
}
