[CmdletBinding()]
param(
    [ValidateRange(1, 17)]
    [int]$Task = 1,
    [string]$Model = 'Qwen/Qwen3-Coder-30B-A3B-Instruct',
    [string]$FallbackModel = 'Qwen/Qwen3-8B',
    [ValidateRange(2, 8)]
    [int]$Quant = 4,
    [ValidateRange(0, 65535)]
    [int]$Port = 0,
    [ValidateRange(4, 96)]
    [int]$MaxToolRounds = 48,
    [ValidateRange(300, 7200)]
    [int]$ServerStartTimeoutSeconds = 3600,
    [ValidateRange(1, 5)]
    [int]$MaxRepairRounds = 3,
    [switch]$RequireCuda,
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

function Write-Step {
    param([string]$Message)
    Write-Host "[3DMk local agent] $Message"
}

function Get-CommandPath {
    param([Parameter(Mandatory)][string]$Name)
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    return $null
}

function Invoke-Git {
    param(
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(ValueFromRemainingArguments)][string[]]$Arguments
    )
    $git = Get-CommandPath 'git'
    if (-not $git) { throw 'git is required.' }
    $output = & $git -C $WorkingDirectory @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed:`n$($output | Out-String)"
    }
    return @($output)
}

function Test-GitRef {
    param(
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][string]$Ref
    )
    $git = Get-CommandPath 'git'
    & $git -C $WorkingDirectory show-ref --verify --quiet $Ref
    return $LASTEXITCODE -eq 0
}

function Get-FreeTcpPort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try { return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port }
    finally { $listener.Stop() }
}

function ConvertTo-ProcessArgument {
    param([string]$Value)
    if ($Value -notmatch '[\s"]') { return $Value }
    return '"' + ($Value -replace '(\\*)"', '$1$1\"') + '"'
}

function Get-ExistingBranchWorktree {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$BranchName
    )
    $lines = Invoke-Git $RepositoryRoot worktree list --porcelain
    $path = $null
    foreach ($line in @($lines) + '') {
        if ($line -like 'worktree *') {
            $path = $line.Substring(9)
            continue
        }
        if ($line -eq "branch refs/heads/$BranchName" -and $path) {
            return $path
        }
        if ([string]::IsNullOrWhiteSpace($line)) {
            $path = $null
        }
    }
    return $null
}

function Resolve-AgentWorktree {
    param([Parameter(Mandatory)][string]$RepositoryRoot)

    Write-Step 'Fetching the implementation branch from GitHub.'
    Invoke-Git $RepositoryRoot fetch --prune origin | Out-Null

    $existing = Get-ExistingBranchWorktree -RepositoryRoot $RepositoryRoot -BranchName $Branch
    if ($existing) {
        Write-Step "Using existing implementation worktree: $existing"
        return (Resolve-Path -LiteralPath $existing).Path
    }

    $worktreeParent = Join-Path $env:LOCALAPPDATA '3DMk\worktrees'
    $worktreePath = Join-Path $worktreeParent 'vwm-authoritative-revision-implementation'
    New-Item -ItemType Directory -Force -Path $worktreeParent | Out-Null
    if (Test-Path -LiteralPath $worktreePath) {
        throw "The intended worktree path exists but is not registered with git: $worktreePath"
    }

    if (Test-GitRef $RepositoryRoot "refs/heads/$Branch") {
        Invoke-Git $RepositoryRoot worktree add $worktreePath $Branch | Out-Null
    }
    elseif (Test-GitRef $RepositoryRoot "refs/remotes/origin/$Branch") {
        Invoke-Git $RepositoryRoot worktree add --track -b $Branch $worktreePath "origin/$Branch" | Out-Null
    }
    else {
        Invoke-Git $RepositoryRoot worktree add -b $Branch $worktreePath "origin/$BaseBranch" | Out-Null
    }

    Write-Step "Created isolated implementation worktree: $worktreePath"
    return (Resolve-Path -LiteralPath $worktreePath).Path
}

function Assert-CleanImplementationBranch {
    param([Parameter(Mandatory)][string]$Worktree)
    $currentBranch = (Invoke-Git $Worktree branch --show-current | Select-Object -First 1).Trim()
    if ($currentBranch -ne $Branch) {
        throw "Refusing to run on branch '$currentBranch'. Required branch: '$Branch'."
    }
    if ($currentBranch -in @('main', 'master')) {
        throw 'Refusing to run a local model on a default branch.'
    }
    $status = Invoke-Git $Worktree status --porcelain
    if ($status.Count -gt 0 -and ($status -join '').Trim()) {
        throw "The implementation worktree has uncommitted changes. Resolve them before local-agent execution:`n$($status -join [Environment]::NewLine)"
    }
}

