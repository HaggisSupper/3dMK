[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$requiredFiles = @(
    'AGENTS.md',
    '.gitignore',
    'docs/README.md',
    'docs/CURRENT_STATE.md',
    'docs/agent-execution/README.md',
    'docs/agent-execution/MISTRALRS_LOCAL_AGENT.md',
    'docs/agent-execution/VWM_PROGRESS.md',
    'docs/agent-execution/CUDA_FOUNDATION_PROGRESS.md',
    'docs/superpowers/specs/2026-08-07-3dmk-world-class-quality-standard.md',
    'docs/superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md',
    'docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md',
    'docs/superpowers/plans/2026-08-07-3dmk-foundation-batch.md',
    'scripts/setup-mistralrs-local-agent.ps1',
    'scripts/run-mistralrs-vwm-task.ps1',
    'scripts/mistralrs-agent/common.ps1',
    'scripts/mistralrs-agent/roles.ps1',
    'scripts/test-mistralrs-local-agent-harness.ps1',
    '.github/workflows/mistralrs-harness-validation.yml'
)

$forbiddenFiles = @(
    'opencode.json',
    '.github/workflows/opencode-big-pickle.yml',
    '.opencode/agents/vwm-executor.md',
    '.opencode/agents/vwm-reviewer.md',
    '.opencode/agents/vwm-verifier.md',
    '.opencode/commands/implement-vwm.md',
    'scripts/start-opencode-vwm.ps1',
    'scripts/test-opencode-vwm-harness.ps1',
    'docs/superpowers/plans/2026-07-31-opencode-vwm-execution-harness.md',
    'docs/superpowers/specs/2026-07-31-opencode-vwm-execution-harness-design.md',
    'docs/superpowers/plans/2026-08-07-mistralrs-local-agent-experiment.md',
    'docs/superpowers/specs/2026-08-07-mistralrs-local-agent-experiment-design.md'
)

function Resolve-RepositoryFile {
    param([Parameter(Mandatory)][string]$RelativePath)
    Join-Path $RepositoryRoot ($RelativePath -replace '/', [IO.Path]::DirectorySeparatorChar)
}

function Read-RepositoryFile {
    param([Parameter(Mandatory)][string]$RelativePath)
    $path = Resolve-RepositoryFile -RelativePath $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required harness file is missing: $RelativePath"
    }
    Get-Content -LiteralPath $path -Raw
}

function Assert-ContainsAll {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Content,
        [Parameter(Mandatory)][string[]]$Required
    )
    foreach ($value in $Required) {
        if (-not $Content.Contains($value, [StringComparison]::Ordinal)) {
            throw "$Name is missing required contract text: $value"
        }
    }
}

function Assert-ContainsNone {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Content,
        [Parameter(Mandatory)][string[]]$Forbidden
    )
    foreach ($value in $Forbidden) {
        if ($Content.Contains($value, [StringComparison]::OrdinalIgnoreCase)) {
            throw "$Name contains prohibited contract text: $value"
        }
    }
}

function Assert-PowerShellParses {
    param([Parameter(Mandatory)][string]$RelativePath)
    $path = Resolve-RepositoryFile -RelativePath $RelativePath
    $tokens = $null
    $parseErrors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile(
        $path,
        [ref]$tokens,
        [ref]$parseErrors
    )
    if (-not $ast -or $parseErrors.Count -gt 0) {
        $details = $parseErrors | ForEach-Object {
            "line $($_.Extent.StartLineNumber), column $($_.Extent.StartColumnNumber): $($_.Message)"
        }
        throw "$RelativePath has PowerShell parse errors:`n$($details -join [Environment]::NewLine)"
    }

    $commandNames = $ast.FindAll(
        { param($node) $node -is [System.Management.Automation.Language.CommandAst] },
        $true
    ) | ForEach-Object { $_.GetCommandName() } | Where-Object { $_ }

    foreach ($forbiddenCommand in @('docker', 'podman', 'wsl', 'opencode')) {
        if ($commandNames -contains $forbiddenCommand) {
            throw "$RelativePath directly invokes prohibited executor command: $forbiddenCommand"
        }
    }
}

