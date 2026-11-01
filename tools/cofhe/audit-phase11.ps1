param(
    [string]$Project = (Join-Path $env:USERPROFILE "Documents\fhec-v2"),
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)

$ErrorActionPreference = "Stop"
$Project = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$EvidenceDirectory = (Resolve-Path -LiteralPath $EvidenceDirectory -ErrorAction Stop).Path
if ((Split-Path -Path $Project -Leaf) -ne "fhec-v2") {
    throw "Only a reviewed, existing fhec-v2 installation can run Phase 11 evidence checks"
}
if (-not (Test-Path -LiteralPath $EvidenceDirectory -PathType Container)) {
    throw "No offline evidence directory found"
}
$manifest = Join-Path $EvidenceDirectory "phase11-evidence-manifest.json"
if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) {
    throw "Offline bundle is missing its Phase 11 manifest"
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw "Node.js is required to independently replay the offline evidence"
}
$original = (Get-Location).Path
try {
    Set-Location -LiteralPath $Project
    node tools/cofhe/phase11/attest.mjs --directory $EvidenceDirectory --verify $manifest
    if ($LASTEXITCODE -ne 0) { throw "Offline Phase 11 cryptographic digest and arithmetic evidence recheck failed" }
    Write-Host "PHASE 11 STANDALONE OFFLINE EVIDENCE RECHECK PASSED" -ForegroundColor Green
    Write-Host "Evidence folder: $EvidenceDirectory"
    Write-Host "No wallet key, network call, paid transaction or project source mutation was needed." -ForegroundColor Yellow
}
finally {
    Set-Location -LiteralPath $original
}
