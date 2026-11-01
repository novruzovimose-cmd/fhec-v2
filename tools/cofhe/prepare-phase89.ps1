param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [ValidateSet("smoke6", "coverage24", "comparison72")][string]$Profile = "smoke6",
    [ValidateSet("optimized-tuned", "three-way")][string]$VariantMode = "optimized-tuned",
    [switch]$SkipDependencyInstall,
    [switch]$SkipRustTests
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
if ((Split-Path -Path $Project -Leaf) -ne "fhec-v2") {
    throw "Phase 8/9 can only prepare the dedicated fhec-v2 workspace"
}
$root = Join-Path $Project "tools\cofhe"
$hardhat = Join-Path $Project "integration\cofhe-hardhat"
$phase89 = Join-Path $hardhat "phase89"
$source = Join-Path $Project "examples\cofhe_phase4.fhe"
$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0,8)
$generated = Join-Path $phase89 "generated\$runId"
$out = Join-Path $Project "target\cofhe-phase89-preflight-$runId"
$prepared = Join-Path $out "prepared-phase89.json"
$seal = Join-Path $generated "phase89-source-seal.json"
$plan = Join-Path $generated "phase89-plan.json"
$tests = @(
    (Join-Path $root "phase89\plan.test.mjs"),
    (Join-Path $root "phase89\source-seal.test.mjs"),
    (Join-Path $root "phase89\rpc.test.mjs"),
    (Join-Path $root "phase89\verify-chain.test.mjs"),
    (Join-Path $root "phase89\analyze-live.test.mjs"),
    (Join-Path $root "phase89\report-html.test.mjs")
    (Join-Path $root "phase89\verify-plans.test.mjs")
)
$required = @(
    $source,
    (Join-Path $Project "Cargo.toml"),
    (Join-Path $hardhat "package.json"),
    (Join-Path $hardhat "pnpm-lock.yaml"),
    (Join-Path $root "phase6\oracle.mjs"),
    (Join-Path $root "phase7\metrics.mjs"),
    (Join-Path $root "phase7\generate-manual.mjs"),
    (Join-Path $root "phase89\plan.mjs"),
    (Join-Path $root "phase89\source-seal.mjs"),
    (Join-Path $root "phase89\verify-chain.mjs"),
    (Join-Path $root "phase89\analyze-live.mjs"),
    (Join-Path $root "phase89\report-html.mjs"),
    (Join-Path $root "phase89\verify-plans.mjs"),
    (Join-Path $phase89 "hardhat.phase89.config.ts"),
    (Join-Path $phase89 "tsconfig.phase89.json"),
    (Join-Path $phase89 "test\FHECPhase89Prepared.test.ts"),
    (Join-Path $phase89 "run-live.ts")
) + $tests
foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing Phase 8/9 prerequisite: $path"
    }
}
foreach ($tool in @("node", "cargo", "pnpm")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Required development tool is unavailable: $tool"
    }
}
if (Test-Path -LiteralPath $generated) { throw "Refusing to reuse generated sources: $generated" }
if (Test-Path -LiteralPath $out) { throw "Refusing to reuse preflight evidence: $out" }

$originalDirectory = (Get-Location).Path
$previousSource = [Environment]::GetEnvironmentVariable("FHEC_PHASE89_SOURCES_DIR", "Process")
$complete = $false
New-Item -Path $generated -ItemType Directory -Force -ErrorAction Stop | Out-Null
New-Item -Path $out -ItemType Directory -Force -ErrorAction Stop | Out-Null

