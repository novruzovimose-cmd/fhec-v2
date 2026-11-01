param(
    [Parameter(Mandatory=$true)][string]$Project,
    [switch]$FastOnly,
    [switch]$RequireAudit,
    [switch]$ColdCargoHome
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Project).Path
$oldHome = $env:CARGO_HOME
$hadHome = Test-Path Env:CARGO_HOME
$oldTarget = $env:CARGO_TARGET_DIR
$hadTarget = Test-Path Env:CARGO_TARGET_DIR
$runId = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
$run = Join-Path $root "target/fhec-d-validation/$runId"
$snapshot = Join-Path $run 'source'
$reportPath = Join-Path $run 'reports'
try {
    Write-Host 'D LOCAL: creating a source-only export with independent Cargo target directory' -ForegroundColor Cyan
    & (Join-Path $root 'tools/d-release/export-source.ps1') -Project $root -Destination $snapshot
    if (-not (Test-Path -LiteralPath $snapshot -PathType Container)) { throw 'Snapshot exporter did not create a source tree.' }
    New-Item -ItemType Directory -Path $reportPath -Force | Out-Null
    if ($ColdCargoHome) {
        $env:CARGO_HOME = Join-Path $run 'fresh-cargo-home'
        Write-Host 'COLD CARGO HOME: a new registry cache will be used; network downloads may be required.' -ForegroundColor Yellow
    } else {
        Write-Host 'CARGO HOME: shared registry cache; build artifacts will remain isolated.' -ForegroundColor Yellow
    }
    $env:CARGO_TARGET_DIR = Join-Path $run 'fresh-cargo-target'
    $mode = if ($RequireAudit) { 'Required' } else { 'Auto' }
    & (Join-Path $snapshot 'tools/d-release/verify-checkout.ps1') -Project $snapshot -Reports $reportPath -FastOnly:$FastOnly -AuditMode $mode
    Write-Host ''
    if ($FastOnly) {
        Write-Host 'STAGE D LOCAL FAST VALIDATION PASSED; full workspace tests remain unverified by this run.' -ForegroundColor Yellow
    } else {
        Write-Host 'STAGE D LOCAL CLEAN-SOURCE VALIDATION PASSED.' -ForegroundColor Green
    }
    Write-Host 'HOSTED WINDOWS/LINUX CI IS NOT YET VERIFIED. DO NOT CLAIM STAGE D FULLY COMPLETE.' -ForegroundColor Yellow
    Write-Host "EVIDENCE: $run"
} finally {
    if ($hadHome) { $env:CARGO_HOME = $oldHome } else { Remove-Item Env:CARGO_HOME -ErrorAction SilentlyContinue }
    if ($hadTarget) { $env:CARGO_TARGET_DIR = $oldTarget } else { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
}