function Invoke-NativeChecked {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter(ValueFromRemainingArguments)][string[]]$Arguments
    )
    $output = & $FilePath @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath $($Arguments -join ' ') failed:`n$($output | Out-String)"
    }
    @($output)
}

foreach ($relativePath in $requiredFiles) {
    if (-not (Test-Path -LiteralPath (Resolve-RepositoryFile $relativePath) -PathType Leaf)) {
        throw "Required harness file is missing: $relativePath"
    }
}
foreach ($relativePath in $forbiddenFiles) {
    if (Test-Path -LiteralPath (Resolve-RepositoryFile $relativePath)) {
        throw "Superseded executor file is still present: $relativePath"
    }
}

foreach ($script in @(
    'scripts/setup-mistralrs-local-agent.ps1',
    'scripts/run-mistralrs-vwm-task.ps1',
    'scripts/mistralrs-agent/common.ps1',
    'scripts/mistralrs-agent/roles.ps1',
    'scripts/test-mistralrs-local-agent-harness.ps1'
)) {
    Assert-PowerShellParses -RelativePath $script
}

$setup = Read-RepositoryFile 'scripts/setup-mistralrs-local-agent.ps1'
Assert-ContainsAll 'Mistral.rs setup' $setup @(
    "[string]`$Version = 'master'",
    '[switch]$ForceSourceBuild',
    'nvidia-smi',
    'nvcc',
    'Visual Studio 2022 C++ Build Tools',
    "`$featureSet = 'cuda flash-attn cudnn'",
    "`$featureSet = 'cuda'",
    'cuda-install.json',
    'Get-FileHash',
    'Assert-CudaDoctorEvidence',
    'CUDA-enabled Mistral.rs is ready'
)
Assert-ContainsNone 'Mistral.rs setup' $setup @(
    'AllowCpuFallback',
    'Install-CpuFallback',
    'MISTRALRS_INSTALL_TAG',
    'degraded CPU',
    'CPU fallback'
)

$runner = Read-RepositoryFile 'scripts/run-mistralrs-vwm-task.ps1'
Assert-ContainsAll 'Mistral.rs controller' $runner @(
    "[CmdletBinding(DefaultParameterSetName = 'Authoritative')]",
    "[ValidateSet('FB1', 'FB2', 'FB3', 'FB4', 'FB5', 'FB6', 'FB7', 'FB8')]",
    "[string]`$Model = 'Qwen/Qwen3-Coder-30B-A3B-Instruct'",
    "[string]`$FallbackModel = 'Qwen/Qwen3-8B'",
    '[int]$ContextLength = 32768',
    '[int]$MaxRepairRounds = 3',
    "`$Branch = 'agent/vwm-authoritative-revision-implementation'",
    '$TaskId',
    '$TaskPlan',
    '$TaskLedger',
    'Assert-TaskDependencies',
    'CUDA_FOUNDATION_PROGRESS.md',
    '2026-08-07-3dmk-foundation-batch.md',
    'Invoke-Implementer',
    'Invoke-Reviewer',
    'Invoke-Verifier',
    'TASK_CANDIDATE',
    'CUDA_RUNTIME: VERIFIED'
)
Assert-ContainsNone 'Mistral.rs controller' $runner @('AllowCpuFallback', 'CPU fallback')

$dependencyIndex = $runner.IndexOf('Assert-TaskDependencies -Worktree $worktree', [StringComparison]::Ordinal)
$cudaSetupIndex = $runner.IndexOf('Ensure-MistralRs -RepositoryRoot $RepositoryRoot', [StringComparison]::Ordinal)
if ($dependencyIndex -lt 0 -or $cudaSetupIndex -lt 0 -or $dependencyIndex -gt $cudaSetupIndex) {
    throw 'The controller must reject unmet task dependencies before CUDA/Mistral.rs setup begins.'
}

$common = Read-RepositoryFile 'scripts/mistralrs-agent/common.ps1'
Assert-ContainsAll 'Mistral.rs common module' $common @(
    'Test-IsLinkedWorktree',
    'function Test-GitAncestor',
    'function Sync-ImplementationBranchWithMain',
    'refs/remotes/origin/$BaseBranch',
    'pull --ff-only origin $BaseBranch',
    'BLOCKED: implementation branch has diverged from origin/',
    'Sync-ImplementationBranchWithMain -Worktree',
    'Assert-CleanImplementationBranch',
    'Assert-CudaDoctorEvidence',
    'Assert-CudaProcess',
    'cuda-runtime-evidence.json',
    '--query-compute-apps=pid,process_name,used_gpu_memory',
    "'--host', '127.0.0.1'",
    "'--enable-shell'",
    "'--shell-workdir'",
    "'--no-ui'",
    '/v1/models',
    '/v1/responses'
)
Assert-ContainsNone 'Mistral.rs common module' $common @('AllowCpuFallback', 'degraded CPU', "'--cpu'")

