param(
    [Parameter(Mandatory=$true)][string]$Project,
    [switch]$FastOnly
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Project -ErrorAction Stop).Path
$previous = (Get-Location).Path

function Fail {
    param([string]$Message)
    throw "C3 verification failed: $Message"
}

function Pass {
    param([string]$Message)
    Write-Host "PASS: $Message" -ForegroundColor Green
}

function Gate {
    param([string]$Name, [string[]]$CargoArguments)
    Write-Host ''
    Write-Host "VERIFY: $Name" -ForegroundColor Cyan
    Write-Host "COMMAND: cargo $($CargoArguments -join ' ')"
    & cargo @CargoArguments
    $nativeExit = $LASTEXITCODE
    if ($nativeExit -ne 0) {
        Fail "$Name (exit code $nativeExit). C3 does not change or roll back existing Rust source."
    }
    Pass $Name
}

function Invoke-Example {
    param([string]$Exe, [string]$Command, [string]$RelativePath, [string]$LogDir)
    $program = ($RelativePath -replace '\\','/')
    $basename = [System.IO.Path]::GetFileNameWithoutExtension($RelativePath)
    $log = Join-Path $LogDir "$Command-$basename.txt"
    Write-Host "VERIFY: $Command $program" -ForegroundColor Cyan
    $output = & $Exe $Command $program | Out-String
    $nativeExit = $LASTEXITCODE
    [System.IO.File]::WriteAllText($log, $output, (New-Object System.Text.UTF8Encoding($false)))
    if ($nativeExit -ne 0) {
        Write-Host ($output | Select-Object -First 1)
        Fail "$Command $program (exit code $nativeExit); inspect $log"
    }
    if ($Command -eq 'build') {
        if (-not $output.Contains('Build successful.') -or -not $output.Contains('Verification: PASS')) {
            Fail "Build did not report circuit verification: $program (log $log)"
        }
    }
    if ($Command -eq 'analyze') {
        $labels = @(
            'FHEC Circuit Analysis',
            'Cost model: default (not hardware-calibrated)',
            'CANDIDATES',
            'Prediction interval:',
            'Interval source:',
            'DECISION',
            'Selected predicted latency:',
            'Rejected predicted latency:',
            'IR COST ATTRIBUTION',
            'Analysis complete.'
        )
        foreach ($label in $labels) {
            if (-not $output.Contains($label)) {
                Fail "Analyzer report missing label [$label] for $program; inspect $log"
            }
        }
        $selected = [regex]::Match($output, '(?m)^Selected predicted latency:\s*([0-9]+(?:\.[0-9]+)?) ms\s*$')
        $rejected = [regex]::Match($output, '(?m)^Rejected predicted latency:\s*([0-9]+(?:\.[0-9]+)?) ms\s*$')
        if (-not $selected.Success -or -not $rejected.Success) {
            Fail "Cannot parse candidate selection latencies in $log"
        }
        $culture = [System.Globalization.CultureInfo]::InvariantCulture
        $selectedMs = [double]::Parse($selected.Groups[1].Value, $culture)
        $rejectedMs = [double]::Parse($rejected.Groups[1].Value, $culture)
        if ($selectedMs -gt ($rejectedMs + 0.001)) {
            Fail "Candidate-selection policy regression for $program ($selectedMs ms > $rejectedMs ms)"
        }
        Write-Host "COST MODEL: uncalibrated; selected $selectedMs ms; rejected $rejectedMs ms"
    }
    Pass "$Command $program"
}

