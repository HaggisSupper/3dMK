[CmdletBinding()]
param(
    [ValidateSet('Start', 'Continue', 'Run')]
    [string]$Mode = 'Start',

    [string]$PreferredModel = 'opencode/big-pickle',

    [string]$ImplementationBranch = 'agent/vwm-authoritative-revision-implementation',

    [switch]$AllowDirty,

    [switch]$SkipCargoMetadata
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$PlanPath = Join-Path $RepositoryRoot 'docs\superpowers\plans\2026-07-31-vwm-authoritative-revision-workflow.md'
$ProgressPath = Join-Path $RepositoryRoot 'docs\agent-execution\VWM_PROGRESS.md'
$AgentPath = Join-Path $RepositoryRoot 'AGENTS.md'
$CommandPath = Join-Path $RepositoryRoot '.opencode\commands\implement-vwm.md'

function Require-Command {
    param([Parameter(Mandatory)][string]$Name)
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $command) {
        throw "Required command '$Name' is not available on PATH."
    }
    return $command
}

function Invoke-NativeChecked {
    param(
        [Parameter(Mandatory)][string]$Command,
        [Parameter()][string[]]$Arguments = @()
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "'$Command $($Arguments -join ' ')' failed with exit code $LASTEXITCODE."
    }
}

function Resolve-BigPickleModel {
    param([Parameter(Mandatory)][string]$Preferred)

    $output = & opencode models opencode --refresh 2>&1
    if ($LASTEXITCODE -ne 0) {
        $text = ($output | Out-String).Trim()
        throw "OpenCode could not list OpenCode Zen models. Connect/authenticate the 'opencode' provider first. Output: $text"
    }

    $catalogIds = foreach ($line in $output) {
        $text = [string]$line
        foreach ($match in [regex]::Matches($text, 'opencode/[A-Za-z0-9._-]+')) {
            $match.Value
        }
    }
    $catalogIds = @($catalogIds | Sort-Object -Unique)

    if ($catalogIds -contains $Preferred) {
        return $Preferred
    }

    $match = $catalogIds |
        Where-Object { $_ -match '(?i)big[-_.]?pickle' } |
        Select-Object -First 1

    if ($match) {
        return $match
    }

    $available = if ($catalogIds.Count) { $catalogIds -join ', ' } else { '<none returned>' }
    throw "Big Pickle is not available in the current OpenCode Zen catalog. Available OpenCode models: $available"
}

Push-Location $RepositoryRoot
try {
    Require-Command opencode | Out-Null
    Require-Command git | Out-Null
    Require-Command cargo | Out-Null
    Require-Command rustc | Out-Null
    Require-Command pwsh | Out-Null

    foreach ($requiredPath in @($PlanPath, $ProgressPath, $AgentPath, $CommandPath)) {
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "Required repository file is missing: $requiredPath"
        }
    }

    Invoke-NativeChecked git @('rev-parse', '--is-inside-work-tree')

    $dirty = @(git status --porcelain)
    if ($dirty.Count -gt 0 -and -not $AllowDirty) {
        throw 'The working tree is not clean. Commit/stash unrelated work or rerun with -AllowDirty only when resuming an interrupted OpenCode session.'
    }

    $currentBranch = (git branch --show-current).Trim()
    if ([string]::IsNullOrWhiteSpace($currentBranch)) {
        throw 'The repository is in detached HEAD state.'
    }

    if ($currentBranch -in @('main', 'master', 'agent/vwm-authoritative-revision-plan')) {
        $existing = git branch --list $ImplementationBranch
        if ($existing) {
            Invoke-NativeChecked git @('switch', $ImplementationBranch)
        }
        else {
            Invoke-NativeChecked git @('switch', '-c', $ImplementationBranch)
        }
    }
    elseif ($currentBranch -ne $ImplementationBranch) {
        throw "Current branch '$currentBranch' is not an approved VWM execution branch. Switch to '$ImplementationBranch' or the plan branch first."
    }

    if (-not $SkipCargoMetadata) {
        Invoke-NativeChecked cargo @('metadata', '--no-deps', '--format-version', '1')
    }

    $resolvedModel = Resolve-BigPickleModel -Preferred $PreferredModel

    Write-Host "Repository: $RepositoryRoot"
    Write-Host "Branch: $(git branch --show-current)"
    Write-Host "OpenCode: $(& opencode --version)"
    Write-Host "Model: $resolvedModel"
    Write-Warning 'Big Pickle is a free OpenCode Zen stealth model. During its free period, submitted data may be used to improve the model. Do not expose secrets or confidential third-party data.'

    $prompt = @"
Read AGENTS.md, docs/agent-execution/VWM_PROGRESS.md,
docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md,
and .opencode/commands/implement-vwm.md.

Execute the repository's authoritative VWM workflow from the first incomplete task.
Use the vwm-reviewer and vwm-verifier gates after every task.
Continue until PROJECT_COMPLETE, a genuine BLOCKED state, or a clean SESSION_BOUNDARY.
"@

    $common = @('--model', $resolvedModel, '--agent', 'vwm-executor', '--auto')

    switch ($Mode) {
        'Start' {
            & opencode '.' @common '--prompt' $prompt
        }
        'Continue' {
            & opencode '.' '--continue' @common
        }
        'Run' {
            & opencode run @common $prompt
        }
    }

    if ($LASTEXITCODE -ne 0) {
        throw "OpenCode exited with code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}
