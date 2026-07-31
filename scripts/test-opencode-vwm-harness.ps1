[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$required = @(
    'AGENTS.md',
    'opencode.json',
    '.opencode\agents\vwm-executor.md',
    '.opencode\agents\vwm-reviewer.md',
    '.opencode\agents\vwm-verifier.md',
    '.opencode\commands\implement-vwm.md',
    'docs\agent-execution\README.md',
    'docs\agent-execution\VWM_PROGRESS.md',
    'docs\superpowers\plans\2026-07-31-vwm-authoritative-revision-workflow.md',
    'scripts\start-opencode-vwm.ps1'
)

foreach ($relative in $required) {
    $path = Join-Path $RepositoryRoot $relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing required OpenCode harness file: $relative"
    }
}

$configPath = Join-Path $RepositoryRoot 'opencode.json'
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json

if ($config.default_agent -ne 'vwm-executor') {
    throw 'opencode.json default_agent must be vwm-executor.'
}

$requiredInstructions = @(
    'AGENTS.md',
    'docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md',
    'docs/agent-execution/VWM_PROGRESS.md'
)
foreach ($instruction in $requiredInstructions) {
    if ($config.instructions -notcontains $instruction) {
        throw "opencode.json is missing instruction: $instruction"
    }
}

$progress = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'docs\agent-execution\VWM_PROGRESS.md') -Raw
for ($task = 1; $task -le 17; $task++) {
    if ($progress -notmatch "(?m)^- \[[ xX]\] \*\*Task $task:") {
        throw "Progress ledger is missing Task $task."
    }
}

$agents = @(
    '.opencode\agents\vwm-reviewer.md',
    '.opencode\agents\vwm-verifier.md'
)
foreach ($relative in $agents) {
    $content = Get-Content -LiteralPath (Join-Path $RepositoryRoot $relative) -Raw
    if ($content -notmatch '(?m)^\s*edit:\s*deny\s*$') {
        throw "$relative must deny edits."
    }
}

$forbiddenRequired = @(
    '"git switch main*": "deny"',
    '"git checkout main*": "deny"',
    '"git push origin main*": "deny"',
    '"git push origin master*": "deny"',
    '"git push --force*": "deny"',
    '"git merge*": "deny"',
    '"git reset --hard*": "deny"',
    '"git clean*": "deny"',
    '"docker *": "deny"',
    '"podman *": "deny"',
    '"wsl *": "deny"'
)
$configRaw = Get-Content -LiteralPath $configPath -Raw
foreach ($rule in $forbiddenRequired) {
    if (-not $configRaw.Contains($rule)) {
        throw "opencode.json is missing required denial rule: $rule"
    }
}

$agentsContract = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'AGENTS.md') -Raw
foreach ($state in @('TASK_COMPLETE', 'BATCH_COMPLETE', 'SESSION_BOUNDARY', 'BLOCKED', 'PROJECT_COMPLETE')) {
    if (-not $agentsContract.Contains($state)) {
        throw "AGENTS.md is missing completion state: $state"
    }
}

Write-Host 'OpenCode VWM harness static checks passed.'
