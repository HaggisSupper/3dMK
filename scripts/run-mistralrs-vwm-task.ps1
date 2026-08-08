[CmdletBinding(DefaultParameterSetName = 'Authoritative')]
param(
    [Parameter(ParameterSetName = 'Authoritative')]
    [ValidateRange(1, 17)]
    [int]$Task = 1,

    [Parameter(Mandatory, ParameterSetName = 'Foundation')]
    [ValidateSet('FB1', 'FB2', 'FB3', 'FB4', 'FB5', 'FB6', 'FB7', 'FB8')]
    [string]$FoundationTask,

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
$AuthoritativeLedger = 'docs/agent-execution/VWM_PROGRESS.md'
$FoundationPlan = 'docs/superpowers/plans/2026-08-07-3dmk-foundation-batch.md'
$FoundationLedger = 'docs/agent-execution/CUDA_FOUNDATION_PROGRESS.md'

$TaskKind = if ($PSCmdlet.ParameterSetName -eq 'Foundation') { 'foundation' } else { 'authoritative' }
$TaskId = if ($TaskKind -eq 'foundation') { $FoundationTask } else { [string]$Task }
$TaskPlan = if ($TaskKind -eq 'foundation') { $FoundationPlan } else { $AuthoritativePlan }
$TaskLedger = if ($TaskKind -eq 'foundation') { $FoundationLedger } else { $AuthoritativeLedger }
$TaskLabel = if ($TaskKind -eq 'foundation') { "Foundation Task $TaskId" } else { "Authoritative Task $TaskId" }
$RunDirectoryName = if ($TaskKind -eq 'foundation') {
    "foundation-$($TaskId.ToLowerInvariant())"
}
else {
    'task-{0:d2}' -f $Task
}

$ModuleRoot = Join-Path $PSScriptRoot 'mistralrs-agent'
. (Join-Path $ModuleRoot 'common.ps1')
. (Join-Path $ModuleRoot 'roles.ps1')

$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not (Test-Path -LiteralPath (Join-Path $RepositoryRoot '.git'))) {
    throw "Repository root could not be resolved from $PSScriptRoot."
}

function Assert-LedgerEntryComplete {
    param(
        [Parameter(Mandatory)][string]$LedgerPath,
        [Parameter(Mandatory)][string]$EntryLabel
    )

    if (-not (Test-Path -LiteralPath $LedgerPath -PathType Leaf)) {
        throw "BLOCKED: required dependency ledger is missing: $LedgerPath"
    }

    $content = Get-Content -LiteralPath $LedgerPath -Raw
    $escapedLabel = [regex]::Escape($EntryLabel)
    if (-not [regex]::IsMatch($content, "(?m)^- \[x\] \*\*${escapedLabel}:")) {
        throw "BLOCKED: $TaskLabel requires completed ledger entry '$EntryLabel' in $LedgerPath."
    }
}

function Assert-TaskDependencies {
    param([Parameter(Mandatory)][string]$Worktree)

    $authoritativeLedgerPath = Join-Path $Worktree $AuthoritativeLedger
    $foundationLedgerPath = Join-Path $Worktree $FoundationLedger

    if ($TaskKind -eq 'foundation') {
        foreach ($requiredTask in 1..5) {
            Assert-LedgerEntryComplete -LedgerPath $authoritativeLedgerPath -EntryLabel "Task $requiredTask"
        }

        $foundationNumber = [int]$FoundationTask.Substring(2)
        if ($foundationNumber -gt 1) {
            foreach ($requiredFoundation in 1..($foundationNumber - 1)) {
                Assert-LedgerEntryComplete -LedgerPath $foundationLedgerPath -EntryLabel "FB$requiredFoundation"
            }
        }
        return
    }

    if ($Task -gt 1) {
        foreach ($requiredTask in 1..($Task - 1)) {
            Assert-LedgerEntryComplete -LedgerPath $authoritativeLedgerPath -EntryLabel "Task $requiredTask"
        }
    }

    if ($Task -ge 6) {
        foreach ($requiredFoundation in 1..8) {
            Assert-LedgerEntryComplete -LedgerPath $foundationLedgerPath -EntryLabel "FB$requiredFoundation"
        }
    }
}

$worktree = Resolve-AgentWorktree -RepositoryRoot $RepositoryRoot
Assert-CleanImplementationBranch -Worktree $worktree
Assert-TaskDependencies -Worktree $worktree
$mistralPath = Ensure-MistralRs -RepositoryRoot $RepositoryRoot

$runDirectory = Join-Path $worktree ('.local-agent\{0}' -f $RunDirectoryName)
New-Item -ItemType Directory -Force -Path $runDirectory | Out-Null

if ($DryRun) {
    Write-Step "CUDA-only dry-run passed for $TaskLabel. Worktree: $worktree"
    Write-Step "Mistral.rs: $mistralPath"
    Write-Step "Plan: $TaskPlan; ledger: $TaskLedger"
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
TASK_KIND: $TaskKind
TASK_ID: $TaskId
TASK_LABEL: $TaskLabel
PLAN: $TaskPlan
LEDGER: $TaskLedger
BASE: $baseCommit
HEAD: $finalHead
MODEL: $($server.Model)
CUDA_RUNTIME: VERIFIED
REVIEW: PASS
VERIFICATION: PASS
"@ | Set-Content -LiteralPath (Join-Path $runDirectory 'controller-summary.txt') -Encoding utf8

    Publish-VerifiedBranch -Worktree $worktree -RunDirectory $runDirectory -SelectedModel $server.Model
    Write-Step "$TaskLabel has fresh CUDA runtime, reviewer, and verifier evidence at commit $finalHead."
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
