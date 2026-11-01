param(
    [Parameter(Mandatory = $true)][string]$Project,
    [switch]$SkipAudit
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Project).Path
$previousDirectory = (Get-Location).Path
$runId = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
$run = Join-Path $root "target/fhec-e-candidate/$runId"
$snapshot = Join-Path $run 'source'
$archive = Join-Path $run 'FHEC-V1-CANDIDATE-NOT-FOR-PUBLICATION.zip'
$manifest = Join-Path $run 'source-manifest.json'
$reports = Join-Path $run 'reports'
$blockers = New-Object 'System.Collections.Generic.List[string]'

function Write-FileUtf8 {
    param([string]$Path, [string]$Content)
    [System.IO.File]::WriteAllText($Path, $Content, (New-Object System.Text.UTF8Encoding($false)))
}

try {
    Set-Location -LiteralPath $root
    foreach ($needed in @(
        'Cargo.toml', 'Cargo.lock', 'LICENSE', 'README.md', 'CHANGELOG.md',
        'tools/d-release/export-source.ps1',
        'tools/d-release/version-consistency.ps1',
        'tools/d-release/e-archive-verify.ps1',
        'docs/v1-c3/FHEC_V1_COMPLETE_GUIDE.md',
        'docs/v1-d/D_ACCEPTANCE.md',
        '.github/workflows/fhec-v1-crossplatform.yml'
    )) {
        if (-not (Test-Path -LiteralPath (Join-Path $root $needed) -PathType Leaf)) {
            throw "Required V1 release file is missing: $needed"
        }
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Cargo is not available on PATH.' }
    New-Item -ItemType Directory -Path $reports -Force | Out-Null
    Write-Host 'E: Freeze a SOURCE-ONLY candidate; NO Git commit, tag, push or publication.' -ForegroundColor Cyan
    & (Join-Path $root 'tools/d-release/export-source.ps1') -Project $root -Destination $snapshot
    if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) { throw 'Source exporter did not produce its SHA-256 manifest.' }
    Write-Host 'E: Verify Cargo version and workspace license consistency.' -ForegroundColor Cyan
    & (Join-Path $root 'tools/d-release/version-consistency.ps1') -Project $root -OutputDirectory $reports
    Write-Host 'E: Create candidate ZIP from the filtered source tree.' -ForegroundColor Cyan
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path -LiteralPath $archive) { throw 'Candidate archive exists unexpectedly; refusing overwrite.' }
    [System.IO.Compression.ZipFile]::CreateFromDirectory($snapshot, $archive, [System.IO.Compression.CompressionLevel]::Optimal, $false)
    & (Join-Path $root 'tools/d-release/e-archive-verify.ps1') -Archive $archive -Manifest $manifest
    $zipSha = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-FileUtf8 -Path (Join-Path $reports 'candidate-sha256.txt') -Content "$zipSha  FHEC-V1-CANDIDATE-NOT-FOR-PUBLICATION.zip`n"

    $dRunRoot = Join-Path $root 'target/fhec-d-validation'
    $lastD = $null
    if (Test-Path -LiteralPath $dRunRoot -PathType Container) {
        $lastD = Get-ChildItem -LiteralPath $dRunRoot -Directory | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
    }
    if ($null -eq $lastD) {
        $blockers.Add('D local evidence directory not found: rerun Stage D local verification.')
    } else {
        Write-Host "REFERENCE: Last Stage D local evidence directory: $($lastD.FullName)" -ForegroundColor Yellow
        $blockers.Add('Confirm Stage D local result belongs to the intended candidate; this new E snapshot differs by its new docs/scripts.')
    }

    $audit = 'NOT RUN'
    if ($SkipAudit) {
        $audit = 'SKIPPED BY REQUEST'
        $blockers.Add('RustSec audit was explicitly skipped.')
    } else {
        Write-Host 'E: Check for cargo-audit.' -ForegroundColor Cyan
        $cargoCommands = & cargo --list
        if ($LASTEXITCODE -ne 0) { throw 'cargo --list failed.' }
        $auditAvailable = (($cargoCommands -join "`n") -match '(?m)^\s*audit\s')
        if (-not $auditAvailable) {
            $audit = 'NOT RUN - cargo-audit missing'
            $blockers.Add('Install cargo-audit and run a successful RustSec audit of the locked dependency graph.')
            Write-Host 'AUDIT: cargo-audit is missing; RustSec gate remains OPEN.' -ForegroundColor Yellow
        } else {
            Write-Host 'E: Running RustSec advisory audit (live output).' -ForegroundColor Cyan
            & cargo audit
            $auditCode = $LASTEXITCODE
            if ($auditCode -ne 0) {
                $audit = "FAILED (exit code $auditCode)"
                $blockers.Add('RustSec audit did not pass; investigate advisories or tool/network errors.')
            } else {
                $audit = 'PASSED on this machine at candidate preparation time'
            }
        }
    }

    $gitState = 'NOT A GIT REPOSITORY'
    $gitChanges = -1
    $remoteCount = 0
    if (Test-Path -LiteralPath (Join-Path $root '.git')) {
        $gitState = 'LOCAL GIT REPOSITORY FOUND'
        $changes = @(& git status --porcelain=v1 --untracked-files=all)
        if ($LASTEXITCODE -ne 0) { throw 'git status failed.' }
        $gitChanges = $changes.Count
        $remotes = @(& git remote)
        if ($LASTEXITCODE -ne 0) { throw 'git remote failed.' }
        $remoteCount = $remotes.Count
        if ($gitChanges -gt 0) { $blockers.Add("Working tree is not release-clean ($gitChanges listed paths). Review before the final release commit.") }
        if ($remoteCount -eq 0) { $blockers.Add('No Git remote configured; final release commit and hosted CI cannot yet be verified.') }
    } else {
        $blockers.Add('Release candidate has not been committed to a Git repository.')
    }
    $blockers.Add('Hosted Windows CI must pass on the exact eventual release commit.')
    $blockers.Add('Hosted Ubuntu CI must pass on that exact commit.')
    $blockers.Add('Hosted RustSec CI must pass on that exact commit.')
    $blockers.Add('Human review required for third-party licenses, notices, archive contents and any remaining security exceptions.')
    $blockers.Add('Choose and reconcile the public release version across Cargo manifests, README, CHANGELOG and Git tag.')
    $blockers.Add('Release commit, tag and public artifact must be inspected before any publication approval.')

    $summary = [ordered]@{
        schema = 'fhec-e-release-preflight-v1'
        generated_utc = [DateTime]::UtcNow.ToString('o')
        package_is_publication_ready = $false
        candidate_archive = $archive
        candidate_sha256 = $zipSha
        manifest = $manifest
        reference_d_run = if ($null -eq $lastD) { $null } else { $lastD.FullName }
        audit_status = $audit
        git_status = $gitState
        dirty_git_paths_count = $gitChanges
        git_remotes_count = $remoteCount
        blockers = @($blockers.ToArray())
    }
    Write-FileUtf8 -Path (Join-Path $reports 'e-preflight.json') -Content ($summary | ConvertTo-Json -Depth 6)
    $lines = @(
        'FHEC V1 STAGE E - CANDIDATE PREPARATION, NOT PUBLIC RELEASE',
        "Run UTC: $($summary.generated_utc)",
        "Candidate: $archive",
        "Archive SHA256: $zipSha",
        "RustSec: $audit",
        "Git: $gitState ($gitChanges working-tree entries; $remoteCount remotes)",
        '', 'UNRESOLVED FINAL-RELEASE CHECKS:'
    )
    $lines += @($blockers | ForEach-Object { '- ' + $_ })
    $lines += @('', 'RESULT: SOURCE CANDIDATE ARCHIVE HASH-VERIFIED; NOT APPROVED FOR PUBLIC RELEASE.')
    Write-FileUtf8 -Path (Join-Path $reports 'E_STATUS.txt') -Content (($lines -join "`r`n") + "`r`n")
    Write-Host ''
    Write-Host 'PASS: E source-only candidate archive and SHA-256 checks.' -ForegroundColor Green
    Write-Host "AUDIT: $audit"
    Write-Host "BLOCKERS: $($blockers.Count) - see E_STATUS.txt. Some are manual or hosted-CI gates." -ForegroundColor Yellow
    Write-Host 'STAGE E CANDIDATE PREPARED; NOT CLEARED FOR PUBLICATION.' -ForegroundColor Yellow
    Write-Host "EVIDENCE: $reports"
} finally {
    Set-Location -LiteralPath $previousDirectory
}
