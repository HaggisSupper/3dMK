[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path

function Invoke-PreparedRepair {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Content
    )

    $temporaryPath = Join-Path ([IO.Path]::GetTempPath()) ("3dmk-{0}-{1}.ps1" -f $Name, [Guid]::NewGuid().ToString('N'))
    try {
        Set-Content -LiteralPath $temporaryPath -Value $Content -Encoding utf8 -NoNewline
        & pwsh -NoLogo -NoProfile -File $temporaryPath
        if ($LASTEXITCODE -ne 0) {
            throw "$Name repair failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Remove-Item -LiteralPath $temporaryPath -Force -ErrorAction SilentlyContinue
    }
}

$integrationPath = Join-Path $RepositoryRoot 'scripts\repair-spatial-vwm-integration.ps1'
$integration = Get-Content -LiteralPath $integrationPath -Raw
$integration = $integration.Replace(
    '[Parameter(Mandatory)][string]$New',
    '[Parameter(Mandatory)][AllowEmptyString()][string]$New'
)
$integration = $integration.Replace(
    'fn local_vlm_completion_url(endpoint: &str) -> Result<reqwest::Url> {',
    'fn local_vlm_completion_url(endpoint: &str) -> Result<String> {'
)
$integration = $integration.Replace(
    "    match url.path().trim_end_matches('/') {",
    "    let normalized_path = url.path().trim_end_matches('/').to_owned();`n    match normalized_path.as_str() {"
)
$integration = $integration.Replace(
    "    Ok(url)`n}`n`npub async fn detect_objects_vlm(",
    "    Ok(url.to_string())`n}`n`npub async fn detect_objects_vlm("
)
$integrationTail = @'

Push-Location $RepositoryRoot
try {
    cargo fmt --all
    if ($LASTEXITCODE -ne 0) { throw "Root rustfmt failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

Write-Host '3DMK_SPATIAL_VWM_INTEGRATION_REPAIR_OK'
'@
$integration = [regex]::Replace(
    $integration,
    '(?s)\r?\nPush-Location \$RepositoryRoot\r?\ntry \{.*\z',
    $integrationTail
)
Invoke-PreparedRepair -Name 'security-compatibility' -Content $integration

$qualityPath = Join-Path $RepositoryRoot 'scripts\repair-spatial-vwm-quality.ps1'
$quality = Get-Content -LiteralPath $qualityPath -Raw
$quality = [regex]::Replace(
    $quality,
    '(?s)\r?\n\$workflowPath = ''.github/workflows/advanced-feature-integration.yml''.*?Write-SourceFile \$workflowPath \$workflow\r?\n',
    "`n"
)
$qualityTail = @'

Push-Location $RepositoryRoot
try {
    cargo fmt --all
    if ($LASTEXITCODE -ne 0) { throw "Root rustfmt failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

Write-Host '3DMK_SPATIAL_VWM_QUALITY_REPAIR_OK'
'@
$quality = [regex]::Replace(
    $quality,
    '(?s)\r?\nPush-Location \$RepositoryRoot\r?\ntry \{.*\z',
    $qualityTail
)
Invoke-PreparedRepair -Name 'quality' -Content $quality

Write-Host '3DMK_SPATIAL_VWM_SOURCE_REPAIR_OK'