$roles = Read-RepositoryFile 'scripts/mistralrs-agent/roles.ps1'
Assert-ContainsAll 'Mistral.rs role module' $roles @(
    'function Invoke-Implementer',
    'function Invoke-Reviewer',
    'function Invoke-Verifier',
    'function Invoke-Repair',
    'function Publish-VerifiedBranch',
    '$TaskId',
    '$TaskPlan',
    '$TaskLedger',
    'VERDICT: PASS',
    'VERDICT: FAIL',
    'cuda-runtime-evidence.json',
    "'--draft'"
)
Assert-ContainsNone 'Mistral.rs role module' $roles @('$AuthoritativePlan', '$ProgressLedger')

$scriptCorpus = @($setup, $runner, $common, $roles) -join [Environment]::NewLine
foreach ($pattern in @(
    '(?im)^\s*Invoke-Git\b[^\r\n]*\bpush\s+--force(?:-with-lease)?\b',
    '(?im)^\s*Invoke-Git\b[^\r\n]*\bmerge\b',
    '(?im)^\s*Invoke-Git\b[^\r\n]*\breset\s+--hard\b',
    '(?im)^\s*Invoke-Git\b[^\r\n]*\bclean\b'
)) {
    if ($scriptCorpus -match $pattern) {
        throw "The local-agent scripts contain a prohibited git invocation matching: $pattern"
    }
}

$agents = Read-RepositoryFile 'AGENTS.md'
Assert-ContainsAll 'AGENTS.md' $agents @(
    '# 3DMk Agent Contract',
    'docs/CURRENT_STATE.md',
    'docs/agent-execution/MISTRALRS_LOCAL_AGENT.md',
    'CUDA Foundation FB1–FB8',
    'separate model sessions',
    'Reviewer and verifier sessions are read-only',
    '- Before model startup, `origin/main` must be an ancestor of the active implementation branch.',
    'CPU LLM inference and cloud inference fallback are prohibited.',
    'agent/vwm-authoritative-revision-implementation',
    'TASK_CANDIDATE',
    'Evidence before assertion. No exceptions.'
)

$runbook = Read-RepositoryFile 'docs/agent-execution/MISTRALRS_LOCAL_AGENT.md'
Assert-ContainsAll 'Mistral.rs runbook' $runbook @(
    'run-mistralrs-vwm-task.ps1',
    '-FoundationTask FB1',
    'Mainline admission',
    'origin/main',
    'fast-forwards the branch when it is strictly behind',
    'blocks before model startup when the refs have diverged',
    'Qwen/Qwen3-Coder-30B-A3B-Instruct',
    'Qwen/Qwen3-8B',
    'Qwen/Qwen3-4B',
    'CPU inference and cloud inference fallback are prohibited.',
    'cuda-runtime-evidence.json',
    '127.0.0.1',
    'VERDICT: PASS',
    'VERDICT: FAIL',
    '.local-agent/task-XX/'
)
Assert-ContainsNone 'Mistral.rs runbook' $runbook @('AllowCpuFallback', 'degraded CPU mode')

$currentState = Read-RepositoryFile 'docs/CURRENT_STATE.md'
Assert-ContainsAll 'Current-state branch admission' $currentState @(
    '**Branch admission policy:**',
    'origin/main',
    'fast-forwards the branch when it is strictly behind',
    'blocks on divergence'
)
Assert-ContainsNone 'Current-state branch admission' $currentState @(
    'is synchronized with current `main`',
    'subsequent merged changes are documentation-only'
)

$executionReadme = Read-RepositoryFile 'docs/agent-execution/README.md'
Assert-ContainsAll 'Agent execution README' $executionReadme @(
    '-FoundationTask FB1',
    'Task 6 cannot begin',
    'CUDA_FOUNDATION_PROGRESS.md'
)

