param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [ValidateRange(1,28)][int]$CasesPerFamily = 2,
    [switch]$SkipDependencyInstall,
    [switch]$SkipRustTests,
    [string]$VerifyLiveTrace,
    [ValidateRange(1,64)][int]$MinimumConfirmations = 2
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
if ((Split-Path -Path $Project -Leaf) -ne "fhec-v2") {
    throw "Phase 7 verification requires the dedicated fhec-v2 project"
}
if ($VerifyLiveTrace) {
    if (-not (Test-Path -LiteralPath $VerifyLiveTrace -PathType Leaf)) {
        throw "Optional Phase 6 testnet trace does not exist"
    }
    if ($env:FHEC_TESTNET_RPC_URL -notmatch '^https://') {
        throw "Read-only testnet verification requires FHEC_TESTNET_RPC_URL with HTTPS"
    }
}

$cofhe = Join-Path $Project "tools\cofhe"
$hardhat = Join-Path $Project "integration\cofhe-hardhat"
$phase7 = Join-Path $hardhat "phase7"
$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0,8)
$out = Join-Path $Project "target\cofhe-phase7-$runId"
$generated = Join-Path $phase7 "generated\$runId"
$source = Join-Path $Project "examples\cofhe_phase4.fhe"
$matrix = Join-Path $generated "phase7-matrix.json"
$raw = Join-Path $out "three-way-mock-raw.json"
$summary = Join-Path $out "three-way-verified-summary.json"
$html = Join-Path $out "phase7-mock-evidence-report.html"
$receipts = Join-Path $out "read-only-testnet-rpc-corroboration.json"
$tests = @(
    (Join-Path $cofhe "phase7\generate-manual.test.mjs"),
    (Join-Path $cofhe "phase7\metrics.test.mjs"),
    (Join-Path $cofhe "phase7\verify-receipts.test.mjs"),
    (Join-Path $cofhe "phase7\report-html.test.mjs"),
    (Join-Path $cofhe "phase7\verify-fixtures.test.mjs")
)
$required = @(
    (Join-Path $Project "Cargo.toml"),
    (Join-Path $hardhat "pnpm-lock.yaml"),
    (Join-Path $hardhat "package.json"),
    $source,
    (Join-Path $cofhe "phase6\oracle.mjs"),
    (Join-Path $cofhe "phase6\generate-solidity.mjs"),
    (Join-Path $cofhe "phase6\validate-results.mjs"),
    (Join-Path $cofhe "phase7\generate-manual.mjs"),
    (Join-Path $cofhe "phase7\metrics.mjs"),
    (Join-Path $cofhe "phase7\verify-receipts.mjs"),
    (Join-Path $cofhe "phase7\report-html.mjs"),
    (Join-Path $cofhe "phase7\verify-fixtures.mjs"),
    (Join-Path $cofhe "phase7\golden-benchmark-384-v1.json"),
    (Join-Path $cofhe "phase7\golden-stress-672-v1.json"),
    (Join-Path $phase7 "hardhat.phase7.config.ts"),
    (Join-Path $phase7 "test\FHECPhase7Triplet.test.ts")
) + $tests
foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing Phase 7 prerequisite: $path"
    }
}
foreach ($tool in @("node", "pnpm", "cargo")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Required tool unavailable: $tool"
    }
}
if (Test-Path -LiteralPath $out) {
    throw "Refusing to reuse Phase 7 evidence directory: $out"
}
if (Test-Path -LiteralPath $generated) {
    throw "Refusing to reuse Phase 7 generated directory: $generated"
}

