[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
)

$ErrorActionPreference = 'Stop'
$archiveText = Join-Path $PSScriptRoot 'spatial-engineering-platform.tar.gz.b64'
$tempArchive = Join-Path ([System.IO.Path]::GetTempPath()) 'spatial-engineering-platform.tar.gz'
$target = Join-Path $RepositoryRoot 'spatial-engineering-platform'

if (-not (Test-Path $archiveText)) {
    throw "Missing archive payload: $archiveText"
}

if (Test-Path $target) {
    $existing = Get-ChildItem -LiteralPath $target -Force | Where-Object { $_.Name -ne 'bootstrap' }
    if ($existing) {
        throw "Target already contains extracted platform files: $target"
    }
}

$base64 = (Get-Content -LiteralPath $archiveText -Raw).Trim()
[System.IO.File]::WriteAllBytes($tempArchive, [Convert]::FromBase64String($base64))

try {
    tar -xzf $tempArchive -C $RepositoryRoot
    if ($LASTEXITCODE -ne 0) {
        throw "tar extraction failed with exit code $LASTEXITCODE"
    }
}
finally {
    Remove-Item -LiteralPath $tempArchive -Force -ErrorAction SilentlyContinue
}

Write-Host "Spatial Engineering Platform extracted to $target"
Write-Host "Run: python spatial-engineering-platform\scripts\verify_repo.py"
