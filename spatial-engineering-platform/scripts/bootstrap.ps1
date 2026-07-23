[CmdletBinding()]
param([switch]$Desktop)
$ErrorActionPreference='Stop'
$Root=Split-Path -Parent $PSScriptRoot
Write-Host "Verifying Python reference implementation..."
py -3 -m unittest discover -s (Join-Path $Root 'reference_py/tests') -v
py -3 (Join-Path $Root 'scripts/verify_repo.py')
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Warning 'Rust is not installed. Install Rust 1.78+ with rustup, then rerun this script.'
  exit 0
}
Push-Location $Root
try { cargo test --workspace }
finally { Pop-Location }
if ($Desktop) {
  if (-not (Get-Command npm -ErrorAction SilentlyContinue)) { throw 'npm is required for the desktop shell.' }
  Push-Location (Join-Path $Root 'apps/desktop')
  try { npm install; npm run build }
  finally { Pop-Location }
}