try {
    Set-Location -LiteralPath $Project
    node --test @tests
    if ($LASTEXITCODE -ne 0) { throw "Independent Phase 8/9 unit and adversarial tests failed" }
    node tools/cofhe/phase89/verify-plans.mjs
    if ($LASTEXITCODE -ne 0) { throw "Canonical six-profile live plan fixtures failed" }

    if (-not $SkipRustTests) {
        cargo fmt --all -- --check
        if ($LASTEXITCODE -ne 0) { throw "Rust formatting failed" }
        cargo test --locked -p fhec-backend-cofhe -p fhec-cli
        if ($LASTEXITCODE -ne 0) { throw "Rust CoFHE regression tests failed" }
        cargo clippy --locked -p fhec-backend-cofhe -p fhec-cli --all-targets -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw "Rust Clippy hardening failed" }
    }

    cargo run --locked -p fhec-cli -- cofhe $source --out (Join-Path $generated "FHECPhase6Optimized.sol") --library FHECPhase6Optimized --optimize --report (Join-Path $out "fhec-optimization-static-report.json")
    if ($LASTEXITCODE -ne 0) { throw "Actual Rust FHEC generated Solidity compilation failed" }
    node tools/cofhe/phase7/generate-manual.mjs --directory $generated
    if ($LASTEXITCODE -ne 0) { throw "Independent reference library and harness generation failed" }
    node tools/cofhe/phase89/plan.mjs --profile $Profile --variants $VariantMode --out $plan
    if ($LASTEXITCODE -ne 0) { throw "Canonical Phase 8/9 live plan generation failed" }
    node tools/cofhe/phase89/source-seal.mjs --directory $generated --out $seal
    if ($LASTEXITCODE -ne 0) { throw "Prepared Solidity source and plan seal failed" }

    [Environment]::SetEnvironmentVariable("FHEC_PHASE89_SOURCES_DIR", $generated, "Process")
    Set-Location -LiteralPath $hardhat
    if (-not $SkipDependencyInstall) {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "Frozen CoFHE dependency installation failed" }
    }
    pnpm exec hardhat compile --config phase89/hardhat.phase89.config.ts
    if ($LASTEXITCODE -ne 0) { throw "Phase 8/9 compilation failed" }
    pnpm exec tsc --noEmit --project phase89/tsconfig.phase89.json
    if ($LASTEXITCODE -ne 0) { throw "Phase 8/9 scoped TypeScript safety check failed" }
    pnpm exec hardhat test --config phase89/hardhat.phase89.config.ts phase89/test/FHECPhase89Prepared.test.ts
    if ($LASTEXITCODE -ne 0) { throw "Prepared live-plan mock lifecycle failed" }
    Set-Location -LiteralPath $Project

    node tools/cofhe/phase89/source-seal.mjs --directory $generated --verify $seal
    if ($LASTEXITCODE -ne 0) { throw "Source or live plan changed during preflight" }

    $planObject = Get-Content -LiteralPath $plan -Raw | ConvertFrom-Json
    $result = [ordered]@{
        schema = "fhec-cofhe-phase89-local-prepared-v1"
        project = $Project
        generated = $generated
        plan_file = $plan
        seal_file = $seal
        plan_sha256 = (Get-FileHash -LiteralPath $plan -Algorithm SHA256).Hash
        seal_sha256 = (Get-FileHash -LiteralPath $seal -Algorithm SHA256).Hash
        optimized_rust_solidity_sha256 = (Get-FileHash -LiteralPath (Join-Path $generated "FHECPhase6Optimized.sol") -Algorithm SHA256).Hash
        profile = $Profile
        variant_mode = $VariantMode
        approved_network = "arb-sepolia"
        chain_id = 421614
        planned_transactions = [int]$planObject.total_planned_transactions
        planned_executions = [int]$planObject.execution_count
        codegen_confirmed = $true
        mock_preflight_passed = $true
        rust_tests_skipped = [bool]$SkipRustTests
        live_transactions_sent = $false
        private_key_read = $false
    }
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText($prepared, ($result | ConvertTo-Json -Depth 7) + [Environment]::NewLine, $utf8)
    $complete = $true
}
finally {
    Set-Location -LiteralPath $originalDirectory
    [Environment]::SetEnvironmentVariable("FHEC_PHASE89_SOURCES_DIR", $previousSource, "Process")
}

if ($complete) {
    Write-Host "PHASE 8/9 NON-TRANSACTING PREFLIGHT PASSED" -ForegroundColor Green
    Write-Host "Prepared file: $prepared"
    Write-Host "Planned real-network transactions, including deployments: $($result.planned_transactions)"
    Write-Host "No testnet deployments or transactions occurred during this command." -ForegroundColor Yellow
    Write-Host "The live runner requires separate fee limits and explicit consent."
}