function Ensure-MistralRs {
    $mistral = Get-CommandPath 'mistralrs'
    if (-not $mistral) {
        Write-Step 'Mistral.rs is not on PATH; invoking repository setup.'
        $setup = Join-Path $RepositoryRoot 'scripts\setup-mistralrs-local-agent.ps1'
        $setupArgs = @('-NoLogo', '-NoProfile', '-File', $setup)
        if (-not $RequireCuda) { $setupArgs += '-AllowCpuFallback' }
        & (Get-CommandPath 'pwsh') @setupArgs
        if ($LASTEXITCODE -ne 0) { throw 'Mistral.rs setup failed.' }
        $env:PATH = "$(Join-Path $env:USERPROFILE '.cargo\bin');$(Join-Path $env:USERPROFILE '.local\bin');$env:PATH"
        $mistral = Get-CommandPath 'mistralrs'
    }
    if (-not $mistral) { throw 'mistralrs is unavailable after setup.' }

    $doctor = (& $mistral doctor 2>&1 | Out-String)
    Write-Host $doctor.TrimEnd()
    $hasCuda = $doctor -match '(?im)build features:.*\bcuda\b'
    if ($RequireCuda -and -not $hasCuda) {
        throw 'The installed Mistral.rs binary is CPU-only and -RequireCuda was specified.'
    }
    if (-not $hasCuda) {
        Write-Warning 'Mistral.rs is running in degraded CPU mode. The experiment remains local but will be slower.'
    }
    return $mistral
}

function Wait-MistralServer {
    param(
        [Parameter(Mandatory)][System.Diagnostics.Process]$Process,
        [Parameter(Mandatory)][string]$BaseUri,
        [Parameter(Mandatory)][int]$TimeoutSeconds,
        [Parameter(Mandatory)][string]$StdoutPath,
        [Parameter(Mandatory)][string]$StderrPath
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if ($Process.HasExited) {
            $stdout = if (Test-Path $StdoutPath) { Get-Content $StdoutPath -Raw } else { '' }
            $stderr = if (Test-Path $StderrPath) { Get-Content $StderrPath -Raw } else { '' }
            throw "Mistral.rs exited before becoming ready.`nSTDOUT:`n$stdout`nSTDERR:`n$stderr"
        }
        try {
            Invoke-RestMethod -Method Get -Uri "$BaseUri/health" -TimeoutSec 3 | Out-Null
            return
        }
        catch {
            Start-Sleep -Seconds 2
        }
    }
    try { Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue } catch {}
    throw "Mistral.rs did not become ready within the configured startup timeout. Inspect $StdoutPath and $StderrPath."
}

function Start-MistralServer {
    param(
        [Parameter(Mandatory)][string]$MistralPath,
        [Parameter(Mandatory)][string]$ModelId,
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$RunDirectory,
        [Parameter(Mandatory)][int]$ListenPort
    )
    $stdoutPath = Join-Path $RunDirectory 'mistralrs.stdout.log'
    $stderrPath = Join-Path $RunDirectory 'mistralrs.stderr.log'
    $pwshPath = Get-CommandPath 'pwsh'
    if (-not $pwshPath) { throw 'PowerShell 7 (pwsh) is required for the local shell executor.' }

    $arguments = @(
        'serve',
        '--host', '127.0.0.1',
        '--port', [string]$ListenPort,
        '--quant', [string]$Quant,
        '-m', $ModelId,
        '--enable-shell',
        '--shell-path', $pwshPath,
        '--shell-workdir', $Worktree,
        '--shell-timeout', '1800',
        '--agent-permission', 'auto',
        '--sandbox', 'off',
        '--no-ui'
    )
    $quoted = $arguments | ForEach-Object { ConvertTo-ProcessArgument $_ }
    Write-Step "Starting Mistral.rs with local model '$ModelId'."
    $process = Start-Process -FilePath $MistralPath -ArgumentList $quoted -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    $baseUri = "http://127.0.0.1:$ListenPort"
    Wait-MistralServer -Process $process -BaseUri $baseUri -TimeoutSeconds $ServerStartTimeoutSeconds -StdoutPath $stdoutPath -StderrPath $stderrPath
    return [pscustomobject]@{
        Process = $process
        BaseUri = $baseUri
        Model = $ModelId
        Stdout = $stdoutPath
        Stderr = $stderrPath
    }
}

