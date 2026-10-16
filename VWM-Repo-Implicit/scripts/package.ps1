$ErrorActionPreference = "Stop"

& "$PSScriptRoot\check.ps1"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$zip = Join-Path $root "vwm-workspace-$stamp.zip"

if (Test-Path $zip) {
    Remove-Item $zip -Force
}

$items = Get-ChildItem $root -Force |
    Where-Object {
        $_.Name -notin @("target", ".git") -and
        $_.Extension -ne ".zip"
    }

Compress-Archive -Path $items.FullName -DestinationPath $zip -Force
Write-Host "Wrote $zip"
