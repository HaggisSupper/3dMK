[CmdletBinding()]
param(
    [ValidateRange(1, 17)]
    [int]$Task = 1,

    [string]$Model = 'Qwen/Qwen3-Coder-30B-A3B-Instruct',

    [string]$FallbackModel = 'Qwen/Qwen3-8B',

    [ValidateSet(2, 3, 4, 5, 6, 8)]
    [int]$Quant = 4,

    [ValidateRange(8192, 131072)]
    [int]$ContextLength = 32768,

    [ValidateRange(0, 65535)]
    [int]$Port = 0,

    [ValidateRange(4, 96)]
    [int]$MaxToolRounds = 48,

    [ValidateRange(300, 7200)]
    [int]$ServerStartTimeoutSeconds = 3600,

    [ValidateRange(1, 5)]
    [int]$MaxRepairRounds = 3,

    [switch]$KeepServer,
    [switch]$SkipPush,
    [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$RepositoryFullName = 'HaggisSupper/3dMK'
$Branch = 'agent/vwm-authoritative-revision-implementation'
$BaseBranch = 'main'
$AuthoritativePlan = 'docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md'
$ProgressLedger = 'docs/agent-execution/VWM_PROGRESS.md'

$ModuleRoot = Join-Path $PSScriptRoot 'mistralrs-agent'
. (Join-Path $ModuleRoot 'common.ps1')
. (Join-Path $ModuleRoot 'roles.ps1')

$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not (Test-Path -LiteralPath (Join-Path $RepositoryRoot '.git'))) {
    throw "Repository root could not be resolved from $PSScriptRoot."
}

$mistralPath = Ensure-MistralRs -RepositoryRoot $RepositoryRoot
$worktree = Resolve-AgentWorktree -RepositoryRoot $RepositoryRoot
Assert-CleanImplementationBranch -Worktree $worktree

$runDirectory = Join-Path $worktree ('.local-agent\task-{0:d2}' -f $Task)
New-Item -ItemType Directory -Force -Path $runDirectory | Out-Null

if ($DryRun) {
    Write-Step "CUDA-only dry-run passed. Worktree: $worktree"
    Write-Step "Mistral.rs: $mistralPath"
    Write-Step "Primary model: $Model; fallback model: $FallbackModel; quantization: $Quant-bit; context: $ContextLength"
    exit 0
}

$listenPort = if ($Port -eq 0) { Get-FreeTcpPort } else { $Port }
$server = $null
try {
    try {
        $primaryServerArguments = @{
            MistralPath = $mistralPath
            ModelId = $Model
            Worktree = $worktree
            RunDirectory = $runDirectory
            ListenPort = $listenPort
            AttemptName = 'primary'
        }
        $server = Start-MistralServer @primaryServerArguments
    }
    catch {
        if (-not $FallbackModel -or $FallbackModel -eq $Model) { throw }
        Write-Warning "Primary model failed to start with CUDA: $($_.Exception.Message)"
        Write-Step "Retrying the smaller model '$FallbackModel' with the same mandatory CUDA gate."
        $listenPort = Get-FreeTcpPort
        $fallbackServerArguments = @{
            MistralPath = $mistralPath
            ModelId = $FallbackModel
            Worktree = $worktree
            RunDirectory = $runDirectory
            ListenPort = $listenPort
            AttemptName = 'fallback'
        }
        $server = Start-MistralServer @fallbackServerArguments
    }

    $baseCommit = (Invoke-Git $worktree rev-parse HEAD | Select-Object -First 1).Trim()
    $baseCommit | Set-Content -LiteralPath (Join-Path $runDirectory 'base-commit.txt') -Encoding ascii

    Invoke-Implementer -BaseUri $server.BaseUri -Worktree $worktree -RunDirectory $runDirectory
    $candidateHead = (Invoke-Git $worktree rev-parse HEAD | Select-Object -First 1).Trim()
    if ($candidateHead -eq $baseCommit) {
        throw 'The implementer session ended without creating a task commit.'
    }
    Assert-CleanImplementationBranch -Worktree $worktree

    $repairRound = 0
    while ($true) {
        $reviewArguments = @{
            BaseUri = $server.BaseUri
            Worktree = $worktree
            RunDirectory = $runDirectory
            BaseCommit = $baseCommit
            Round = $repairRound
        }
        $review = Invoke-Reviewer @reviewArguments

        if ($review.Verdict -ne 'PASS') {
            if ($repairRound -ge $MaxRepairRounds) {
                throw "BLOCKED: reviewer did not pass after $MaxRepairRounds repair rounds. Last verdict: $($review.Verdict)."
            }
            $repairRound += 1
            $findings = if (Test-Path -LiteralPath $review.Path) {
                Get-Content -LiteralPath $review.Path -Raw
            }
            else {
                'Reviewer report was missing or malformed.'
            }
            $reviewRepairArguments = @{
                BaseUri = $server.BaseUri
                Worktree = $worktree
                RunDirectory = $runDirectory
                Findings = $findings
                Round = $repairRound
            }
            Invoke-Repair @reviewRepairArguments
            continue
        }

        $verificationArguments = @{
            BaseUri = $server.BaseUri
            Worktree = $worktree
            RunDirectory = $runDirectory
            BaseCommit = $baseCommit
            Round = $repairRound
        }
        $verification = Invoke-Verifier @verificationArguments

        if ($verification.Verdict -ne 'PASS') {
            if ($repairRound -ge $MaxRepairRounds) {
                throw "BLOCKED: verifier did not pass after $MaxRepairRounds repair rounds. Last verdict: $($verification.Verdict)."
            }
            $repairRound += 1
            $findings = if (Test-Path -LiteralPath $verification.Path) {
                Get-Content -LiteralPath $verification.Path -Raw
            }
            else {
                'Verifier report was missing or malformed.'
            }
            $verificationRepairArguments = @{
                BaseUri = $server.BaseUri
                Worktree = $worktree
                RunDirectory = $runDirectory
                Findings = $findings
                Round = $repairRound
            }
            Invoke-Repair @verificationRepairArguments
            continue
        }

        break
    }

    $finalHead = (Invoke-Git $worktree rev-parse HEAD | Select-Object -First 1).Trim()
    @"
STATE: TASK_CANDIDATE
TASK: $Task
BASE: $baseCommit
HEAD: $finalHead
MODEL: $($server.Model)
CUDA_RUNTIME: VERIFIED
REVIEW: PASS
VERIFICATION: PASS
"@ | Set-Content -LiteralPath (Join-Path $runDirectory 'controller-summary.txt') -Encoding utf8

    Publish-VerifiedBranch -Worktree $worktree -RunDirectory $runDirectory -SelectedModel $server.Model
    Write-Step "Task $Task has fresh CUDA runtime, reviewer, and verifier evidence at commit $finalHead."
}
finally {
    if ($server -and -not $KeepServer -and -not $server.Process.HasExited) {
        Write-Step 'Stopping the local Mistral.rs server.'
        Stop-Process -Id $server.Process.Id -Force -ErrorAction SilentlyContinue
    }
    elseif ($server -and $KeepServer) {
        Write-Step "CUDA-backed Mistral.rs remains available at $($server.BaseUri)."
    }
}