$ErrorActionPreference = "Stop"

$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$manifestPath = Join-Path $root "tests\fixtures\manifest.json"
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json

foreach ($fixture in $manifest.fixtures) {
    $path = Join-Path $root ($fixture.path -replace "/", "\")
    $item = Get-Item -LiteralPath $path
    if ($item.Length -ne $fixture.bytes) {
        throw "$($fixture.id): expected $($fixture.bytes) bytes, found $($item.Length)"
    }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    if ($actual -ne $fixture.sha256) {
        throw "$($fixture.id): SHA-256 mismatch"
    }
}

Write-Output "Verified $($manifest.fixtures.Count) fixture identities."
