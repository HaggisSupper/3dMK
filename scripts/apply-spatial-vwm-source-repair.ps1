[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$visionPath = Join-Path $RepositoryRoot 'src\ai_vision.rs'
$vision = Get-Content -LiteralPath $visionPath -Raw

if (-not $vision.Contains('fn local_vlm_completion_url(endpoint: &str) -> Result<String>', [StringComparison]::Ordinal)) {
    throw 'The loopback-only VLM URL contract is missing; the bounded integration repair did not apply.'
}

$urlReturnPattern = [regex]'(?s)(fn local_vlm_completion_url\(endpoint: &str\) -> Result<String> \{.*?\r?\n\s*)Ok\(url\)(\r?\n\})'
$patchedVision = $urlReturnPattern.Replace(
    $vision,
    '${1}Ok(url.to_string())${2}',
    1
)
if ($patchedVision -ne $vision) {
    Set-Content -LiteralPath $visionPath -Value $patchedVision -Encoding utf8 -NoNewline
}
elseif (-not $vision.Contains('Ok(url.to_string())', [StringComparison]::Ordinal)) {
    throw 'The VLM URL helper has neither the expected Url return nor the required String return.'
}

Push-Location $RepositoryRoot
try {
    cargo fmt --all
    if ($LASTEXITCODE -ne 0) {
        throw "Root rustfmt failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

Write-Host '3DMK_SPATIAL_VWM_SOURCE_REPAIR_OK'
