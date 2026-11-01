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
    if ($packages.Count -ne 12) { throw "Expected 12 release workspace packages; found $($packages.Count). Review workspace layout before release." }
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
