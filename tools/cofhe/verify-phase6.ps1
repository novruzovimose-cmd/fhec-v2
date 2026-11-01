param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [ValidateRange(1,28)][int]$CasesPerFamily = 2,
    [switch]$SkipRustTests,
    [switch]$SkipDependencyInstall,
    [switch]$Live,
    [switch]$AllowLiveTransactions
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
if ((Split-Path -Path $Project -Leaf) -ne "fhec-v2") {
    throw "Phase 6 verifier requires the dedicated fhec-v2 project"
}
if ($Live -and -not $AllowLiveTransactions) {
    throw "Live testnet deployment requires the explicit AllowLiveTransactions switch"
}
if (-not $Live -and $AllowLiveTransactions) {
    throw "AllowLiveTransactions must only be provided with Live"
}
if ($Live) {
    if ($env:FHEC_TESTNET_RPC_URL -notmatch '^https://') {
        throw "A user-configured HTTPS FHEC_TESTNET_RPC_URL is required"
    }
    if ($env:FHEC_TESTNET_PRIVATE_KEY -notmatch '^0x[a-fA-F0-9]{64}$') {
        throw "A locally configured 32-byte FHEC_TESTNET_PRIVATE_KEY is required"
    }
}

$root = Join-Path $Project "tools\cofhe"
$hardhat = Join-Path $Project "integration\cofhe-hardhat"
$phase6 = Join-Path $hardhat "phase6"
$generated = Join-Path $phase6 "generated"
$tests = @(
    (Join-Path $root "phase6\oracle.test.mjs"),
    (Join-Path $root "phase6\validate-results.test.mjs")
)
$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0,8)
$out = Join-Path $Project "target\cofhe-phase6-$runId"
$source = Join-Path $Project "examples\cofhe_phase4.fhe"
$compiled = Join-Path $generated "FHECPhase6Optimized.sol"
$wrapper = Join-Path $generated "FHECPhase6Live.sol"
$matrix = Join-Path $generated "phase6-matrix.json"
$reports = [ordered]@{
    matrix = $matrix
    rust_solidity = $compiled
    wrapper_solidity = $wrapper
}
$staged = @()
$stageHashes = @{}
$originalDirectory = (Get-Location).Path
$priorMock = [Environment]::GetEnvironmentVariable("FHEC_PHASE6_MOCK_EVIDENCE_FILE", "Process")
$priorMatrix = [Environment]::GetEnvironmentVariable("FHEC_PHASE6_MATRIX_FILE", "Process")
$priorLive = [Environment]::GetEnvironmentVariable("FHEC_PHASE6_LIVE_EVIDENCE_FILE", "Process")
$priorConsent = [Environment]::GetEnvironmentVariable("FHEC_PHASE6_LIVE_ALLOW_SEND", "Process")
$complete = $false

$required = @($source, (Join-Path $Project "Cargo.toml"), (Join-Path $hardhat "pnpm-lock.yaml"),
    (Join-Path $root "phase6\oracle.mjs"),
    (Join-Path $root "phase6\golden-boundary-v1.json"),
    (Join-Path $root "phase6\golden-full-v1.json"),
    (Join-Path $root "phase6\generate-solidity.mjs"),
    (Join-Path $root "phase6\validate-results.mjs"),
    (Join-Path $phase6 "hardhat.phase6.config.ts"),
    (Join-Path $phase6 "test\FHECPhase6Boundary.test.ts"),
    (Join-Path $phase6 "run-live.ts")) + $tests
foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing Phase 6 prerequisite: $path"
    }
}
foreach ($tool in @("node", "cargo", "pnpm")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Missing required tool: $tool" }
}
foreach ($name in $reports.Keys) {
    if (Test-Path -LiteralPath $reports[$name]) {
        throw "Refusing to overwrite preexisting Phase 6 generated output: $($reports[$name])"
    }
}

New-Item -ItemType Directory -Path $out -ErrorAction Stop | Out-Null
if (-not (Test-Path -LiteralPath $generated -PathType Container)) {
    New-Item -ItemType Directory -Path $generated -Force -ErrorAction Stop | Out-Null
}