$gitignore = Read-RepositoryFile '.gitignore'
Assert-ContainsAll '.gitignore' $gitignore @('/.local-agent/', '/.worktrees/', '/.opencode/')

$workflow = Read-RepositoryFile '.github/workflows/mistralrs-harness-validation.yml'
Assert-ContainsAll 'Mistral.rs validation workflow' $workflow @(
    'runs-on: windows-latest',
    'scripts/test-mistralrs-local-agent-harness.ps1',
    'pwsh'
)

$commonPath = Resolve-RepositoryFile 'scripts/mistralrs-agent/common.ps1'
. $commonPath
$git = (Get-Command git -ErrorAction Stop).Source
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("3dmk-mainline-admission-{0}" -f [Guid]::NewGuid().ToString('N'))
$origin = Join-Path $tempRoot 'origin.git'
$seed = Join-Path $tempRoot 'seed'
$work = Join-Path $tempRoot 'work'
try {
    New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
    Invoke-NativeChecked $git init --bare $origin | Out-Null
    Invoke-NativeChecked $git init $seed | Out-Null
    Invoke-NativeChecked $git -C $seed config user.name '3DMk Harness'
    Invoke-NativeChecked $git -C $seed config user.email '3dmk-harness@example.invalid'
    Set-Content -LiteralPath (Join-Path $seed 'state.txt') -Value 'base' -Encoding utf8
    Invoke-NativeChecked $git -C $seed add state.txt | Out-Null
    Invoke-NativeChecked $git -C $seed commit -m 'base' | Out-Null
    Invoke-NativeChecked $git -C $seed branch -M main | Out-Null
    Invoke-NativeChecked $git -C $seed remote add origin $origin | Out-Null
    Invoke-NativeChecked $git -C $seed push -u origin main | Out-Null
    Invoke-NativeChecked $git -C $seed switch -c $Branch | Out-Null
    Invoke-NativeChecked $git -C $seed push -u origin $Branch | Out-Null
    Invoke-NativeChecked $git clone --branch $Branch $origin $work | Out-Null
    Invoke-NativeChecked $git -C $work config user.name '3DMk Harness'
    Invoke-NativeChecked $git -C $work config user.email '3dmk-harness@example.invalid'

    Invoke-NativeChecked $git -C $seed switch main | Out-Null
    Set-Content -LiteralPath (Join-Path $seed 'main-a.txt') -Value 'main-a' -Encoding utf8
    Invoke-NativeChecked $git -C $seed add main-a.txt | Out-Null
    Invoke-NativeChecked $git -C $seed commit -m 'main-a' | Out-Null
    Invoke-NativeChecked $git -C $seed push origin main | Out-Null

    Sync-ImplementationBranchWithMain -Worktree $work
    $head = (Invoke-NativeChecked $git -C $work rev-parse HEAD | Select-Object -First 1).Trim()
    $main = (Invoke-NativeChecked $git -C $work rev-parse origin/main | Select-Object -First 1).Trim()
    if ($head -ne $main) {
        throw "Mainline admission did not fast-forward a strictly behind branch. HEAD=$head origin/main=$main"
    }

    Set-Content -LiteralPath (Join-Path $work 'branch-only.txt') -Value 'branch' -Encoding utf8
    Invoke-NativeChecked $git -C $work add branch-only.txt | Out-Null
    Invoke-NativeChecked $git -C $work commit -m 'branch-only' | Out-Null

    Set-Content -LiteralPath (Join-Path $seed 'main-b.txt') -Value 'main-b' -Encoding utf8
    Invoke-NativeChecked $git -C $seed add main-b.txt | Out-Null
    Invoke-NativeChecked $git -C $seed commit -m 'main-b' | Out-Null
    Invoke-NativeChecked $git -C $seed push origin main | Out-Null

    $divergenceObserved = $false
    try {
        Sync-ImplementationBranchWithMain -Worktree $work
    }
    catch {
        $divergenceObserved = $_.Exception.Message -like 'BLOCKED: implementation branch has diverged from origin/*'
    }
    if (-not $divergenceObserved) {
        throw 'Mainline admission did not fail closed when the implementation branch diverged from origin/main.'
    }
}
finally {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host 'MISTRALRS_LOCAL_AGENT_HARNESS_OK'
