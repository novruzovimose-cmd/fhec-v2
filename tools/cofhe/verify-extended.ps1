param(
    [string]$Project = "$env:USERPROFILE\Documents\fhec-v2",
    [string]$Reference = "$env:USERPROFILE\Documents\fhec-cofhe-reference"
)

$ErrorActionPreference = "Stop"

$projectManifest = Join-Path $Project "Cargo.toml"
$inputSource = Join-Path $Project "examples\cofhe_extended.fhe"
$referenceManifest = Join-Path $Reference "hardhat.config.ts"
$referenceGenerated = Join-Path $Reference "contracts\FHECGenerated.sol"
$referenceTest = Join-Path $Reference "test\FHECComparisonDemo.test.ts"

foreach ($required in @($projectManifest, $inputSource, $referenceManifest, $referenceGenerated, $referenceTest)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Required file missing: $required"
    }
}

foreach ($toolName in @("cargo", "pnpm")) {
    if (-not (Get-Command $toolName -ErrorAction SilentlyContinue)) {
        throw "Required tool unavailable: $toolName"
    }
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$runDirectory = Join-Path $Project "target\cofhe-extended-$stamp-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
New-Item -ItemType Directory -Path $runDirectory -ErrorAction Stop | Out-Null

$generated = Join-Path $runDirectory "FHECGenerated.sol"
$original = Join-Path $runDirectory "FHECGenerated.original.sol"

Copy-Item -LiteralPath $referenceGenerated -Destination $original -ErrorAction Stop
$originalHash = (Get-FileHash -LiteralPath $original -Algorithm SHA256).Hash
$hasReplaced = $false
$passed = $false

try {
    Set-Location -LiteralPath $Project

    cargo test --locked -p fhec-backend-cofhe
    if ($LASTEXITCODE -ne 0) { throw "CoFHE backend tests failed." }

    cargo test --locked -p fhec-cli
    if ($LASTEXITCODE -ne 0) { throw "FHEC CLI regression tests failed." }

    cargo run --locked -p fhec-cli -- cofhe $inputSource --out $generated
    if ($LASTEXITCODE -ne 0) { throw "CoFHE extended source generation failed." }

    if (-not (Test-Path -LiteralPath $generated -PathType Leaf)) {
        throw "Expected generated Solidity missing."
    }

    $generatedContent = Get-Content -LiteralPath $generated -Raw
    foreach ($requiredText in @("function pick_greater(", "function add_u16(", "function multiply_u32(", "function greater_u64(", "function four_times_u8(")) {
        if (-not $generatedContent.Contains($requiredText)) {
            throw "Generated Solidity omitted expected function: $requiredText"
        }
    }

    $hasReplaced = $true
    Copy-Item -LiteralPath $generated -Destination $referenceGenerated -Force -ErrorAction Stop

    Set-Location -LiteralPath $Reference

    pnpm compile
    if ($LASTEXITCODE -ne 0) { throw "Fhenix Solidity compilation failed." }

    pnpm exec hardhat test test/FHECComparisonDemo.test.ts
    if ($LASTEXITCODE -ne 0) { throw "CoFHE encrypted comparison regression failed." }

    $passed = $true
}
finally {
    if ($hasReplaced) {
        Copy-Item -LiteralPath $original -Destination $referenceGenerated -Force -ErrorAction Stop
        $restoredHash = (Get-FileHash -LiteralPath $referenceGenerated -Algorithm SHA256).Hash
        if ($restoredHash -ne $originalHash) {
            throw "Failed to verify restoration of Fhenix reference generated source."
        }
    }
}

if ($passed) {
    Write-Host "FHEC V2 EXTENDED COFHE VALIDATION PASSED" -ForegroundColor Green
    Write-Host "Generated source: $generated" -ForegroundColor Cyan
    Write-Host "Reference generated source was restored and SHA-256 verified." -ForegroundColor Cyan
}
