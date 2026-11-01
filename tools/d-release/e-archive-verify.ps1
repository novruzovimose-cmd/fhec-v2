param(
    [Parameter(Mandatory = $true)][string]$Archive,
    [Parameter(Mandatory = $true)][string]$Manifest
)

$ErrorActionPreference = 'Stop'
$archivePath = (Resolve-Path -LiteralPath $Archive).Path
$manifestPath = (Resolve-Path -LiteralPath $Manifest).Path
$document = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($document.schema -ne 'fhec-d-source-manifest-v1') {
    throw 'Unrecognized D source-manifest schema.'
}
$records = @($document.files)
if ($records.Count -lt 50) {
    throw 'Incomplete source manifest: fewer than 50 entries.'
}
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$expected = [System.Collections.Generic.Dictionary[string,object]]::new([System.StringComparer]::Ordinal)
foreach ($record in $records) {
    $name = [string]$record.path
    if ([string]::IsNullOrWhiteSpace($name) -or $name.StartsWith('/') -or $name.Contains('\') -or $name.Contains('../') -or $name.Contains('/..') -or $expected.ContainsKey($name)) {
        throw "Unsafe or repeated source manifest path: $name"
    }
    $expected.Add($name, $record)
}
$zip = [System.IO.Compression.ZipFile]::OpenRead($archivePath)
try {
    $actualCount = 0
    foreach ($entry in $zip.Entries) {
        if ([string]::IsNullOrEmpty($entry.Name)) { continue }
        $name = $entry.FullName.Replace('\', '/')
        if (-not $expected.ContainsKey($name)) {
            throw "Archive contains an unexpected file: $name"
        }
        $record = $expected[$name]
        if ($entry.Length -ne [long]$record.bytes) {
            throw "Archive size mismatch: $name"
        }
        $sha = [System.Security.Cryptography.SHA256]::Create()
        $stream = $entry.Open()
        try {
            $digest = [System.BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '').ToLowerInvariant()
        } finally {
            $stream.Dispose()
            $sha.Dispose()
        }
        if ($digest -ne ([string]$record.sha256).ToLowerInvariant()) {
            throw "Archive SHA-256 mismatch: $name"
        }
        [void]$expected.Remove($name)
        $actualCount++
    }
    if ($expected.Count -ne 0) {
        $missing = @($expected.Keys | Sort-Object | Select-Object -First 10) -join ', '
        throw "Archive is missing $($expected.Count) manifest files: $missing"
    }
    if ($actualCount -ne $records.Count) {
        throw 'Archive file count differs from source manifest.'
    }
} finally {
    $zip.Dispose()
}
Write-Host "PASS: Verified $actualCount archive files against D source SHA-256 manifest." -ForegroundColor Green
Write-Host "ARCHIVE SHA256: $((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant())"
