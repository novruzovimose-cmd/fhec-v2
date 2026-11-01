param(
    [Parameter(Mandatory=$true)][string]$Project,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [ValidateSet('Auto','Required','Skip')][string]$AuditMode = 'Auto'
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Project).Path
if (-not (Test-Path -LiteralPath $OutputDirectory -PathType Container)) { New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null }
$previous = (Get-Location).Path
try {
    Set-Location -LiteralPath $root
    Write-Host 'DEPENDENCIES: cargo metadata --locked --format-version 1' -ForegroundColor Cyan
    $metadataRaw = & cargo metadata --locked --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed.' }
    $metadata = ($metadataRaw -join "`n") | ConvertFrom-Json
    if (-not $metadata.packages -or $metadata.packages.Count -lt 12) { throw 'Cargo metadata package inventory is incomplete.' }
    $members = New-Object 'System.Collections.Generic.HashSet[string]'
    foreach ($member in $metadata.workspace_members) { [void]$members.Add([string]$member) }
    $rows = New-Object 'System.Collections.Generic.List[object]'
    foreach ($pkg in $metadata.packages) {
        $license = if ($pkg.license) { [string]$pkg.license } else { 'UNKNOWN' }
        $scope = if ($members.Contains([string]$pkg.id)) { 'workspace' } else { 'dependency' }
        [void]$rows.Add([pscustomobject]@{
            name = [string]$pkg.name
            version = [string]$pkg.version
            license = $license
            scope = $scope
            source = [string]$pkg.source
        })
    }
    $sorted = @($rows | Sort-Object scope,name,version)
    $output = Join-Path $OutputDirectory 'dependency-inventory.json'
    [System.IO.File]::WriteAllText($output, ($sorted | ConvertTo-Json -Depth 5), (New-Object System.Text.UTF8Encoding($false)))
    $unknown = @($sorted | Where-Object { $_.license -eq 'UNKNOWN' })
    $copyleft = @($sorted | Where-Object { $_.license -match '(?i)GPL|AGPL|LGPL|MPL|EPL' })
    $gitDependencies = @($sorted | Where-Object { $_.scope -eq 'dependency' -and $_.source -match '^git\+' })
    $review = @(
        "Packages: $($sorted.Count)",
        "Unknown license fields: $($unknown.Count)",
        "Potential copyleft / reciprocal license expressions: $($copyleft.Count)",
        "Git sourced dependencies: $($gitDependencies.Count)",
        '',
        'UNKNOWN LICENSE FIELDS:',
        ($unknown | ForEach-Object { "$($_.name) $($_.version) ($($_.scope))" }),
        '',
        'LICENSE EXPRESSIONS FOR MANUAL REVIEW:',
        ($copyleft | ForEach-Object { "$($_.name) $($_.version) $($_.license)" }),
        '',
        'GIT DEPENDENCIES:',
        ($gitDependencies | ForEach-Object { "$($_.name) $($_.version) $($_.source)" })
    )
    [System.IO.File]::WriteAllLines((Join-Path $OutputDirectory 'license-review.txt'), [string[]]$review)
    Write-Host "PASS: Dependency inventory generated ($($sorted.Count) packages)." -ForegroundColor Green
    Write-Host 'NOTE: License metadata inventory is not a legal license clearance.' -ForegroundColor Yellow
    if ($AuditMode -eq 'Skip') {
        Write-Host 'AUDIT: SKIPPED by request; vulnerability review remains open.' -ForegroundColor Yellow
        return
    }
    if (-not (Get-Command cargo-audit -ErrorAction SilentlyContinue)) {
        $check = & cargo --list
        if ($LASTEXITCODE -ne 0 -or ($check -join "`n") -notmatch '(?m)^\s*audit\s') {
            if ($AuditMode -eq 'Required') { throw 'cargo-audit is missing. Install it and retry required audit.' }
            Write-Host 'AUDIT: NOT RUN, cargo-audit is not installed. Vulnerability review remains open.' -ForegroundColor Yellow
            return
        }
    }
    Write-Host 'AUDIT: running cargo audit using the current RustSec advisory database' -ForegroundColor Cyan
    & cargo audit
    if ($LASTEXITCODE -ne 0) { throw 'cargo audit found an issue or could not complete. Review output before release.' }
    Write-Host 'PASS: cargo audit returned success for the current advisory database.' -ForegroundColor Green
} finally {
    Set-Location -LiteralPath $previous
}
