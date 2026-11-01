param(
    [Parameter(Mandatory=$true)][string]$Project,
    [Parameter(Mandatory=$true)][string]$Destination
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Project).Path.TrimEnd([char[]]@('\','/'))
$destinationFull = [System.IO.Path]::GetFullPath($Destination).TrimEnd([char[]]@('\','/'))
$comparison = if ($env:OS -eq 'Windows_NT') { [System.StringComparison]::OrdinalIgnoreCase } else { [System.StringComparison]::Ordinal }
if ($destinationFull.Equals($root, $comparison)) { throw 'Export destination cannot be the project root.' }
if (-not ($destinationFull.StartsWith($root + [System.IO.Path]::DirectorySeparatorChar, $comparison))) {
    throw 'For safety, export destination must be within the project root (typically target/fhec-d/...).'
}
if (Test-Path -LiteralPath $destinationFull) { throw 'Export destination already exists; refusing to overwrite.' }
$selected = New-Object 'System.Collections.Generic.List[object]'
$seen = @{}
$extensions = @('.rs', '.toml', '.fhe', '.md', '.json', '.txt', '.csv', '.yaml', '.yml')
$ignoredDirs = '^(target|\.git|\.svn|node_modules|__pycache__|\.venv|venv|dist|backup|backups|optimizer_patch_backups|fhec-cli-backups|\.fhec.*)$'
$ignoredFiles = '(?i)(^\.env($|\.)|\.bak($|\.)|\.backup($|\.)|\.corrupted\.|\.orig$|\.rej$|\.tmp$|\.log$|\.patch$|\.zip$|\.pem$|\.key$|\.p12$|\.pfx$|^id_rsa$|^id_ed25519$|^\.DS_Store$)'

function Add-Selected {
    param([string]$Relative)
    $relativeClean = $Relative.Replace('\','/')
    if ($seen.ContainsKey($relativeClean)) { throw "Duplicate export path: $relativeClean" }
    $seen[$relativeClean] = $true
    $full = Join-Path $root ($relativeClean.Replace('/', [System.IO.Path]::DirectorySeparatorChar))
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { throw "Missing source file: $relativeClean" }
    $item = Get-Item -LiteralPath $full -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Refusing linked source file: $relativeClean" }
    if ($item.Name -match $ignoredFiles) { throw "Suspicious release file: $relativeClean" }
    [void]$selected.Add([pscustomobject]@{ relative = $relativeClean; source = $full; length = $item.Length })
}

function Select-Directory {
    param([string]$RelativeDirectory, [string[]]$AcceptedExtensions)
    $start = Join-Path $root ($RelativeDirectory.Replace('/', [System.IO.Path]::DirectorySeparatorChar))
    if (-not (Test-Path -LiteralPath $start -PathType Container)) { return }
    $pending = New-Object 'System.Collections.Generic.Stack[string]'
    $pending.Push($start)
    while ($pending.Count -gt 0) {
        $current = $pending.Pop()
        $dir = Get-Item -LiteralPath $current -Force
        if (($dir.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Refusing linked release directory: $current" }
        foreach ($item in (Get-ChildItem -LiteralPath $current -Force)) {
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Refusing linked item inside release directory: $($item.FullName)" }
            if ($item.PSIsContainer) {
                if ($item.Name -notmatch $ignoredDirs) { $pending.Push($item.FullName) }
                continue
            }
            if ($item.Name -match $ignoredFiles) { continue }
            if ($AcceptedExtensions -notcontains $item.Extension.ToLowerInvariant()) { continue }
            $rel = $item.FullName.Substring($root.Length).TrimStart([char[]]@('\','/')).Replace('\','/')
            Add-Selected -Relative $rel
        }
    }
}

foreach ($required in @('Cargo.toml','Cargo.lock','rust-toolchain.toml','LICENSE','README.md','CHANGELOG.md','.gitignore')) {
    Add-Selected -Relative $required
}
foreach ($optional in @('SECURITY.md','CONTRIBUTING.md','NOTICE','NOTICE.md','.editorconfig','rustfmt.toml','.rustfmt.toml','.cargo/config.toml')) {
    if (Test-Path -LiteralPath (Join-Path $root $optional) -PathType Leaf) { Add-Selected -Relative $optional }
}
Select-Directory -RelativeDirectory 'crates' -AcceptedExtensions $extensions
Select-Directory -RelativeDirectory 'docs' -AcceptedExtensions @('.md','.txt','.json')
Select-Directory -RelativeDirectory 'examples' -AcceptedExtensions @('.fhe','.md','.txt','.json')
Select-Directory -RelativeDirectory '.github/workflows' -AcceptedExtensions @('.yaml','.yml')
Select-Directory -RelativeDirectory 'tools/d-release' -AcceptedExtensions @('.ps1','.sh','.json','.md')
foreach ($optional in @('tools/verify-c3.ps1','tools/c3-manifest.json')) {
    if (Test-Path -LiteralPath (Join-Path $root $optional) -PathType Leaf) { Add-Selected -Relative $optional }
}
if ($selected.Count -lt 50) { throw "Only $($selected.Count) source files found; aborting incomplete export." }

$privateKey = '-----BEGIN (?:OPENSSH |RSA |EC |DSA )?PRIVATE KEY-----'
$accessKeys = '(?<![A-Z0-9])(AKIA[0-9A-Z]{16})(?![A-Z0-9])'
$githubToken = '(?<![A-Za-z0-9_])gh[pousr]_[A-Za-z0-9_]{30,}'
foreach ($entry in $selected) {
    if ($entry.length -gt 5MB) { throw "Unexpectedly large source file: $($entry.relative)" }
    $data = [System.IO.File]::ReadAllText($entry.source)
    if ($data -match $privateKey -or $data -match $accessKeys -or $data -match $githubToken) {
        throw "Possible credential content in $($entry.relative). Review before exporting. No destination files created."
    }
}

$created = $false
try {
    New-Item -ItemType Directory -Path $destinationFull -Force | Out-Null
    $created = $true
    $records = New-Object 'System.Collections.Generic.List[object]'
    foreach ($entry in ($selected | Sort-Object relative)) {
        $copy = Join-Path $destinationFull ($entry.relative.Replace('/', [System.IO.Path]::DirectorySeparatorChar))
        $folder = Split-Path -Path $copy -Parent
        if (-not (Test-Path -LiteralPath $folder -PathType Container)) { New-Item -ItemType Directory -Path $folder -Force | Out-Null }
        Copy-Item -LiteralPath $entry.source -Destination $copy -ErrorAction Stop
        $srcHash = (Get-FileHash -LiteralPath $entry.source -Algorithm SHA256).Hash.ToLowerInvariant()
        $copyHash = (Get-FileHash -LiteralPath $copy -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($srcHash -ne $copyHash) { throw "Source changed during copying or copied bytes differ: $($entry.relative)" }
        [void]$records.Add([pscustomobject]@{path=$entry.relative;sha256=$copyHash;bytes=$entry.length})
    }
    $manifest = [ordered]@{schema='fhec-d-source-manifest-v1';files=@($records.ToArray())}
    $manifestFile = Join-Path (Split-Path -Parent $destinationFull) 'source-manifest.json'
    [System.IO.File]::WriteAllText($manifestFile, ($manifest | ConvertTo-Json -Depth 6), (New-Object System.Text.UTF8Encoding($false)))
    Write-Host "PASS: Clean source export ($($records.Count) files)." -ForegroundColor Green
    Write-Host "SOURCE SNAPSHOT: $destinationFull"
    Write-Host "SHA256 MANIFEST: $manifestFile"
} catch {
    if ($created -and (Test-Path -LiteralPath $destinationFull)) { Remove-Item -LiteralPath $destinationFull -Recurse -Force -ErrorAction SilentlyContinue }
    throw
}