try {
    Set-Location -LiteralPath $root
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Fail 'Cargo was not found on PATH.'
    }

    $manifestPath = Join-Path $root 'tools/c3-manifest.json'
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        Fail 'Missing tools/c3-manifest.json. Run C3 install.ps1 first.'
    }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($manifest.schema -ne 'fhec-c3-1' -or $manifest.files.Count -lt 10) {
        Fail 'Unexpected C3 manifest format.'
    }

    Write-Host 'VERIFY: Installed C3 files and digests' -ForegroundColor Cyan
    foreach ($entry in $manifest.files) {
        $relative = [string]$entry.path
        if ($relative -match '^[/\\]' -or $relative.Contains('..') -or $relative.Contains(':')) {
            Fail "Unsafe C3 manifest path: $relative"
        }
        $destination = Join-Path $root ($relative.Replace('/', [System.IO.Path]::DirectorySeparatorChar))
        if (-not (Test-Path -LiteralPath $destination -PathType Leaf)) {
            Fail "C3 file is missing: $relative"
        }
        $actual = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -ne [string]$entry.sha256) {
            Fail "C3 file differs from the bundle: $relative"
        }
    }
    Pass "Installed C3 file hashes ($($manifest.files.Count))"

    $prerequisites = @(
        'Cargo.toml',
        'Cargo.lock',
        'crates/fhec-cli/src/main.rs',
        'crates/fhec-cli/src/cli_analyze.rs',
        'crates/fhec-cli/tests/cli_contract_c2.rs',
        'docs/benchmark-methodology.md',
        'docs/c2-cli-contract.md',
        'examples/conditional_mux.fhe',
        'examples/boolean_mux.fhe'
    )
    foreach ($relative in $prerequisites) {
        $path = Join-Path $root ($relative.Replace('/', [System.IO.Path]::DirectorySeparatorChar))
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            Fail "Missing pre-C3 prerequisite: $relative"
        }
    }
    Pass 'C1 and C2 prerequisites present'

    $stamp = [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
    $logDir = Join-Path $root ("target/fhec-c3-verification/$stamp")
    New-Item -ItemType Directory -Path $logDir -Force | Out-Null
    Write-Host "C3 report directory: $logDir"

    Gate -Name 'Workspace formatting' -CargoArguments @('fmt', '--all', '--', '--check')
    Gate -Name 'Locked workspace compilation' -CargoArguments @('check', '--workspace', '--locked')
    Gate -Name 'C2 executable contract regressions' -CargoArguments @('test', '-p', 'fhec-cli', '--test', 'cli_contract_c2', '--locked')
    Gate -Name 'Optimizer regressions' -CargoArguments @('test', '-p', 'fhec-optimizer', '--locked')
    Gate -Name 'Compiler regressions' -CargoArguments @('test', '-p', 'fhec-compiler', '--locked')
    Gate -Name 'Analyzer regressions' -CargoArguments @('test', '-p', 'fhec-analyzer', '--locked')
    Gate -Name 'Benchmark unit regressions' -CargoArguments @('test', '-p', 'fhec-bench', '--locked', '--bin', 'fhec-bench')

    if (-not $FastOnly) {
        Write-Host ''
        Write-Host 'NOTE: Full workspace tests include real TFHE operations. Output is streamed live.' -ForegroundColor Yellow
        Gate -Name 'Full workspace including real TFHE' -CargoArguments @('test', '--workspace', '--locked')
    }

    Gate -Name 'Zero-warning Clippy, all workspace targets' -CargoArguments @('clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings')
    Gate -Name 'Release CLI and benchmark binaries' -CargoArguments @('build', '--release', '--locked', '-p', 'fhec-cli', '-p', 'fhec-bench', '--bins')

    $targetRoot = Join-Path $root 'target'
    if ($env:CARGO_TARGET_DIR) {
        if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
            $targetRoot = $env:CARGO_TARGET_DIR
        } else {
            $targetRoot = Join-Path $root $env:CARGO_TARGET_DIR
        }
    }
    $extension = if ($env:OS -eq 'Windows_NT') { '.exe' } else { '' }
    $exe = Join-Path $targetRoot ("release/fhec-cli$extension")
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
        Fail "Release executable not found at $exe"
    }

    $exampleFiles = @(
        'examples/conditional_mux.fhe',
        'examples/boolean_mux.fhe',
        'examples/c3/nested_selection.fhe',
        'examples/c3/signed_comparison.fhe',
        'examples/c3/wide_addition.fhe',
        'examples/c3/boolean_selection.fhe'
    )
    foreach ($relative in $exampleFiles) {
        Invoke-Example -Exe $exe -Command 'build' -RelativePath $relative -LogDir $logDir
        Invoke-Example -Exe $exe -Command 'analyze' -RelativePath $relative -LogDir $logDir
    }

    $guide = Get-Content -LiteralPath (Join-Path $root 'docs/v1-c3/FHEC_V1_COMPLETE_GUIDE.md') -Raw -Encoding UTF8
    $mandatory = @('LANGUAGE_REFERENCE.md', 'ANALYZER_AND_OPTIMIZER.md', 'BENCHMARKS_AND_CALIBRATION.md', 'SECURITY_AND_RUNTIME.md', 'DEVELOPER_INSTALL_AND_TROUBLESHOOT.md')
    foreach ($name in $mandatory) {
        if (-not $guide.Contains($name)) { Fail "Master guide lacks companion reference: $name" }
    }
    Pass 'Documentation cross references'

    if ($FastOnly) {
        $result = 'C3 FAST CHECKS PASSED; full TFHE workspace gate was intentionally skipped.'
        Write-Host $result -ForegroundColor Yellow
    } else {
        $result = 'STEP C3 VERIFIED'
        Write-Host $result -ForegroundColor Green
    }
    [System.IO.File]::WriteAllText((Join-Path $logDir 'c3-result.txt'), ($result + "`n"), (New-Object System.Text.UTF8Encoding($false)))
    Write-Host "C3 evidence: $logDir"
    Write-Host 'C3 does not create Git commits, change dependencies or publish artifacts.'
} finally {
    Set-Location -LiteralPath $previous
}
