param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [Parameter(Mandatory = $true)][string]$PreparedFile,
    [switch]$AllowLiveTransactions,
    [Parameter(Mandatory = $true)][string]$ConsentPhrase,
    [Parameter(Mandatory = $true)][string]$MaxGasPriceWei,
    [Parameter(Mandatory = $true)][string]$MaxNativeFeesWei,
    [ValidateRange(1,500)][int]$MaxTransactions,
    [ValidateRange(1,64)][int]$MinimumConfirmations = 2
)

$ErrorActionPreference = "Stop"
$consent = "YES_I_ACCEPT_ARBITRUM_SEPOLIA_TRANSACTIONS_AND_FEES"
if (-not $AllowLiveTransactions -or $ConsentPhrase -cne $consent) {
    throw "No live transaction authorization. Supply both the explicit switch and exact consent phrase."
}
if ($MaxGasPriceWei -notmatch '^[1-9][0-9]*$' -or $MaxNativeFeesWei -notmatch '^[1-9][0-9]*$') {
    throw "Explicit EIP-1559 gas-price and total native-chain fee ceilings must be positive decimal wei strings"
}
if ($env:FHEC_TESTNET_RPC_URL -notmatch '^https://') {
    throw "Set a trusted HTTPS Arbitrum Sepolia RPC URL in FHEC_TESTNET_RPC_URL"
}
if ($env:FHEC_TESTNET_PRIVATE_KEY -notmatch '^0x[a-fA-F0-9]{64}$') {
    throw "Set a dedicated, funded testnet wallet in FHEC_TESTNET_PRIVATE_KEY; never paste it into a chat or report"
}

$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$PreparedFile = (Resolve-Path -LiteralPath $PreparedFile -ErrorAction Stop).Path
if ((Split-Path -Path $Project -Leaf) -ne "fhec-v2") {
    throw "Live execution is permitted only in the dedicated fhec-v2 workspace"
}
$prepared = Get-Content -LiteralPath $PreparedFile -Raw | ConvertFrom-Json
if ($prepared.schema -ne "fhec-cofhe-phase89-local-prepared-v1" -or
    $prepared.project -ne $Project -or $prepared.chain_id -ne 421614 -or
    $prepared.mock_preflight_passed -ne $true -or $prepared.codegen_confirmed -ne $true -or
    $prepared.live_transactions_sent -ne $false -or [int]$prepared.planned_transactions -gt $MaxTransactions) {
    throw "Prepared report is invalid or MaxTransactions is below the explicit planned transaction count"
}
$generated = (Resolve-Path -LiteralPath ([string]$prepared.generated) -ErrorAction Stop).Path
$allowedRoot = Join-Path $Project "integration\cofhe-hardhat\phase89\generated"
if (-not $generated.StartsWith(($allowedRoot + [IO.Path]::DirectorySeparatorChar), [StringComparison]::OrdinalIgnoreCase)) {
    throw "Prepared contract files are outside the reviewed project-generated directory"
}
$plan = [string]$prepared.plan_file
$seal = [string]$prepared.seal_file
if ((Get-FileHash -LiteralPath $plan -Algorithm SHA256).Hash -ne $prepared.plan_sha256 -or
    (Get-FileHash -LiteralPath $seal -Algorithm SHA256).Hash -ne $prepared.seal_sha256) {
    throw "Prepared source seal or exact live plan was modified; no transactions sent"
}

$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [guid]::NewGuid().ToString("N").Substring(0,8)
$out = Join-Path $Project "target\cofhe-phase89-live-$runId"
$raw = Join-Path $out "provisional-sdk-live-trace.json"
$journal = Join-Path $out "append-only-transaction-journal.jsonl"
if (Test-Path -LiteralPath $out) { throw "Refusing to overwrite existing Phase 8/9 live evidence" }

$previous = @{}
$environmentNames = @(
    "FHEC_PHASE89_SOURCES_DIR", "FHEC_PHASE89_SEAL_FILE", "FHEC_PHASE89_JOURNAL_FILE",
    "FHEC_PHASE89_RAW_EVIDENCE_FILE", "FHEC_PHASE89_CONSENT",
    "FHEC_PHASE89_MAX_GAS_PRICE_WEI", "FHEC_PHASE89_MAX_NATIVE_FEES_WEI", "FHEC_PHASE89_MAX_TX_COUNT"
)
foreach ($name in $environmentNames) {
    $previous[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
$original = (Get-Location).Path
$hardhat = Join-Path $Project "integration\cofhe-hardhat"

try {
    Set-Location -LiteralPath $Project
    node tools/cofhe/phase89/source-seal.mjs --directory $generated --verify $seal
    if ($LASTEXITCODE -ne 0) { throw "Prepared source integrity verification failed before transaction submission" }
    New-Item -ItemType Directory -Path $out -ErrorAction Stop | Out-Null
    $values = @{
        "FHEC_PHASE89_SOURCES_DIR" = $generated
        "FHEC_PHASE89_SEAL_FILE" = $seal
        "FHEC_PHASE89_JOURNAL_FILE" = $journal
        "FHEC_PHASE89_RAW_EVIDENCE_FILE" = $raw
        "FHEC_PHASE89_CONSENT" = $consent
        "FHEC_PHASE89_MAX_GAS_PRICE_WEI" = $MaxGasPriceWei
        "FHEC_PHASE89_MAX_NATIVE_FEES_WEI" = $MaxNativeFeesWei
        "FHEC_PHASE89_MAX_TX_COUNT" = [string]$MaxTransactions
    }
    foreach ($name in $values.Keys) {
        [Environment]::SetEnvironmentVariable($name, $values[$name], "Process")
    }
    Set-Location -LiteralPath $hardhat
    pnpm exec hardhat run --config phase89/hardhat.phase89.config.ts --network arb-sepolia phase89/run-live.ts
    if ($LASTEXITCODE -ne 0) {
        throw "Phase 8 paid testnet run is incomplete. Inspect the journal at $journal. Do NOT rerun automatically: transactions may already have been sent."
    }
}
finally {
    Set-Location -LiteralPath $original
    foreach ($name in $environmentNames) {
        [Environment]::SetEnvironmentVariable($name, $previous[$name], "Process")
    }
}

Write-Host "PHASE 8 SDK TESTNET TRANSACTIONS COMPLETED; INDEPENDENT RPC CHECK NEXT" -ForegroundColor Yellow
Write-Host "SDK trace: $raw"
Write-Host "Journal: $journal"
Write-Host "Do not rerun the paid Phase 8 command if the read-only RPC audit fails."
& (Join-Path $Project "tools\cofhe\audit-phase89.ps1") -Project $Project -PreparedFile $PreparedFile -RawEvidence $raw -MinimumConfirmations $MinimumConfirmations
if (-not $?) { throw "Read-only evidence audit did not complete; use audit-phase89.ps1 to retry without another deployment" }
