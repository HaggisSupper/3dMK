# Role orchestration for the 3DMk local Mistral.rs task controller.
# Dot-source this file after common.ps1.

function Invoke-Implementer {
    param(
        [Parameter(Mandatory)][string]$BaseUri,
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$RunDirectory,
        [string]$RepairContext = '',
        [int]$Round = 0
    )

    $relativeRunDirectory = [IO.Path]::GetRelativePath($Worktree, $RunDirectory).Replace('\', '/')
    $responseName = if ($Round -eq 0) { 'implementer-response.json' } else { "repair-$Round-response.json" }
    $reportName = if ($Round -eq 0) { 'implementer.md' } else { "repair-$Round.md" }
    $outputPath = Join-Path $RunDirectory $responseName
    $reportRelativePath = "$relativeRunDirectory/$reportName"
    $worktreeLiteral = ConvertTo-PowerShellSingleQuotedLiteral -Value $Worktree
    $taskSpecificRule = if ($TaskKind -eq 'authoritative' -and $TaskId -eq '1') {
        '- Authoritative Task 1 requires intended red tests as evidence; do not change production behavior merely to make those tests green.'
    }
    else {
        '- Implement the selected task production behavior only after observing the required failing test.'
    }

    $repairSection = if ($RepairContext) {
        @"

A prior independent gate failed. Repair every load-bearing finding below before rerunning affected checks and committing one coherent fix commit.

--- findings ---
$RepairContext
--- end findings ---
"@
    }
    else {
        ''
    }

    $prompt = @"
You are the local Mistral.rs implementation worker for $TaskLabel.

Use the shell tool continuously; do not merely describe commands. Your first shell command must be:
Set-Location -LiteralPath '$worktreeLiteral'

Read, in this exact order:
1. AGENTS.md
2. docs/CURRENT_STATE.md
3. $AuthoritativeLedger
4. $FoundationLedger
5. $TaskPlan
6. The complete section headed Task $TaskId and only its directly referenced source and tests.

Mandatory rules:
- Confirm the current branch is exactly $Branch and never switch to main or master.
- Execute only $TaskLabel. Do not begin the next task.
- Follow test-driven development and the task's exact acceptance conditions.
$taskSpecificRule
- Use real repository APIs and fixtures. Do not manufacture failures.
- Run the focused commands and affected regression checks required by the task.
- Update $TaskLedger with exact commands, exit codes, decisive outputs, files, commit, accelerator evidence when applicable, and residual risks.
- Update docs/CURRENT_STATE.md in the same task when implementation truth changes.
- Do not use Docker, Podman, WSL, OpenCode, cloud inference, force push, merge, git reset --hard, or git clean.
- Preserve unrelated files.
- Commit all tracked task changes locally with the prescribed message or a more accurate equivalent.
- Do not push. The controller pushes only after independent gates pass.
- Write a concise execution report to $reportRelativePath before finishing. Include the commit SHA and exact observed test outcomes.
- End your final response with one state: TASK_CANDIDATE, SESSION_BOUNDARY, or BLOCKED.
$repairSection
Begin now and continue until this bounded task has a committed candidate or a genuine evidence-backed blocker.
"@

    $sessionId = "3dmk-$TaskKind-$TaskId-implementer-$Round-$([Guid]::NewGuid().ToString('N'))"
    Invoke-AgentResponse -BaseUri $BaseUri -Prompt $prompt -SessionId $sessionId -OutputPath $outputPath | Out-Null
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
    $reportRelativePath = "$relativeRunDirectory/review-$Round.md"
    $reportPath = Join-Path $RunDirectory "review-$Round.md"
    $responsePath = Join-Path $RunDirectory "review-$Round-response.json"
    $worktreeLiteral = ConvertTo-PowerShellSingleQuotedLiteral -Value $Worktree
    $expectedHead = (Invoke-Git $Worktree rev-parse HEAD | Select-Object -First 1).Trim()
    $taskOneReviewNote = if ($TaskKind -eq 'authoritative' -and $TaskId -eq '1') {
        'Authoritative Task 1 intentionally requires red tests that reproduce real authority defects; do not demand production fixes in that task.'
    }
    else {
        'Require the selected task to implement its declared production behavior and make the intended failing test pass.'
    }
    Remove-Item -LiteralPath $reportPath -Force -ErrorAction SilentlyContinue

    $prompt = @"
You are the independent 3DMk reviewer for $TaskLabel. This is a read-only tracked-file role.

Use the shell. Your first shell command must be:
Set-Location -LiteralPath '$worktreeLiteral'

Read:
1. AGENTS.md
2. docs/CURRENT_STATE.md
3. $AuthoritativeLedger
4. $FoundationLedger
5. the complete Task $TaskId section in $TaskPlan
6. every commit and the full diff in $BaseCommit...HEAD
7. the implementer and repair reports under $relativeRunDirectory
8. $relativeRunDirectory/cuda-runtime-evidence.json

Review specification compliance first, then architecture, safety, code, tests, documentation truth, and scope. Verify that claimed failures or passes match the actual task semantics. $taskOneReviewNote Confirm mandatory CUDA runtime evidence: the JSON file must identify a positive Mistral.rs process ID, the selected model, and the observed NVIDIA compute-process row.

Do not modify, stage, commit, push, reset, restore, or delete tracked files. You may run read-only git commands and tests. Do not use OpenCode or a cloud model.

Write the complete review to $reportRelativePath.
The first line must be exactly VERDICT: PASS or VERDICT: FAIL.
A failing report must list each blocking or important finding with file evidence and a concrete correction. A passing report must account for every task acceptance condition, dependency boundary, documentation update, and CUDA evidence gate.
"@

    $sessionId = "3dmk-$TaskKind-$TaskId-reviewer-$Round-$([Guid]::NewGuid().ToString('N'))"
    Invoke-AgentResponse -BaseUri $BaseUri -Prompt $prompt -SessionId $sessionId -OutputPath $responsePath | Out-Null
    Assert-ReadOnlyRoleState -Worktree $Worktree -Role 'Reviewer' -ExpectedHead $expectedHead

    return [pscustomobject]@{
        Path = $reportPath
        Verdict = Get-ReportVerdict -ReportPath $reportPath
    }
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
    $reportRelativePath = "$relativeRunDirectory/verification-$Round.md"
    $reportPath = Join-Path $RunDirectory "verification-$Round.md"
    $responsePath = Join-Path $RunDirectory "verification-$Round-response.json"
    $worktreeLiteral = ConvertTo-PowerShellSingleQuotedLiteral -Value $Worktree
    $expectedHead = (Invoke-Git $Worktree rev-parse HEAD | Select-Object -First 1).Trim()
    $taskOneVerificationNote = if ($TaskKind -eq 'authoritative' -and $TaskId -eq '1') {
        'Interpret the expected red tests correctly: PASS means they compile, execute, and fail for the intended authority gaps rather than syntax, fixture, path, or harness errors.'
    }
    else {
        'Require the intended failure-reproducing test to pass after the production implementation and reject unexecuted or fabricated evidence.'
    }
    Remove-Item -LiteralPath $reportPath -Force -ErrorAction SilentlyContinue

    $prompt = @"
You are the independent 3DMk verifier for $TaskLabel. This is a read-only tracked-file role.

Use the shell. Your first shell command must be:
Set-Location -LiteralPath '$worktreeLiteral'

Read AGENTS.md, docs/CURRENT_STATE.md, $AuthoritativeLedger, $FoundationLedger, the full Task $TaskId section in $TaskPlan, $BaseCommit...HEAD, and $relativeRunDirectory/cuda-runtime-evidence.json. Run fresh verification commands rather than trusting another model session.

Verification requirements:
- confirm branch $Branch and a clean tracked worktree;
- confirm all selected-task predecessors remain complete in the appropriate ledgers;
- confirm mandatory CUDA runtime evidence exists and contains a positive process_id, the selected model, and a non-empty nvidia_compute_process row;
- run every focused command specified by Task $TaskId;
- run directly affected regression checks;
- confirm $TaskLedger records observed commands and outcomes;
- confirm docs/CURRENT_STATE.md remains accurate when implementation truth changed;
- confirm the commit contains only bounded task changes;
- $taskOneVerificationNote

Do not modify, stage, commit, push, reset, restore, or delete tracked files. Do not use OpenCode or a cloud model.

Write the complete verification report to $reportRelativePath.
The first line must be exactly VERDICT: PASS or VERDICT: FAIL.
Include each command, exit code, decisive output, dependency evidence, and CUDA evidence fields. A failure report must identify the exact repair required.
"@

    $sessionId = "3dmk-$TaskKind-$TaskId-verifier-$Round-$([Guid]::NewGuid().ToString('N'))"
    Invoke-AgentResponse -BaseUri $BaseUri -Prompt $prompt -SessionId $sessionId -OutputPath $responsePath | Out-Null
    Assert-ReadOnlyRoleState -Worktree $Worktree -Role 'Verifier' -ExpectedHead $expectedHead

    return [pscustomobject]@{
        Path = $reportPath
        Verdict = Get-ReportVerdict -ReportPath $reportPath
    }
}

function Invoke-Repair {
    param(
        [Parameter(Mandatory)][string]$BaseUri,
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$RunDirectory,
        [Parameter(Mandatory)][string]$Findings,
        [Parameter(Mandatory)][int]$Round
    )

    $before = (Invoke-Git $Worktree rev-parse HEAD | Select-Object -First 1).Trim()
    $repairArguments = @{
        BaseUri = $BaseUri
        Worktree = $Worktree
        RunDirectory = $RunDirectory
        RepairContext = $Findings
        Round = $Round
    }
    Invoke-Implementer @repairArguments

    Assert-CleanImplementationBranch -Worktree $Worktree
    $after = (Invoke-Git $Worktree rev-parse HEAD | Select-Object -First 1).Trim()
    if ($after -eq $before) {
        throw "Repair round $Round did not create a new commit."
    }
}

function Publish-VerifiedBranch {
    param(
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$RunDirectory,
        [Parameter(Mandatory)][string]$SelectedModel
    )

    if ($SkipPush) {
        Write-Warning 'Review and verification passed locally, but -SkipPush prevented GitHub synchronization.'
        return
    }

    $cudaEvidencePath = Join-Path $RunDirectory 'cuda-runtime-evidence.json'
    if (-not (Test-Path -LiteralPath $cudaEvidencePath -PathType Leaf)) {
        throw 'Publication is prohibited because cuda-runtime-evidence.json is missing.'
    }

    Write-Step 'Pushing the CUDA-verified, reviewed, and verified implementation branch.'
    Invoke-Git $Worktree push -u origin $Branch | Out-Null

    $gh = Get-CommandPath 'gh'
    if (-not $gh) {
        throw 'The branch was pushed, but GitHub CLI is unavailable; a draft PR could not be created or updated.'
    }

    & $gh auth status 2>&1 | Set-Content -LiteralPath (Join-Path $RunDirectory 'gh-auth-status.log') -Encoding utf8
    if ($LASTEXITCODE -ne 0) {
        throw 'The branch was pushed, but GitHub CLI is not authenticated; run gh auth login to create the draft PR.'
    }

    $prListArguments = @(
        'pr', 'list',
        '--repo', $RepositoryFullName,
        '--head', $Branch,
        '--base', $BaseBranch,
        '--state', 'open',
        '--json', 'number,url,isDraft'
    )
    $existingJson = & $gh @prListArguments 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw 'GitHub CLI could not inspect existing pull requests.'
    }

    $existing = @($existingJson | ConvertFrom-Json)
    if ($existing.Count -gt 0) {
        $existingJson | Set-Content -LiteralPath (Join-Path $RunDirectory 'pr.json') -Encoding utf8
        Write-Step "Existing PR found: $($existing[0].url)"
        return
    }

    $bodyPath = Join-Path $RunDirectory 'pr-body.md'
    @"
## Scope

Executes $TaskLabel through a local CUDA-backed Mistral.rs model. This PR remains task-gated and does not claim the complete 3DMk project is finished.

## Selected task

$TaskLabel from $TaskPlan.

## Execution backend

- Mistral.rs bound to 127.0.0.1
- local model: $SelectedModel
- CUDA build, host, and live process gates passed
- live process proof recorded in `.local-agent/$RunDirectoryName/cuda-runtime-evidence.json`
- independent implementer, reviewer, and verifier sessions
- OpenCode is not required

## Evidence

See $TaskLedger for exact task commands and outcomes. The ignored local run directory contains the CUDA process record and complete role reports.
"@ | Set-Content -LiteralPath $bodyPath -Encoding utf8

    $prCreateArguments = @(
        'pr', 'create',
        '--repo', $RepositoryFullName,
        '--draft',
        '--base', $BaseBranch,
        '--head', $Branch,
        '--title', "3DMk workflow: $TaskLabel",
        '--body-file', $bodyPath
    )
    & $gh @prCreateArguments
    if ($LASTEXITCODE -ne 0) { throw 'GitHub CLI failed to create the draft pull request.' }
}