function Invoke-AgentResponse {
    param(
        [Parameter(Mandatory)][string]$BaseUri,
        [Parameter(Mandatory)][string]$Prompt,
        [Parameter(Mandatory)][string]$SessionId,
        [Parameter(Mandatory)][string]$OutputPath
    )
    $payload = [ordered]@{
        model = 'default'
        input = $Prompt
        tools = @(
            [ordered]@{
                type = 'shell'
                environment = [ordered]@{ type = 'container_auto' }
            }
        )
        tool_choice = 'required'
        max_tool_rounds = $MaxToolRounds
        max_output_tokens = 16384
        session_id = $SessionId
    }
    $body = $payload | ConvertTo-Json -Depth 20
    $response = Invoke-RestMethod -Method Post -Uri "$BaseUri/v1/responses" -ContentType 'application/json' -Body $body -TimeoutSec 7200
    $response | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $OutputPath -Encoding utf8
    return $response
}

function Get-ReportVerdict {
    param([Parameter(Mandatory)][string]$ReportPath)
    if (-not (Test-Path -LiteralPath $ReportPath)) { return 'MISSING' }
    $firstLine = Get-Content -LiteralPath $ReportPath -TotalCount 1
    if ($firstLine -eq 'VERDICT: PASS') { return 'PASS' }
    if ($firstLine -eq 'VERDICT: FAIL') { return 'FAIL' }
    return 'MALFORMED'
}

function Assert-NoTrackedSessionChanges {
    param(
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$Role
    )
    $status = Invoke-Git $Worktree status --porcelain --untracked-files=no
    if ($status.Count -gt 0 -and ($status -join '').Trim()) {
        throw "$Role modified tracked files despite its read-only contract:`n$($status -join [Environment]::NewLine)"
    }
}

