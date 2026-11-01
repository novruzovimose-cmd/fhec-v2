param(
    [string]$Project = (Join-Path $PSScriptRoot "..\.."),
    [switch]$SkipDependencyInstall,
    [switch]$FullWorkspace
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$hardhat = Join-Path $Project "integration\cofhe-hardhat"
$source = Join-Path $Project "examples\cofhe_optimization.fhe"
$extended = Join-Path $Project "examples\cofhe_extended.fhe"
$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
$runDirectory = Join-Path $Project "target\cofhe-optimization-$runId"

foreach ($required in @(
    (Join-Path $Project "Cargo.lock"),
    $source,
    $extended,
    (Join-Path $hardhat "package.json"),
    (Join-Path $hardhat "pnpm-lock.yaml"),
    (Join-Path $hardhat "optimization\FHECOptimizationComparison.sol"),
    (Join-Path $hardhat "test\FHECOptimizationComparison.test.ts")
)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Missing optimization verification input: $required"
    }
}

foreach ($toolName in @("cargo", "pnpm")) {
    if (-not (Get-Command $toolName -ErrorAction SilentlyContinue)) {
        throw "Missing required tool: $toolName"
    }
}

New-Item -ItemType Directory -Path $runDirectory -ErrorAction Stop | Out-Null

$baseline = Join-Path $runDirectory "FHECOptimizationBaseline.sol"
$optimized = Join-Path $runDirectory "FHECOptimizationOptimized.sol"
$generated = Join-Path $runDirectory "FHECGenerated.sol"
$baselineReport = Join-Path $runDirectory "baseline-static-report.json"
$optimizedReport = Join-Path $runDirectory "optimized-static-report.json"
$evidencePath = Join-Path $runDirectory "validation-report.json"
$transcript = Join-Path $runDirectory "validation-transcript.txt"

$files = @("FHECGenerated.sol", "FHECOptimizationBaseline.sol", "FHECOptimizationOptimized.sol", "FHECOptimizationComparison.sol")
$previous = @{}
$placed = @()
$originalLocation = (Get-Location).Path
$recording = $false
$passed = $false

foreach ($name in $files) {
    $destination = Join-Path $hardhat "contracts\$name"
    if ((Test-Path -LiteralPath $destination) -and
        -not (Test-Path -LiteralPath $destination -PathType Leaf)) {
        throw "Expected a file at $destination"
    }
    if (Test-Path -LiteralPath $destination -PathType Leaf) {
        $backup = Join-Path $runDirectory "previous-$name"
        Copy-Item -LiteralPath $destination -Destination $backup -ErrorAction Stop
        $previous[$name] = @{
            Backup = $backup
            Sha256 = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
        }
    }
}