try {
    Set-Location -LiteralPath $Project
    node --test @tests
    if ($LASTEXITCODE -ne 0) { throw "Phase 6 independent oracle/attestation tests failed" }

    if (-not $SkipRustTests) {
        cargo fmt --all -- --check
        if ($LASTEXITCODE -ne 0) { throw "Rust formatting regression" }
        cargo test --locked -p fhec-backend-cofhe -p fhec-cli
        if ($LASTEXITCODE -ne 0) { throw "Rust compiler regression" }
        cargo clippy --locked -p fhec-backend-cofhe -p fhec-cli --all-targets -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE clippy regression" }
    }

    $staged += $compiled
    cargo run --locked -p fhec-cli -- cofhe $source --out $compiled --library FHECPhase6Optimized --optimize --report (Join-Path $out "static-report.json")
    if ($LASTEXITCODE -ne 0) { throw "Real Rust CoFHE codegen failed" }
    $stageHashes[$compiled] = (Get-FileHash -LiteralPath $compiled -Algorithm SHA256).Hash

    $staged += $wrapper
    node tools/cofhe/phase6/generate-solidity.mjs --out $wrapper
    if ($LASTEXITCODE -ne 0) { throw "Phase 6 Solidity wrapper generation failed" }
    $stageHashes[$wrapper] = (Get-FileHash -LiteralPath $wrapper -Algorithm SHA256).Hash

    $staged += $matrix
    node tools/cofhe/phase6/oracle.mjs --per-family $CasesPerFamily --out $matrix
    if ($LASTEXITCODE -ne 0) { throw "Phase 6 deterministic corpus generation failed" }
    $stageHashes[$matrix] = (Get-FileHash -LiteralPath $matrix -Algorithm SHA256).Hash

    foreach ($name in $reports.Keys) {
        Copy-Item -LiteralPath $reports[$name] -Destination (Join-Path $out ([IO.Path]::GetFileName($reports[$name]))) -ErrorAction Stop
    }

    Set-Location -LiteralPath $hardhat
    if (-not $SkipDependencyInstall) {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "CoFHE Hardhat frozen dependency installation failed" }
    }

    pnpm exec hardhat compile --config phase6/hardhat.phase6.config.ts
    if ($LASTEXITCODE -ne 0) { throw "Phase 6 compiler-generated Solidity failed Hardhat compilation" }

    $rawMock = Join-Path $out "phase6-mock-raw.json"
    [Environment]::SetEnvironmentVariable("FHEC_PHASE6_MOCK_EVIDENCE_FILE", $rawMock, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE6_MATRIX_FILE", $matrix, "Process")

    pnpm exec hardhat test --config phase6/hardhat.phase6.config.ts phase6/test/FHECPhase6Boundary.test.ts
    if ($LASTEXITCODE -ne 0) { throw "Phase 6 Hardhat encrypted boundary/ACL mock tests failed" }

    Set-Location -LiteralPath $Project
    node tools/cofhe/phase6/validate-results.mjs --mock $matrix $rawMock --out (Join-Path $out "verified-phase6-mock.json")
    if ($LASTEXITCODE -ne 0) { throw "Independent Phase 6 mock coverage validation failed" }

    if ($Live) {
        [Environment]::SetEnvironmentVariable("FHEC_PHASE6_LIVE_ALLOW_SEND", "YES_I_ACCEPT_TESTNET_TRANSACTIONS", "Process")
        $liveRaw = Join-Path $out "provisional-live-sdk-trace.json"
        [Environment]::SetEnvironmentVariable("FHEC_PHASE6_LIVE_EVIDENCE_FILE", $liveRaw, "Process")
        Set-Location -LiteralPath $hardhat
        pnpm exec hardhat run --config phase6/hardhat.phase6.config.ts --network arb-sepolia phase6/run-live.ts
        if ($LASTEXITCODE -ne 0) { throw "Opt-in live testnet CoFHE execution failed" }
        Set-Location -LiteralPath $Project
        node tools/cofhe/phase6/validate-results.mjs --live $liveRaw --out (Join-Path $out "provisional-live-check.json")
        if ($LASTEXITCODE -ne 0) { throw "SDK live trace consistency failed" }
    }

    $complete = $true
}
finally {
    Set-Location -LiteralPath $originalDirectory
    [Environment]::SetEnvironmentVariable("FHEC_PHASE6_MOCK_EVIDENCE_FILE", $priorMock, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE6_MATRIX_FILE", $priorMatrix, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE6_LIVE_EVIDENCE_FILE", $priorLive, "Process")
    [Environment]::SetEnvironmentVariable("FHEC_PHASE6_LIVE_ALLOW_SEND", $priorConsent, "Process")

    $preserved = @()
    foreach ($file in $staged) {
        if (Test-Path -LiteralPath $file -PathType Leaf) {
            if ($stageHashes.ContainsKey($file) -and (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -eq $stageHashes[$file]) {
                Remove-Item -LiteralPath $file -Force -ErrorAction Stop
            } else {
                $preserved += $file
            }
        }
    }
    if ($preserved.Count) { Write-Warning "Generated files were modified during validation and preserved: $($preserved -join ', ')" }
}

if ($complete) {
    Write-Host "PHASE 6 LOCAL ENGINEERING VALIDATION PASSED" -ForegroundColor Green
    Write-Host "Results: $out"
    Write-Host "Local matrix: $(24 * $CasesPerFamily) encrypted comparisons plus five negative tests"
    if ($Live) {
        Write-Host "Opt-in SDK testnet trace saved. Independent receipt/task review is still required." -ForegroundColor Yellow
    } else {
        Write-Host "No live transactions sent. Local Hardhat mocks are not real FHE execution." -ForegroundColor Yellow
    }
}
