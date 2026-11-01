param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [Parameter(Mandatory = $true)][string]$PreparedFile,
    [Parameter(Mandatory = $true)][string]$RawEvidence,
    [ValidateRange(1,64)][int]$MinimumConfirmations = 2
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$PreparedFile = (Resolve-Path -LiteralPath $PreparedFile -ErrorAction Stop).Path
$RawEvidence = (Resolve-Path -LiteralPath $RawEvidence -ErrorAction Stop).Path
if ((Split-Path -Path $Project -Leaf) -ne "fhec-v2") {
    throw "Read-only Phase 8/9 audit requires the dedicated fhec-v2 project"
}
if ($env:FHEC_TESTNET_RPC_URL -notmatch '^https://') {
    throw "Read-only independent verification requires a configured HTTPS Arbitrum Sepolia RPC endpoint"
}
$prepared = Get-Content -LiteralPath $PreparedFile -Raw | ConvertFrom-Json
if ($prepared.schema -ne "fhec-cofhe-phase89-local-prepared-v1" -or
    $prepared.project -ne $Project -or $prepared.mock_preflight_passed -ne $true -or
    $prepared.codegen_confirmed -ne $true -or $prepared.live_transactions_sent -ne $false -or
    $prepared.chain_id -ne 421614) {
    throw "Prepared source validation record is absent or incompatible"
}
$generated = [string]$prepared.generated
$plan = [string]$prepared.plan_file
$seal = [string]$prepared.seal_file
if (-not (Test-Path -LiteralPath $plan -PathType Leaf) -or -not (Test-Path -LiteralPath $seal -PathType Leaf) -or
    (Get-FileHash -LiteralPath $plan -Algorithm SHA256).Hash -ne $prepared.plan_sha256 -or
    (Get-FileHash -LiteralPath $seal -Algorithm SHA256).Hash -ne $prepared.seal_sha256) {
    throw "Prepared plan or source seal was modified; no RPC calls performed"
}
$normalizedGenerated = (Resolve-Path -LiteralPath $generated -ErrorAction Stop).Path
$allowedRoot = Join-Path $Project "integration\cofhe-hardhat\phase89\generated"
if (-not $normalizedGenerated.StartsWith(($allowedRoot + [IO.Path]::DirectorySeparatorChar), [StringComparison]::OrdinalIgnoreCase)) {
    throw "Prepared Solidity sources are outside the approved project-generated directory"
}

$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0,8)
$out = Join-Path $Project "target\cofhe-phase89-audit-$runId"
if (Test-Path -LiteralPath $out) { throw "Refusing to overwrite a prior independent audit" }
$original = (Get-Location).Path
New-Item -ItemType Directory -Path $out -ErrorAction Stop | Out-Null

try {
    Set-Location -LiteralPath $Project
    node tools/cofhe/phase89/source-seal.mjs --directory $normalizedGenerated --verify $seal
    if ($LASTEXITCODE -ne 0) { throw "Prepared Solidity source audit failed" }

    $rpcReport = Join-Path $out "independently-corroborated-rpc-receipts.json"
    $comparison = Join-Path $out "phase9-real-native-chain-comparison.json"
    $html = Join-Path $out "phase9-offline-testnet-evidence.html"
    node tools/cofhe/phase89/verify-chain.mjs --plan $plan --raw $RawEvidence --out $rpcReport --confirmations $MinimumConfirmations
    if ($LASTEXITCODE -ne 0) { throw "Independent read-only Arbitrum Sepolia receipt corroboration failed" }
    node tools/cofhe/phase89/analyze-live.mjs --plan $plan --raw $RawEvidence --attestation $rpcReport --out $comparison
    if ($LASTEXITCODE -ne 0) { throw "Real testnet cost analysis failed" }
    node tools/cofhe/phase89/report-html.mjs --summary $comparison --out $html
    if ($LASTEXITCODE -ne 0) { throw "Real testnet evidence report rendering failed" }

    Write-Host "PHASES 8 + 9 READ-ONLY TESTNET EVIDENCE AUDIT PASSED" -ForegroundColor Green
    Write-Host "Reports: $out"
    Write-Host "No transaction was sent. This pass validates on-chain receipts and native fees, not off-chain coprocessor fees." -ForegroundColor Yellow
}
finally {
    Set-Location -LiteralPath $original
}
