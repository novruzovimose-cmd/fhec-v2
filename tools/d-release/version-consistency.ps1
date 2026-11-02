param(
    [Parameter(Mandatory=$true)][string]$Project,
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Project).Path
if (-not (Test-Path -LiteralPath $OutputDirectory -PathType Container)) { New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null }
$previous = (Get-Location).Path
try {
    Set-Location -LiteralPath $root
    Write-Host 'VERIFY: All local Cargo workspace package versions agree' -ForegroundColor Cyan
    $raw = & cargo metadata --locked --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata --locked --no-deps failed.' }
    $metadata = ($raw -join "`n") | ConvertFrom-Json
    $members = @($metadata.workspace_members)
    $packages = @($metadata.packages | Where-Object { $members -contains $_.id })
    $expectedPackageNames = @(
        'fhec-analyzer',
        'fhec-backend',
        'fhec-backend-cofhe',
        'fhec-backend-tfhe',
        'fhec-bench',
        'fhec-circuit',
        'fhec-cli',
        'fhec-compiler',
        'fhec-frontend',
        'fhec-ir',
        'fhec-optimizer',
        'fhec-runtime',
        'fhec-tests'
    )
    $actualPackageNames = @($packages | ForEach-Object { [string]$_.name } | Sort-Object -Unique)
    $missingPackages = @($expectedPackageNames | Where-Object { $actualPackageNames -notcontains $_ })
    $unexpectedPackages = @($actualPackageNames | Where-Object { $expectedPackageNames -notcontains $_ })
    if ($packages.Count -ne $expectedPackageNames.Count -or $missingPackages.Count -gt 0 -or $unexpectedPackages.Count -gt 0) {
        throw ("Release workspace package inventory mismatch. Missing: [{0}]. Unexpected: [{1}]. Expected {2} packages; found {3}." -f ($missingPackages -join ', '), ($unexpectedPackages -join ', '), $expectedPackageNames.Count, $packages.Count)
    }
    $versions = @($packages | ForEach-Object { [string]$_.version } | Sort-Object -Unique)
    if ($versions.Count -ne 1) { throw "Conflicting package versions: $($versions -join ', ')" }
    $licenses = @($packages | ForEach-Object { [string]$_.license } | Sort-Object -Unique)
    if ($licenses.Count -ne 1 -or $licenses[0] -ne 'Apache-2.0') { throw "Workspace license mismatch: $($licenses -join ', ')" }
    $lines = @(
        "Cargo workspace version: $($versions[0])",
        "Workspace package count: $($packages.Count)",
        'All workspace licenses: Apache-2.0',
        '',
        'Public release version and README/CHANGELOG labeling still require a human decision.',
        'Do not infer a public v1.0.0 tag from a 0.1.0 package version.',
        ''
    )
    $lines += @($packages | Sort-Object name | ForEach-Object { "$($_.name) $($_.version) $($_.license)" })
    [System.IO.File]::WriteAllLines((Join-Path $OutputDirectory 'version-consistency.txt'),[string[]]$lines)
    Write-Host "PASS: All $($packages.Count) package manifests match version $($versions[0]) and Apache-2.0 license." -ForegroundColor Green
    Write-Host 'NOTE: Public tag/README/changelog version reconciliation remains pending.' -ForegroundColor Yellow
} finally {
    Set-Location -LiteralPath $previous
}