function Invoke-Implementer {
    param(
        [Parameter(Mandatory)][string]$BaseUri,
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$RunDirectory,
        [string]$RepairContext = '',
        [int]$Round = 0
    )
    $relativeRunDirectory = [IO.Path]::GetRelativePath($Worktree, $RunDirectory).Replace('\', '/')
    $outputPath = Join-Path $RunDirectory (if ($Round -eq 0) { 'implementer-response.json' } else { "repair-$Round-response.json" })
    $reportPath = "$relativeRunDirectory/" + (if ($Round -eq 0) { 'implementer.md' } else { "repair-$Round.md" })
    $repairSection = if ($RepairContext) {
        @"

A prior independent gate failed. Read the complete findings below and repair every load-bearing issue before re-running affected checks and committing a new coherent fix commit.

--- findings ---
$RepairContext
--- end findings ---
"@
    } else { '' }

    $prompt = @"
You are the local Mistral.rs implementation worker for 3DMk Task $Task.

Use the shell tool continuously; do not merely describe commands. The shell starts in the dedicated implementation worktree.

Read, in this exact order:
1. AGENTS.md
2. $ProgressLedger
3. $AuthoritativePlan
4. The complete section headed `### Task $Task:` and only its directly referenced files.

Mandatory execution rules:
- Confirm the current branch is exactly `$Branch` and never switch to main or master.
- Execute only Task $Task. Do not begin the next task.
- Follow test-driven development and the task's exact acceptance conditions.
- For Task 1, intended red tests are evidence: do not change production behavior merely to make them green.
- Use real repository APIs and fixtures. Do not manufacture a failure.
- Run the focused commands and affected regression checks required by the task.
- Update $ProgressLedger with exact commands, outputs, files, commit, and residual risks.
- Do not use Docker, Podman, WSL, OpenCode, cloud inference, force push, merge, `git reset --hard`, or `git clean`.
- Preserve unrelated files.
- Commit all tracked task changes locally with the task's prescribed message or a more accurate equivalent.
- Do not push. The controller pushes only after independent gates pass.
- Write a concise execution report to `$reportPath` before finishing. Include the commit SHA and exact observed test outcomes.
- End your final response with one state: TASK_CANDIDATE, SESSION_BOUNDARY, or BLOCKED.
$repairSection
Begin now and continue until this bounded task has a committed candidate or a genuine evidence-backed blocker.
"@
    Invoke-AgentResponse -BaseUri $BaseUri -Prompt $prompt -SessionId "3dmk-task-$Task-implementer-$Round" -OutputPath $outputPath | Out-Null
}

function Invoke-Reviewer {
    param(
        [Parameter(Mandatory)][string]$BaseUri,
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$RunDirectory,
        [Parameter(Mandatory)][string]$BaseCommit,
        [Parameter(Mandatory)][int]$Round
    )
    $relativeRunDirectory = [IO.Path]::GetRelativePath($Worktree, $RunDirectory).Replace('\', '/')
    $reportRelative = "$relativeRunDirectory/review-$Round.md"
    $reportPath = Join-Path $RunDirectory "review-$Round.md"
    $responsePath = Join-Path $RunDirectory "review-$Round-response.json"
    Remove-Item -LiteralPath $reportPath -Force -ErrorAction SilentlyContinue

    $prompt = @"
You are the independent 3DMk reviewer for authoritative plan Task $Task. This is a read-only tracked-file role.

Use the shell to read:
1. AGENTS.md
2. $ProgressLedger
3. the complete `### Task $Task:` section in $AuthoritativePlan
4. every commit and the full diff in `$BaseCommit...HEAD`
5. the implementer report under `$relativeRunDirectory`

Review specification compliance first, then code/test quality. Verify that claimed failures or passes match the actual task semantics. Task 1 intentionally requires red tests that reproduce real authority defects; do not demand production fixes in Task 1.

Do not modify, stage, commit, push, reset, restore, or delete tracked files. You may run read-only git commands and tests. Do not use OpenCode or a cloud model.

Write the complete review to `$reportRelative`.
The first line must be exactly `VERDICT: PASS` or `VERDICT: FAIL`.
A failing report must list each blocking or important finding with file/path evidence and a concrete correction. A passing report must state why every task acceptance condition is met.
"@
    Invoke-AgentResponse -BaseUri $BaseUri -Prompt $prompt -SessionId "3dmk-task-$Task-review-$Round" -OutputPath $responsePath | Out-Null
    Assert-NoTrackedSessionChanges -Worktree $Worktree -Role 'Reviewer'
    return [pscustomobject]@{ Path = $reportPath; Verdict = Get-ReportVerdict $reportPath }
}

function Invoke-Verifier {
    param(
        [Parameter(Mandatory)][string]$BaseUri,
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$RunDirectory,
        [Parameter(Mandatory)][string]$BaseCommit,
        [Parameter(Mandatory)][int]$Round
    )
    $relativeRunDirectory = [IO.Path]::GetRelativePath($Worktree, $RunDirectory).Replace('\', '/')
    $reportRelative = "$relativeRunDirectory/verification-$Round.md"
    $reportPath = Join-Path $RunDirectory "verification-$Round.md"
    $responsePath = Join-Path $RunDirectory "verification-$Round-response.json"
    Remove-Item -LiteralPath $reportPath -Force -ErrorAction SilentlyContinue

    $prompt = @"
You are the independent 3DMk verifier for authoritative plan Task $Task. This is a read-only tracked-file role.

Use the shell to read AGENTS.md, $ProgressLedger, the full Task $Task plan section, and `$BaseCommit...HEAD`. Run fresh verification commands rather than trusting the implementer or reviewer.

Verification requirements:
- confirm branch `$Branch` and a clean tracked worktree;
- run every focused command specified by Task $Task;
- run directly affected regression checks;
- confirm the progress ledger records observed commands and outcomes;
- confirm the commit contains only bounded task changes;
- interpret Task 1's expected red tests correctly: PASS means they compile, execute, and fail for the intended authority gaps rather than syntax, fixture, or harness errors;
- reject fabricated or unexecuted evidence.

Do not modify, stage, commit, push, reset, restore, or delete tracked files. Do not use OpenCode or a cloud model.

Write the complete verification report to `$reportRelative`.
The first line must be exactly `VERDICT: PASS` or `VERDICT: FAIL`.
Include each command, exit code, and the decisive output. A failure report must identify the exact repair required.
"@
    Invoke-AgentResponse -BaseUri $BaseUri -Prompt $prompt -SessionId "3dmk-task-$Task-verifier-$Round" -OutputPath $responsePath | Out-Null
    Assert-NoTrackedSessionChanges -Worktree $Worktree -Role 'Verifier'
    return [pscustomobject]@{ Path = $reportPath; Verdict = Get-ReportVerdict $reportPath }
}

function Publish-VerifiedBranch {
    param(
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$RunDirectory
    )
    if ($SkipPush) {
        Write-Warning 'Review and verification passed locally, but -SkipPush prevented GitHub synchronization.'
        return
    }

    Write-Step 'Pushing the reviewed and verified implementation branch.'
    Invoke-Git $Worktree push -u origin $Branch | Out-Null

    $gh = Get-CommandPath 'gh'
    if (-not $gh) {
        throw 'The branch was pushed, but GitHub CLI is unavailable; a draft PR could not be created or updated.'
    }
    & $gh auth status 2>&1 | Set-Content -LiteralPath (Join-Path $RunDirectory 'gh-auth-status.log') -Encoding utf8
    if ($LASTEXITCODE -ne 0) {
        throw 'The branch was pushed, but GitHub CLI is not authenticated; run gh auth login to create the draft PR.'
    }

    & $gh pr view $Branch --repo $RepositoryFullName --json number,url,state 2>$null | Set-Content -LiteralPath (Join-Path $RunDirectory 'pr.json') -Encoding utf8
    if ($LASTEXITCODE -eq 0) {
        Write-Step 'Existing draft or open PR found; branch synchronization is complete.'
        return
    }

    $bodyPath = Join-Path $RunDirectory 'pr-body.md'
    @"
## Scope

Executes the authoritative 3DMk VWM revision workflow through a local Mistral.rs model. This PR remains task-gated; it does not claim the complete 17-task project is finished.

## Current task

Task $Task from `$AuthoritativePlan`.

## Execution backend

- Mistral.rs on `127.0.0.1`
- local model: $Model
- independent implementer, reviewer, and verifier sessions
- OpenCode is not required

## Evidence

See `$ProgressLedger` for exact commands and outcomes.
"@ | Set-Content -LiteralPath $bodyPath -Encoding utf8

    & $gh pr create --repo $RepositoryFullName --draft --base $BaseBranch --head $Branch --title "3DMk authoritative revision workflow: Task $Task" --body-file $bodyPath
    if ($LASTEXITCODE -ne 0) { throw 'GitHub CLI failed to create the draft pull request.' }
}

$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not (Test-Path -LiteralPath (Join-Path $RepositoryRoot '.git'))) {
    throw "Repository root could not be resolved from $PSScriptRoot."
}
if (-not (Get-CommandPath 'pwsh')) { throw 'PowerShell 7 (pwsh) is required.' }

$mistralPath = Ensure-MistralRs
$worktree = Resolve-AgentWorktree -RepositoryRoot $RepositoryRoot
Assert-CleanImplementationBranch -Worktree $worktree

$runDirectory = Join-Path $worktree ('.local-agent\task-{0:d2}' -f $Task)
New-Item -ItemType Directory -Force -Path $runDirectory | Out-Null

if ($DryRun) {
    Write-Step "Dry-run passed. Worktree: $worktree"
    Write-Step "Mistral.rs: $mistralPath"
    Write-Step "Primary model: $Model; fallback model: $FallbackModel; quantization: $Quant-bit"
    exit 0
}

$listenPort = if ($Port -eq 0) { Get-FreeTcpPort } else { $Port }
$server = $null
try {
    try {
        $server = Start-MistralServer -MistralPath $mistralPath -ModelId $Model -Worktree $worktree -RunDirectory $runDirectory -ListenPort $listenPort
    }
    catch {
        if (-not $FallbackModel -or $FallbackModel -eq $Model) { throw }
        Write-Warning "Primary model failed to start: $($_.Exception.Message)"
        Write-Step "Retrying with fallback model '$FallbackModel'."
        $listenPort = Get-FreeTcpPort
        $server = Start-MistralServer -MistralPath $mistralPath -ModelId $FallbackModel -Worktree $worktree -RunDirectory $runDirectory -ListenPort $listenPort
    }

    $baseCommit = (Invoke-Git $worktree rev-parse HEAD | Select-Object -First 1).Trim()
    $baseCommit | Set-Content -LiteralPath (Join-Path $runDirectory 'base-commit.txt') -Encoding ascii

    Invoke-Implementer -BaseUri $server.BaseUri -Worktree $worktree -RunDirectory $runDirectory
    $candidateHead = (Invoke-Git $worktree rev-parse HEAD | Select-Object -First 1).Trim()
    if ($candidateHead -eq $baseCommit) {
        throw 'The implementer session ended without creating a task commit.'
    }
    Assert-CleanImplementationBranch -Worktree $worktree

    $gateRound = 0
    while ($true) {
        $review = Invoke-Reviewer -BaseUri $server.BaseUri -Worktree $worktree -RunDirectory $runDirectory -BaseCommit $baseCommit -Round $gateRound
        if ($review.Verdict -ne 'PASS') {
            if ($gateRound -ge $MaxRepairRounds) {
                throw "BLOCKED: reviewer did not pass after $MaxRepairRounds repair rounds. Last verdict: $($review.Verdict)."
            }
            $gateRound += 1
            $findings = if (Test-Path $review.Path) { Get-Content $review.Path -Raw } else { 'Reviewer report was missing or malformed.' }
            Invoke-Implementer -BaseUri $server.BaseUri -Worktree $worktree -RunDirectory $runDirectory -RepairContext $findings -Round $gateRound
            Assert-CleanImplementationBranch -Worktree $worktree
            continue
        }

        $verification = Invoke-Verifier -BaseUri $server.BaseUri -Worktree $worktree -RunDirectory $runDirectory -BaseCommit $baseCommit -Round $gateRound
        if ($verification.Verdict -ne 'PASS') {
            if ($gateRound -ge $MaxRepairRounds) {
                throw "BLOCKED: verifier did not pass after $MaxRepairRounds repair rounds. Last verdict: $($verification.Verdict)."
            }
            $gateRound += 1
            $findings = if (Test-Path $verification.Path) { Get-Content $verification.Path -Raw } else { 'Verifier report was missing or malformed.' }
            Invoke-Implementer -BaseUri $server.BaseUri -Worktree $worktree -RunDirectory $runDirectory -RepairContext $findings -Round $gateRound
            Assert-CleanImplementationBranch -Worktree $worktree
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
REVIEW: PASS
VERIFICATION: PASS
"@ | Set-Content -LiteralPath (Join-Path $runDirectory 'controller-summary.txt') -Encoding utf8

    Publish-VerifiedBranch -Worktree $worktree -RunDirectory $runDirectory
    Write-Step "Task $Task has local reviewer and verifier PASS evidence at commit $finalHead."
}
finally {
    if ($server -and -not $KeepServer -and -not $server.Process.HasExited) {
        Write-Step 'Stopping the local Mistral.rs server.'
        Stop-Process -Id $server.Process.Id -Force -ErrorAction SilentlyContinue
    }
    elseif ($server -and $KeepServer) {
        Write-Step "Mistral.rs remains available at $($server.BaseUri)."
    }
}
