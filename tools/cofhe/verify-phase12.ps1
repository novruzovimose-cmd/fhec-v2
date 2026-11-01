param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [Parameter(Mandatory = $true)][string]$Dossier,
    [string]$RepeatDossier,
    [string]$ExpectedSourceSealSha256,
    [switch]$SkipRustWorkspace
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$Dossier = (Resolve-Path -LiteralPath $Dossier -ErrorAction Stop).Path
if ((Split-Path -Path $Project -Leaf) -ne "fhec-v2") {
    throw "Phase 12 verifier requires the dedicated existing fhec-v2 workspace"
}
if (-not (Test-Path -LiteralPath (Join-Path $Dossier "phase11-evidence-manifest.json") -PathType Leaf)) {
    throw "Provide a complete Phase 11 source-sealed dossier, not a bare transcript or unverified claim"
}
if ($RepeatDossier) {
    $RepeatDossier = (Resolve-Path -LiteralPath $RepeatDossier -ErrorAction Stop).Path
}
if ($ExpectedSourceSealSha256 -and $ExpectedSourceSealSha256 -cnotmatch '^[A-Fa-f0-9]{64}$') {
    throw "Optional independently saved Phase 8/9 source seal must be exactly 64 hexadecimal characters"
}
$pin = @()
if ($ExpectedSourceSealSha256) { $pin = @("--expected-seal", $ExpectedSourceSealSha256) }

foreach ($name in @(
    "tools\cofhe\phase11\attest.mjs",
    "tools\cofhe\phase12\cli.mjs",
    "tools\cofhe\phase12\stress-fixtures.mjs",
    "tools\cofhe\phase12\fixtures\mock-dossier\phase11-evidence-manifest.json",
    "integration\cofhe-hardhat\pnpm-lock.yaml",
    "Cargo.toml", "Cargo.lock"
)) {
    if (-not (Test-Path -LiteralPath (Join-Path $Project $name) -PathType Leaf)) {
        throw "Phase 12 required project file missing: $name"
    }
}
foreach ($binary in @("node", "cargo")) {
    if (-not (Get-Command $binary -ErrorAction SilentlyContinue)) {
        if ($binary -eq "cargo" -and $SkipRustWorkspace) { continue }
        throw "Required Phase 12 command missing: $binary"
    }
}

$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
$result = Join-Path $Project "target\cofhe-phase12-candidate-$runId"
$original = (Get-Location).Path
try {
    Set-Location -LiteralPath $Project
    node tools/cofhe/phase11/attest.mjs --directory $Dossier --verify (Join-Path $Dossier "phase11-evidence-manifest.json")
    if ($LASTEXITCODE -ne 0) { throw "Independent Phase 11 evidence replay failed" }

    $tests = @(
        "tools/cofhe/phase12/coverage.test.mjs",
        "tools/cofhe/phase12/inventory.test.mjs",
        "tools/cofhe/phase12/candidate.test.mjs",
        "tools/cofhe/phase12/adversarial.test.mjs",
        "tools/cofhe/phase12/repro.test.mjs",
        "tools/cofhe/phase12/report.test.mjs",
        "tools/cofhe/phase12/stress-fixtures.test.mjs"
    )
    node --test @tests
    if ($LASTEXITCODE -ne 0) { throw "Phase 12 adversarial, coverage, release capsule or deterministic reference tests failed" }

    node tools/cofhe/phase12/stress-fixtures.mjs --verify (Join-Path $Project "tools\cofhe\phase12\fixtures\stress")
    if ($LASTEXITCODE -ne 0) { throw "4,608 deterministic arithmetic stress reference vectors did not validate" }

    node tools/cofhe/phase12/cli.mjs attack --dossier $Dossier @pin
    if ($LASTEXITCODE -ne 0) { throw "Actual chosen user dossier did not reject all 12 deliberate tampering attacks" }

    if ($RepeatDossier) {
        node tools/cofhe/phase12/cli.mjs compare --first $Dossier --second $RepeatDossier
        if ($LASTEXITCODE -ne 0) { throw "Independent second run yielded different oracle results, a different source seal or incompatible corpus" }
    }

    if (-not $SkipRustWorkspace) {
        cargo test --workspace --locked
        if ($LASTEXITCODE -ne 0) { throw "Full Rust workspace regression tests did not pass" }
    }

    node tools/cofhe/phase12/cli.mjs create --project $Project --dossier $Dossier --out $result @pin
    if ($LASTEXITCODE -ne 0) { throw "Atomic release candidate generation failed" }

    node tools/cofhe/phase12/cli.mjs verify --candidate $result --project $Project @pin
    if ($LASTEXITCODE -ne 0) { throw "Independent portable candidate verification failed" }

    Write-Host "PHASE 12 LOCAL COFHE QUALIFICATION PASSED" -ForegroundColor Green
    Write-Host "Source-sealed, tamper-tested portable candidate: $result"
    Write-Host "Release and funding gates: $(Join-Path $result 'qualification.json')"
    Write-Host "Offline grant evidence summary: $(Join-Path $result 'qualification-report.html')"
    if ($SkipRustWorkspace) {
        Write-Host "Rust workspace tests were SKIPPED; do not claim a complete Rust regression pass." -ForegroundColor Yellow
    }
    if (-not $RepeatDossier) {
        Write-Host "Second independent run was not supplied; cross-run reproducibility is UNVERIFIED." -ForegroundColor Yellow
    }
    Write-Host "No blockchain transactions, wallet actions, testnet fees or live grants were performed." -ForegroundColor Yellow
    Write-Host "Four external evidence gates remain pending, even after this local verification passes." -ForegroundColor Yellow
}
finally {
    Set-Location -LiteralPath $original
}
