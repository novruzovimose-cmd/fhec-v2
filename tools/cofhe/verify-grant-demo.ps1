param(
    [string]$Project = (Join-Path $PSScriptRoot "..\.."),
    [switch]$FullWorkspace,
    [switch]$SkipDependencyInstall
)

$ErrorActionPreference = "Stop"

$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$reference = Join-Path $Project "integration\cofhe-hardhat"
$source = Join-Path $Project "examples\cofhe_extended.fhe"
$generatedInReference = Join-Path $reference "contracts\FHECGenerated.sol"
$lockfile = Join-Path $reference "pnpm-lock.yaml"

foreach ($required in @(
    (Join-Path $Project "Cargo.toml"),
    (Join-Path $Project "Cargo.lock"),
    $source,
    $lockfile,
    (Join-Path $reference "hardhat.config.ts"),
    (Join-Path $reference "test\FHECComparisonDemo.test.ts"),
    (Join-Path $reference "test\FHECExtendedDemo.test.ts")
)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "FHEC grant demonstration input missing: $required"
    }
}

foreach ($toolName in @("cargo", "pnpm")) {
    if (-not (Get-Command $toolName -ErrorAction SilentlyContinue)) {
        throw "Required executable unavailable: $toolName"
    }
}

$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
$runDirectory = Join-Path $Project "target\cofhe-grant-demo-$runId"
New-Item -ItemType Directory -Path $runDirectory -ErrorAction Stop | Out-Null

$generated = Join-Path $runDirectory "FHECGenerated.sol"
$backup = Join-Path $runDirectory "FHECGenerated.previous.sol"
$report = Join-Path $runDirectory "validation-report.json"
$transcript = Join-Path $runDirectory "validation-transcript.txt"

$originalLocation = (Get-Location).Path
$hadGenerated = Test-Path -LiteralPath $generatedInReference -PathType Leaf
$originalHash = $null
$replaced = $false
$recording = $false
$passed = $false

if ((Test-Path -LiteralPath $generatedInReference) -and -not $hadGenerated) {
    throw "Generated Solidity path exists but is not a file: $generatedInReference"
}

if ($hadGenerated) {
    Copy-Item -LiteralPath $generatedInReference -Destination $backup -ErrorAction Stop
    $originalHash = (Get-FileHash -LiteralPath $backup -Algorithm SHA256).Hash
}

try {
    Start-Transcript -LiteralPath $transcript -ErrorAction Stop | Out-Null
    $recording = $true

    Set-Location -LiteralPath $Project

    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "Rust formatting check failed" }

    cargo test --locked -p fhec-backend-cofhe -p fhec-cli
    if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE backend / CLI tests failed" }

    cargo clippy --locked -p fhec-backend-cofhe -p fhec-cli --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE lint verification failed" }

    if ($FullWorkspace) {
        cargo test --locked --workspace
        if ($LASTEXITCODE -ne 0) { throw "Full Rust workspace regression tests failed" }

        cargo clippy --locked --workspace --all-targets -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw "Full Rust workspace lint verification failed" }
    }

    cargo run --locked -p fhec-cli -- cofhe $source --out $generated
    if ($LASTEXITCODE -ne 0) { throw "FHEC CoFHE code generation failed" }

    if (-not (Test-Path -LiteralPath $generated -PathType Leaf)) {
        throw "FHEC did not produce the expected Solidity file"
    }

    $content = Get-Content -LiteralPath $generated -Raw
    foreach ($expected in @(
        "function pick_greater(",
        "function add_u16(",
        "function multiply_u32(",
        "function greater_u64(",
        "function four_times_u8("
    )) {
        if (-not $content.Contains($expected)) {
            throw "Generated Solidity omitted required function: $expected"
        }
    }

    Set-Location -LiteralPath $reference

    if (-not $SkipDependencyInstall) {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "Pinned CoFHE dependency installation failed" }
    }

    $replaced = $true
    Copy-Item -LiteralPath $generated -Destination $generatedInReference -Force -ErrorAction Stop

    pnpm compile
    if ($LASTEXITCODE -ne 0) { throw "Generated CoFHE Solidity failed to compile" }

    pnpm run test:comparison
    if ($LASTEXITCODE -ne 0) { throw "Encrypted comparison mock tests failed" }

    pnpm run test:extended
    if ($LASTEXITCODE -ne 0) { throw "Extended CoFHE mock/security tests failed" }

    $passed = $true
}
finally {
    Set-Location -LiteralPath $originalLocation

    try {
        if ($replaced) {
            if ($hadGenerated) {
                Copy-Item -LiteralPath $backup -Destination $generatedInReference -Force -ErrorAction Stop
                $restoredHash = (Get-FileHash -LiteralPath $generatedInReference -Algorithm SHA256).Hash
                if ($restoredHash -ne $originalHash) {
                    throw "Original generated Solidity was not restored exactly"
                }
            } elseif (Test-Path -LiteralPath $generatedInReference -PathType Leaf) {
                Remove-Item -LiteralPath $generatedInReference -Force -ErrorAction Stop
            }
        }
    }
    finally {
        if ($recording) {
            Stop-Transcript | Out-Null
        }
    }
}

if ($passed) {
    $evidence = [ordered]@{
        schema = "fhec-v2-cofhe-grant-demo-v1"
        timestamp_utc = [DateTime]::UtcNow.ToString("o")
        mode = "Fhenix Hardhat CoFHE local mock"
        source_sha256 = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
        generated_solidity_sha256 = (Get-FileHash -LiteralPath $generated -Algorithm SHA256).Hash
        pnpm_lock_sha256 = (Get-FileHash -LiteralPath $lockfile -Algorithm SHA256).Hash
        rust_backend_and_cli_tests = "passed"
        rust_clippy = "passed"
        rust_workspace_checked = [bool]$FullWorkspace
        solidity_compile = "passed"
        comparison_mock_tests = "passed"
        extended_mock_security_tests = "passed"
        reference_restored = $true
        public_testnet_execution = "not tested"
        real_network_gas = "not measured"
    }

    $evidence | ConvertTo-Json -Depth 4 |
        Set-Content -LiteralPath $report -Encoding UTF8 -ErrorAction Stop

    Write-Host "FHEC V2 COFHE GRANT DEMO VERIFIED" -ForegroundColor Green
    Write-Host "Report: $report"
    Write-Host "Transcript: $transcript"
    Write-Host "Generated Solidity: $generated"
}