$priorPath = (Get-Location).Path
$envNames = @("FHEC_PHASE7_MATRIX_FILE", "FHEC_PHASE7_MOCK_EVIDENCE_FILE", "FHEC_PHASE7_SOURCES_DIR")
$previous = @{}
foreach ($name in $envNames) {
    $previous[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
$success = $false
$receiptPath = ""
New-Item -ItemType Directory -Path $out -Force -ErrorAction Stop | Out-Null
New-Item -ItemType Directory -Path $generated -Force -ErrorAction Stop | Out-Null

try {
    Set-Location -LiteralPath $Project
    node --test @tests
    if ($LASTEXITCODE -ne 0) { throw "Independent Phase 7 Node unit tests failed" }
    node tools/cofhe/phase7/verify-fixtures.mjs --directory tools/cofhe/phase7
    if ($LASTEXITCODE -ne 0) { throw "Phase 7 canonical 384/672-case fixture verification failed" }

    if (-not $SkipRustTests) {
        cargo fmt --all -- --check
        if ($LASTEXITCODE -ne 0) { throw "Rust formatting regression" }
        cargo test --locked -p fhec-backend-cofhe -p fhec-cli
        if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE regression" }
        cargo clippy --locked -p fhec-backend-cofhe -p fhec-cli --all-targets -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE clippy regression" }
    }

    cargo run --locked -p fhec-cli -- cofhe $source --out (Join-Path $generated "FHECPhase6Optimized.sol") --library FHECPhase6Optimized --optimize --report (Join-Path $out "fhec-optimized-static-report.json")
    if ($LASTEXITCODE -ne 0) { throw "Rust compiler failed to emit optimized CoFHE Solidity" }

    node tools/cofhe/phase7/generate-manual.mjs --directory $generated
    if ($LASTEXITCODE -ne 0) { throw "Independent Solidity reference generation failed" }

    node tools/cofhe/phase6/oracle.mjs --per-family $CasesPerFamily --out $matrix
    if ($LASTEXITCODE -ne 0) { throw "Deterministic oracle matrix generation failed" }

    foreach ($name in @("FHECPhase6Optimized.sol", "FHECPhase6Live.sol", "FHECPhase7ManualStraight.sol", "FHECPhase7ManualTuned.sol", "FHECPhase7Straight.sol", "FHECPhase7Tuned.sol", "phase7-matrix.json")) {
        $path = Join-Path $generated $name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Generated Solidity or matrix missing: $name"
        }
    }

    [Environment]::SetEnvironmentVariable("FHEC_PHASE7_MATRIX_FILE", $matrix, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE7_MOCK_EVIDENCE_FILE", $raw, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE7_SOURCES_DIR", $generated, "Process")

    Set-Location -LiteralPath $hardhat
    if (-not $SkipDependencyInstall) {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "Frozen CoFHE dependency installation failed" }
    }
    pnpm exec hardhat compile --config phase7/hardhat.phase7.config.ts
    if ($LASTEXITCODE -ne 0) { throw "Phase 7 three-way Solidity compilation failed" }
    pnpm exec hardhat test --config phase7/hardhat.phase7.config.ts phase7/test/FHECPhase7Triplet.test.ts
    if ($LASTEXITCODE -ne 0) { throw "Three-way encrypted Hardhat mock comparison failed" }

    Set-Location -LiteralPath $Project
    node tools/cofhe/phase7/metrics.mjs --matrix $matrix --raw $raw --out $summary
    if ($LASTEXITCODE -ne 0) { throw "Independent receipt/matrix/gas accounting failed" }

    if ($VerifyLiveTrace) {
        node tools/cofhe/phase7/verify-receipts.mjs --trace $VerifyLiveTrace --out $receipts --confirmations $MinimumConfirmations
        if ($LASTEXITCODE -ne 0) { throw "Read-only Arbitrum Sepolia receipt corroboration failed" }
        $receiptPath = $receipts
    }

    if ($receiptPath) {
        node tools/cofhe/phase7/report-html.mjs --summary $summary --out $html --receipts $receiptPath
    } else {
        node tools/cofhe/phase7/report-html.mjs --summary $summary --out $html
    }
    if ($LASTEXITCODE -ne 0) { throw "Independent HTML evidence report rendering failed" }

    $provenance = [ordered]@{
        schema = "fhec-phase7-local-provenance-v1"
        evidence_tier = "local-mock; not live CoFHE gas"
        cases = (24 * $CasesPerFamily)
        evaluated_transactions = (72 * $CasesPerFamily)
        runtime = "Hardhat local mocks"
        rust_generated_solidity_sha256 = (Get-FileHash -LiteralPath (Join-Path $generated "FHECPhase6Optimized.sol") -Algorithm SHA256).Hash
        source_matrix_sha256 = (Get-FileHash -LiteralPath $matrix -Algorithm SHA256).Hash
        raw_evidence_sha256 = (Get-FileHash -LiteralPath $raw -Algorithm SHA256).Hash
        independent_summary_sha256 = (Get-FileHash -LiteralPath $summary -Algorithm SHA256).Hash
        report_sha256 = (Get-FileHash -LiteralPath $html -Algorithm SHA256).Hash
        read_only_rpc = [bool]$receiptPath
        source_directory = $generated
    }
    $json = $provenance | ConvertTo-Json -Depth 6
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText((Join-Path $out "provenance.json"), $json + [Environment]::NewLine, $utf8)
    $success = $true
}
finally {
    Set-Location -LiteralPath $priorPath
    foreach ($name in $envNames) {
        [Environment]::SetEnvironmentVariable($name, $previous[$name], "Process")
    }
}

if ($success) {
    Write-Host "PHASE 7 THREE-WAY MOCK VERIFICATION PASSED" -ForegroundColor Green
    Write-Host "Evidence: $out"
    Write-Host "Rust-generated library: $(Join-Path $generated 'FHECPhase6Optimized.sol')"
    Write-Host "Encrypted comparisons: $(24 * $CasesPerFamily), three-way mock receipts: $(72 * $CasesPerFamily)"
    if ($receiptPath) {
        Write-Host "Separate Arbitrum Sepolia RPC receipts corroborated; SDK decryption not independently attested." -ForegroundColor Yellow
    } else {
        Write-Host "No transactions sent to any network. Gas values are from local Hardhat mocks only." -ForegroundColor Yellow
    }
}
