param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [string]$ReusePhase4Run = "",
    [switch]$SkipDependencyInstall,
    [switch]$SkipRustTests,
    [ValidateRange(1,8)][int]$CasesPerFamily = 2,
    [string]$OnchainTarget = "",
    [string]$RpcEnv = ""
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$tools = Join-Path $Project "tools\cofhe"
$phase5 = Join-Path $tools "phase5"
$phase4Verifier = Join-Path $tools "verify-phase4.ps1"
$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0,8)
$directory = Join-Path $Project "target\cofhe-phase5-$runId"
$replaced = $false
$original = (Get-Location).Path

if ((Split-Path -Path $Project -Leaf) -ne "fhec-v2") {
    throw "Phase 5 verification is only supported in the dedicated fhec-v2 workspace"
}
if (-not (Test-Path -LiteralPath (Join-Path $Project "Cargo.toml") -PathType Leaf)) {
    throw "FHEC V2 Cargo manifest missing"
}
if (-not (Get-Command "node" -ErrorAction SilentlyContinue)) {
    throw "Node.js is required for Phase 5 validation"
}
if (-not (Test-Path -LiteralPath $phase4Verifier -PathType Leaf)) {
    throw "Phase 4 verifier missing. Install and pass Phase 4 first."
}
if ([bool]$OnchainTarget -ne [bool]$RpcEnv) {
    throw "OnchainTarget and RpcEnv must be supplied together"
}
if ($RpcEnv -and $RpcEnv -notmatch '^[A-Z][A-Z0-9_]{3,70}$') {
    throw "Invalid RPC environment variable name"
}

$testFiles = @(
    (Join-Path $phase5 "audit-local.test.mjs"),
    (Join-Path $phase5 "verify-receipts.test.mjs"),
    (Join-Path $phase5 "evidence-bundle.test.mjs")
)
foreach ($required in $testFiles) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Missing Phase 5 validation test: $required"
    }
}

New-Item -ItemType Directory -Path $directory -ErrorAction Stop | Out-Null
try {
    Set-Location -LiteralPath $Project
    node --test @testFiles
    if ($LASTEXITCODE -ne 0) { throw "Phase 5 local unit tests failed" }

    if ($ReusePhase4Run) {
        $phase4Run = (Resolve-Path -LiteralPath $ReusePhase4Run -ErrorAction Stop).Path
    } else {
        $target = Join-Path $Project "target"
        $before = @{}
        Get-ChildItem -LiteralPath $target -Directory -Filter "cofhe-phase4-*" | ForEach-Object { $before[$_.FullName] = $true }
        & $phase4Verifier -Project $Project -SkipDependencyInstall:$SkipDependencyInstall -SkipRustTests:$SkipRustTests -CasesPerFamily $CasesPerFamily
        if (-not $?) { throw "Phase 4 verification failed inside Phase 5" }
        $new = @(Get-ChildItem -LiteralPath $target -Directory -Filter "cofhe-phase4-*" | Where-Object { -not $before.ContainsKey($_.FullName) })
        if ($new.Count -ne 1) { throw "Expected exactly one new Phase 4 report directory" }
        $phase4Run = $new[0].FullName
    }

    $local = Join-Path $directory "local-attestation.json"
    node tools/cofhe/phase5/audit-local.mjs --project $Project --run $phase4Run --out $local
    if ($LASTEXITCODE -ne 0) { throw "Independent local evidence verification failed" }

    $receipts = ""
    if ($OnchainTarget) {
        $target = (Resolve-Path -LiteralPath $OnchainTarget -ErrorAction Stop).Path
        $receipts = Join-Path $directory "read-only-receipts.json"
        node tools/cofhe/phase5/verify-receipts.mjs --target $target --rpc-env $RpcEnv --out $receipts
        if ($LASTEXITCODE -ne 0) { throw "Public-network receipt validation failed" }
    }

    $grant = Join-Path $directory "grant-evidence.json"
    if ($receipts) {
        node tools/cofhe/phase5/evidence-bundle.mjs --local $local --receipts $receipts --out $grant
    } else {
        node tools/cofhe/phase5/evidence-bundle.mjs --local $local --out $grant
    }
    if ($LASTEXITCODE -ne 0) { throw "Grant evidence composition failed" }

    $hash = (Get-FileHash -LiteralPath $grant -Algorithm SHA256).Hash
    Write-Host "PHASE 5 INDEPENDENT EVIDENCE VALIDATION PASSED" -ForegroundColor Green
    Write-Host "Phase 4 verified source: $phase4Run"
    Write-Host "Phase 5 output: $directory"
    Write-Host "Evidence bundle SHA-256: $hash"
    Write-Host "IMPORTANT: No live FHE execution or production security audit has been established by this verifier." -ForegroundColor Yellow
}
finally {
    Set-Location -LiteralPath $original
}
