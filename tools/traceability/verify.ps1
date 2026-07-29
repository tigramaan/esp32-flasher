param(
    [string]$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path,
    [switch]$Json
)

$ErrorActionPreference = "Stop"

function Get-UniqueMatches {
    param(
        [string]$Text,
        [string]$Pattern
    )

    return [regex]::Matches($Text, $Pattern, [Text.RegularExpressions.RegexOptions]::Multiline) |
        ForEach-Object { $_.Groups[1].Value } |
        Sort-Object -Unique
}

$specPath = Join-Path $ProjectRoot "specs\001-esp32-programmer\spec.md"
$matrixPath = Join-Path $ProjectRoot "specs\TRACEABILITY_MATRIX.md"

if (-not (Test-Path -LiteralPath $specPath -PathType Leaf)) {
    throw "Specification not found: $specPath"
}
if (-not (Test-Path -LiteralPath $matrixPath -PathType Leaf)) {
    throw "Traceability matrix not found: $matrixPath"
}

$specText = Get-Content -LiteralPath $specPath -Raw -Encoding UTF8
$matrixText = Get-Content -LiteralPath $matrixPath -Raw -Encoding UTF8
$specRequirements = @(Get-UniqueMatches -Text $specText -Pattern "\*\*(REQ-\d{3})\*\*")
$matrixRequirements = @(Get-UniqueMatches -Text $matrixText -Pattern "^\|\s*(REQ-\d{3})\s*\|")
$missing = @($specRequirements | Where-Object { $_ -notin $matrixRequirements })
$unexpected = @($matrixRequirements | Where-Object { $_ -notin $specRequirements })
$incompleteRows = @()

foreach ($line in ($matrixText -split "\r?\n")) {
    if ($line -notmatch "^\|\s*REQ-\d{3}\s*\|") {
        continue
    }
    $columns = @($line.Split("|") | ForEach-Object { $_.Trim() })
    if ($columns.Count -lt 6 -or [string]::IsNullOrWhiteSpace($columns[2]) -or
        [string]::IsNullOrWhiteSpace($columns[3]) -or
        [string]::IsNullOrWhiteSpace($columns[4])) {
        $incompleteRows += $line
    }
}

$passed = $specRequirements.Count -gt 0 -and
    $missing.Count -eq 0 -and
    $unexpected.Count -eq 0 -and
    $incompleteRows.Count -eq 0
$result = [ordered]@{
    passed = $passed
    requirement_count = $specRequirements.Count
    matrix_count = $matrixRequirements.Count
    missing = $missing
    unexpected = $unexpected
    incomplete_rows = $incompleteRows
}

if ($Json) {
    $result | ConvertTo-Json -Depth 3
} else {
    Write-Output "Traceability: $($specRequirements.Count)/$($matrixRequirements.Count) requirements"
    if ($passed) {
        Write-Output "PASS"
    } else {
        $result | Format-List | Out-String | Write-Output
    }
}

if (-not $passed) {
    exit 1
}