try {
    Start-Transcript -LiteralPath $transcript -ErrorAction Stop | Out-Null
    $recording = $true
    Set-Location -LiteralPath $Project

    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "Rust formatting failed" }

    cargo test --locked -p fhec-backend-cofhe -p fhec-cli
    if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE tests failed" }

    cargo clippy --locked -p fhec-backend-cofhe -p fhec-cli --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE lint failed" }

    if ($FullWorkspace) {
        cargo test --locked --workspace
        if ($LASTEXITCODE -ne 0) { throw "Full workspace Rust tests failed" }
    }

    cargo run --locked -p fhec-cli -- cofhe $source --out $baseline --library FHECOptimizationBaseline --report $baselineReport
    if ($LASTEXITCODE -ne 0) { throw "Baseline Solidity generation failed" }

    cargo run --locked -p fhec-cli -- cofhe $source --out $optimized --library FHECOptimizationOptimized --optimize --report $optimizedReport
    if ($LASTEXITCODE -ne 0) { throw "Optimized Solidity generation failed" }

    cargo run --locked -p fhec-cli -- cofhe $extended --out $generated
    if ($LASTEXITCODE -ne 0) { throw "Reference Solidity generation failed" }

    $report = Get-Content -LiteralPath $optimizedReport -Raw | ConvertFrom-Json
    if ($report.schema -ne "fhec-cofhe-static-optimization-v1" -or
        $report.emitted_variant -ne "optimized" -or
        $report.baseline_totals.encrypted_operations -ne 6 -or
        $report.optimized_totals.encrypted_operations -ne 2 -or
        $report.eliminated_encrypted_operations -ne 4) {
        throw "Static compiler optimization report does not match the demonstration"
    }

    $sources = @{
        "FHECGenerated.sol" = $generated
        "FHECOptimizationBaseline.sol" = $baseline
        "FHECOptimizationOptimized.sol" = $optimized
        "FHECOptimizationComparison.sol" = (Join-Path $hardhat "optimization\FHECOptimizationComparison.sol")
    }
    foreach ($name in $files) {
        $destination = Join-Path $hardhat "contracts\$name"
        $placed += $name
        Copy-Item -LiteralPath $sources[$name] -Destination $destination -Force -ErrorAction Stop
    }

    Set-Location -LiteralPath $hardhat
    if (-not $SkipDependencyInstall) {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "Pinned Hardhat dependency installation failed" }
    }

    pnpm compile
    if ($LASTEXITCODE -ne 0) { throw "Generated Solidity compilation failed" }

    pnpm run test:comparison
    if ($LASTEXITCODE -ne 0) { throw "Original CoFHE comparison tests failed" }

    pnpm run test:extended
    if ($LASTEXITCODE -ne 0) { throw "Original extended CoFHE tests failed" }

    pnpm exec hardhat test test/FHECOptimizationComparison.test.ts
    if ($LASTEXITCODE -ne 0) { throw "Differential CoFHE mock execution tests failed" }

    $passed = $true
}
finally {
    Set-Location -LiteralPath $originalLocation
    try {
        foreach ($name in $placed) {
            $destination = Join-Path $hardhat "contracts\$name"
            if ($previous.ContainsKey($name)) {
                Copy-Item -LiteralPath $previous[$name].Backup -Destination $destination -Force -ErrorAction Stop
                $actual = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
                if ($actual -ne $previous[$name].Sha256) {
                    throw "Failed to restore original source hash for $name"
                }
            } else {
                Remove-Item -LiteralPath $destination -Force -ErrorAction Stop
            }
        }
    }
    finally {
        if ($recording) { Stop-Transcript | Out-Null }
    }
}

if ($passed) {
    $evidence = [ordered]@{
        schema = "fhec-cofhe-optimization-validation-v1"
        timestamp_utc = [DateTime]::UtcNow.ToString("o")
        execution_environment = "Fhenix CoFHE Hardhat local mocks"
        source_sha256 = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
        baseline_sha256 = (Get-FileHash -LiteralPath $baseline -Algorithm SHA256).Hash
        optimized_sha256 = (Get-FileHash -LiteralPath $optimized -Algorithm SHA256).Hash
        baseline_encrypted_operations = 6
        optimized_encrypted_operations = 2
        static_operations_eliminated = 4
        local_mock_differential_tests = "passed"
        existing_mock_regressions = "passed"
        rust_backend_and_cli_tests = "passed"
        rust_clippy = "passed"
        generated_libraries_restored = $true
        public_testnet_execution = "not tested"
        real_network_gas = "not measured"
    }
    $evidence | ConvertTo-Json -Depth 5 |
        Set-Content -LiteralPath $evidencePath -Encoding UTF8 -ErrorAction Stop

    Write-Host "FHEC COFHE OPTIMIZATION DIFFERENTIAL DEMO VERIFIED" -ForegroundColor Green
    Write-Host "Baseline: $baseline"
    Write-Host "Optimized: $optimized"
    Write-Host "Operation report: $optimizedReport"
    Write-Host "Evidence: $evidencePath"
    Write-Host "Transcript: $transcript"
}
