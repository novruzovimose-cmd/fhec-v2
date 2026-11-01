param(
    [Parameter(Mandatory=$true)][string]$Project,
    [switch]$FastOnly,
    [ValidateSet('Auto','Required','Skip')][string]$AuditMode = 'Auto',
    [string]$Reports
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Project).Path
$old = (Get-Location).Path
$oldCargoTarget = $env:CARGO_TARGET_DIR
$hadCargoTarget = Test-Path Env:CARGO_TARGET_DIR
$reportPath = $Reports
if (-not $reportPath) { $reportPath = Join-Path $root 'target/fhec-d-checkout-reports' }
if (-not [System.IO.Path]::IsPathRooted($reportPath)) { $reportPath = Join-Path $root $reportPath }
$reportPath = [System.IO.Path]::GetFullPath($reportPath)
if (-not (Test-Path -LiteralPath $reportPath -PathType Container)) { New-Item -ItemType Directory -Path $reportPath -Force | Out-Null }

function Invoke-CargoGate {
    param([string]$Name,[string[]]$Arguments)
    Write-Host ''
    Write-Host "VERIFY: $Name" -ForegroundColor Cyan
    Write-Host "COMMAND: cargo $($Arguments -join ' ')"
    & cargo @Arguments
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) { throw "$Name failed (exit code $exitCode). Source files were not changed." }
    Write-Host "PASS: $Name" -ForegroundColor Green
}

try {
    Set-Location -LiteralPath $root
    foreach ($required in @('Cargo.toml','Cargo.lock','LICENSE','rust-toolchain.toml','docs/v1-c3/FHEC_V1_COMPLETE_GUIDE.md','tools/d-release/dependency-inventory.ps1','examples/conditional_mux.fhe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $root $required) -PathType Leaf)) { throw "Missing checkout prerequisite: $required" }
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Cargo is missing from PATH.' }
    $manifestContent = Get-Content -LiteralPath (Join-Path $root 'Cargo.toml') -Raw -Encoding UTF8
    if ($manifestContent -notmatch 'license\s*=\s*"Apache-2.0"') { throw 'Workspace Apache-2.0 license declaration is missing.' }
    $licenseContent = Get-Content -LiteralPath (Join-Path $root 'LICENSE') -Raw -Encoding UTF8
    if ($licenseContent -notmatch 'Apache License' -or $licenseContent -notmatch 'Version 2.0' -or $licenseContent.Length -lt 9000) { throw 'LICENSE does not appear to contain the complete Apache-2.0 license.' }
    if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $reportPath 'cargo-target' }
    $env:CARGO_TARGET_DIR = [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
    Write-Host "CARGO TARGET: $env:CARGO_TARGET_DIR"
    & (Join-Path $root 'tools/d-release/version-consistency.ps1') -Project $root -OutputDirectory $reportPath
    Invoke-CargoGate -Name 'Cargo formatting' -Arguments @('fmt','--all','--','--check')
    Invoke-CargoGate -Name 'Locked workspace check' -Arguments @('check','--workspace','--locked')
    if ($FastOnly) {
        Invoke-CargoGate -Name 'Fast CLI contract tests' -Arguments @('test','-p','fhec-cli','--test','cli_contract_c2','--locked')
    } else {
        Write-Host 'NOTE: Real TFHE tests can take several minutes. Test output is streamed live.' -ForegroundColor Yellow
        Invoke-CargoGate -Name 'Full locked workspace tests' -Arguments @('test','--workspace','--locked')
    }
    Invoke-CargoGate -Name 'Zero-warning Clippy' -Arguments @('clippy','--workspace','--all-targets','--locked','--','-D','warnings')
    Invoke-CargoGate -Name 'Release binaries' -Arguments @('build','--release','--locked','-p','fhec-cli','-p','fhec-bench','--bins')
    $exeSuffix = if ([System.Environment]::OSVersion.Platform -eq [System.PlatformID]::Win32NT) { '.exe' } else { '' }
    $exe = Join-Path $env:CARGO_TARGET_DIR "release/fhec-cli$exeSuffix"
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Release binary missing: $exe" }
    Write-Host ''
    Write-Host 'VERIFY: Direct binary help and source-to-circuit build' -ForegroundColor Cyan
    & $exe --help
    if ($LASTEXITCODE -ne 0) { throw 'Release CLI --help failed.' }
    & $exe build 'examples/conditional_mux.fhe'
    if ($LASTEXITCODE -ne 0) { throw 'Release CLI build example failed.' }
    & $exe analyze 'examples/conditional_mux.fhe'
    if ($LASTEXITCODE -ne 0) { throw 'Release CLI analyzer example failed.' }
    Write-Host 'PASS: Direct CLI workflows' -ForegroundColor Green
    & (Join-Path $root 'tools/d-release/dependency-inventory.ps1') -Project $root -OutputDirectory $reportPath -AuditMode $AuditMode
    if ($FastOnly) {
        Write-Host 'D CHECKOUT FAST GATES PASSED; FULL TESTS NOT RUN.' -ForegroundColor Yellow
    } else {
        Write-Host 'D CHECKOUT GATES PASSED (on this machine only).' -ForegroundColor Green
    }
    Write-Host "REPORTS: $reportPath"
} finally {
    Set-Location -LiteralPath $old
    if ($hadCargoTarget) { $env:CARGO_TARGET_DIR = $oldCargoTarget } else { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
}
