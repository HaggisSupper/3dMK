[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$requiredFiles = @(
    'AGENTS.md',
    '.gitignore',
    'docs/agent-execution/MISTRALRS_LOCAL_AGENT.md',
    'docs/superpowers/specs/2026-08-07-mistralrs-local-agent-experiment-design.md',
    'docs/superpowers/plans/2026-08-07-mistralrs-local-agent-experiment.md',
    'scripts/setup-mistralrs-local-agent.ps1',
    'scripts/run-mistralrs-vwm-task.ps1',
    'scripts/mistralrs-agent/common.ps1',
    'scripts/mistralrs-agent/roles.ps1',
    'scripts/test-mistralrs-local-agent-harness.ps1',
    '.github/workflows/mistralrs-harness-validation.yml'
)

function Resolve-RepositoryFile {
    param([Parameter(Mandatory)][string]$RelativePath)
    return Join-Path $RepositoryRoot ($RelativePath -replace '/', [IO.Path]::DirectorySeparatorChar)
}

function Read-RepositoryFile {
    param([Parameter(Mandatory)][string]$RelativePath)
    $path = Resolve-RepositoryFile -RelativePath $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required harness file is missing: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw
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

foreach ($relativePath in $requiredFiles) {
    $path = Resolve-RepositoryFile -RelativePath $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required harness file is missing: $relativePath"
    }
}

Assert-PowerShellParses -RelativePath 'scripts/setup-mistralrs-local-agent.ps1'
Assert-PowerShellParses -RelativePath 'scripts/run-mistralrs-vwm-task.ps1'
Assert-PowerShellParses -RelativePath 'scripts/mistralrs-agent/common.ps1'
Assert-PowerShellParses -RelativePath 'scripts/mistralrs-agent/roles.ps1'
Assert-PowerShellParses -RelativePath 'scripts/test-mistralrs-local-agent-harness.ps1'

$setup = Read-RepositoryFile -RelativePath 'scripts/setup-mistralrs-local-agent.ps1'
Assert-ContainsAll -Name 'Mistral.rs setup' -Content $setup -Required @(
    "[string]`$Version = 'master'",
    '[switch]$ForceSourceBuild',
    'Rust 1.94+',
    "@('git', 'cargo', 'rustc', 'nvidia-smi', 'nvcc')",
    'Visual Studio 2022 C++ Build Tools',
    "`$featureSet = 'cuda flash-attn cudnn'",
    "`$featureSet = 'cuda'",
    'cuda-install.json',
    'Get-FileHash',
    'Assert-CudaDoctorEvidence',
    'CUDA-enabled Mistral.rs is ready'
)
Assert-ContainsNone -Name 'Mistral.rs setup' -Content $setup -Forbidden @(
    'AllowCpuFallback',
    'Install-CpuFallback',
    'MISTRALRS_INSTALL_TAG',
    'degraded CPU',
    'CPU fallback'
)

$runner = Read-RepositoryFile -RelativePath 'scripts/run-mistralrs-vwm-task.ps1'
Assert-ContainsAll -Name 'Mistral.rs controller' -Content $runner -Required @(
    "[string]`$Model = 'Qwen/Qwen3-Coder-30B-A3B-Instruct'",
    "[string]`$FallbackModel = 'Qwen/Qwen3-8B'",
    '[int]$ContextLength = 32768',
    '[int]$MaxRepairRounds = 3',
    "`$Branch = 'agent/vwm-authoritative-revision-implementation'",
    "`$ModuleRoot = Join-Path `$PSScriptRoot 'mistralrs-agent'",
    ". (Join-Path `$ModuleRoot 'common.ps1')",
    ". (Join-Path `$ModuleRoot 'roles.ps1')",
    'Invoke-Implementer',
    'Invoke-Reviewer',
    'Invoke-Verifier',
    'MaxRepairRounds',
    'TASK_CANDIDATE',
    'CUDA_RUNTIME: VERIFIED'
)
Assert-ContainsNone -Name 'Mistral.rs controller' -Content $runner -Forbidden @(
    'AllowCpuFallback',
    'CPU mode',
    'CPU fallback'
)

$common = Read-RepositoryFile -RelativePath 'scripts/mistralrs-agent/common.ps1'
Assert-ContainsAll -Name 'Mistral.rs common module' -Content $common -Required @(
    'Test-IsLinkedWorktree',
    'worktree add',
    'Assert-CleanImplementationBranch',
    'Assert-CudaDoctorEvidence',
    'Assert-CudaProcess',
    'cuda-runtime-evidence.json',
    '--query-compute-apps=pid,process_name,used_gpu_memory',
    'CUDA startup failed; stopping Mistral.rs process',
    "'--host', '127.0.0.1'",
    "'--max-seq-len'",
    "'--max-tool-rounds'",
    "'--enable-shell'",
    "'--shell-path'",
    "'--shell-workdir'",
    "'--agent-permission', 'auto'",
    "'--sandbox', 'off'",
    "'--no-ui'",
    '/v1/models',
    '/v1/responses',
    "type = 'shell'",
    "type = 'container_auto'",
    "tool_choice = 'auto'",
    'session_id = $SessionId'
)
Assert-ContainsNone -Name 'Mistral.rs common module' -Content $common -Forbidden @(
    'AllowCpuFallback',
    'degraded CPU',
    'CPU mode',
    "'--cpu'"
)

$roles = Read-RepositoryFile -RelativePath 'scripts/mistralrs-agent/roles.ps1'
Assert-ContainsAll -Name 'Mistral.rs role module' -Content $roles -Required @(
    'function Invoke-Implementer',
    'function Invoke-Reviewer',
    'function Invoke-Verifier',
    'NewGuid',
    'function Invoke-Repair',
    'function Publish-VerifiedBranch',
    'VERDICT: PASS',
    'VERDICT: FAIL',
    'Repair round',
    "'--draft'",
    'Task 1, intended red tests are evidence',
    'cuda-runtime-evidence.json',
    'confirm mandatory CUDA runtime evidence'
)

$scriptCorpus = @($setup, $runner, $common, $roles) -join [Environment]::NewLine
$forbiddenGitInvocationPatterns = @(
    '(?im)^\s*Invoke-Git\b[^\r\n]*\bpush\s+--force(?:-with-lease)?\b',
    '(?im)^\s*Invoke-Git\b[^\r\n]*\bmerge\b',
    '(?im)^\s*Invoke-Git\b[^\r\n]*\breset\s+--hard\b',
    '(?im)^\s*Invoke-Git\b[^\r\n]*\bclean\b'
)
foreach ($pattern in $forbiddenGitInvocationPatterns) {
    if ($scriptCorpus -match $pattern) {
        throw "The local-agent scripts contain a prohibited git invocation matching: $pattern"
    }
}

$agents = Read-RepositoryFile -RelativePath 'AGENTS.md'
Assert-ContainsAll -Name 'AGENTS.md' -Content $agents -Required @(
    '# 3DMk Agent Contract',
    'docs/agent-execution/MISTRALRS_LOCAL_AGENT.md',
    'inactive historical tooling',
    'separate model sessions',
    'Reviewer and verifier sessions are read-only',
    'The local Mistral.rs executor must run with CUDA acceleration.',
    'agent/vwm-authoritative-revision-implementation',
    'TASK_CANDIDATE',
    'Evidence before assertion. No exceptions.'
)

$runbook = Read-RepositoryFile -RelativePath 'docs/agent-execution/MISTRALRS_LOCAL_AGENT.md'
Assert-ContainsAll -Name 'Mistral.rs runbook' -Content $runbook -Required @(
    'run-mistralrs-vwm-task.ps1 -Task 1',
    'Qwen/Qwen3-Coder-30B-A3B-Instruct',
    'Qwen/Qwen3-8B',
    'Qwen/Qwen3-4B',
    'CUDA is mandatory',
    'CPU execution is prohibited',
    'cuda-runtime-evidence.json',
    '127.0.0.1',
    'VERDICT: PASS',
    'VERDICT: FAIL',
    '.local-agent/task-XX/'
)
Assert-ContainsNone -Name 'Mistral.rs runbook' -Content $runbook -Forbidden @(
    'AllowCpuFallback',
    'CPU fallback',
    'degraded CPU mode'
)

$design = Read-RepositoryFile -RelativePath 'docs/superpowers/specs/2026-08-07-mistralrs-local-agent-experiment-design.md'
Assert-ContainsAll -Name 'Mistral.rs design' -Content $design -Required @(
    'CUDA execution is mandatory',
    'No CPU fallback is permitted',
    'live CUDA process evidence'
)
Assert-ContainsNone -Name 'Mistral.rs design' -Content $design -Forbidden @(
    'explicitly reported CPU fallback',
    'CPU is the only automatic'
)

$plan = Read-RepositoryFile -RelativePath 'docs/superpowers/plans/2026-08-07-mistralrs-local-agent-experiment.md'
Assert-ContainsAll -Name 'Mistral.rs implementation plan' -Content $plan -Required @(
    'CUDA is mandatory',
    'No CPU fallback is permitted',
    'live CUDA process evidence'
)
Assert-ContainsNone -Name 'Mistral.rs implementation plan' -Content $plan -Forbidden @(
    'AllowCpuFallback',
    'explicit CPU fallback'
)

$gitignore = Read-RepositoryFile -RelativePath '.gitignore'
Assert-ContainsAll -Name '.gitignore' -Content $gitignore -Required @(
    '/.local-agent/',
    '/.worktrees/'
)

$workflow = Read-RepositoryFile -RelativePath '.github/workflows/mistralrs-harness-validation.yml'
Assert-ContainsAll -Name 'Mistral.rs validation workflow' -Content $workflow -Required @(
    'runs-on: windows-latest',
    'scripts/test-mistralrs-local-agent-harness.ps1',
    'pwsh'
)

Write-Host 'MISTRALRS_LOCAL_AGENT_HARNESS_OK'