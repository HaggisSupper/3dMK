[CmdletBinding()]
param(
    [Parameter()]
    [string] $RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$requiredFiles = @(
    "AGENTS.md",
    "docs/CURRENT_STATE.md",
    ".codex/skills/3dmk-dev/SKILL.md",
    ".codex/skills/3dmk-dev/references/authority-map.md",
    ".codex/skills/3dmk-dev/references/verification-matrix.md"
)

$missing = @(
    $requiredFiles | Where-Object {
        -not (Test-Path -LiteralPath (Join-Path $RepositoryRoot $_) -PathType Leaf)
    }
)

if ($missing.Count -gt 0) {
    throw "Missing required 3DMK toolkit files: $($missing -join ', ')"
}

$skill = Get-Content -LiteralPath (Join-Path $RepositoryRoot ".codex/skills/3dmk-dev/SKILL.md") -Raw
$requiredTerms = @(
    "## Trigger",
    "## Authority boundary",
    "## Deterministic evidence",
    "## Workflow",
    "admit → stage → execute → validate → publish → finalize",
    "## Output artifact",
    "## Stop condition"
)

foreach ($term in $requiredTerms) {
    if ($skill.IndexOf($term, [StringComparison]::Ordinal) -lt 0) {
        throw "SKILL.md is missing required term: $term"
    }
}

$forbidden = @("TODO", "FIXME", "Docker", "Podman", "WSL", "Electron")
foreach ($term in $forbidden) {
    if ($skill -match "(?i)\b$([regex]::Escape($term))\b" -and $term -notin @("Docker", "Podman", "WSL", "Electron")) {
        throw "Unexpected placeholder in toolkit: $term"
    }
}

Write-Output "3DMK toolkit validation: PASS"
