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
You are the local Mistral.rs implementation worker for 3DMk Task $Task.

Use the shell tool continuously; do not merely describe commands. Your first shell command must be:
Set-Location -LiteralPath '$worktreeLiteral'

Read, in this exact order:
1. AGENTS.md
2. $ProgressLedger
3. $AuthoritativePlan
4. The complete section headed Task $Task and only its directly referenced files.

Mandatory rules:
- Confirm the current branch is exactly $Branch and never switch to main or master.
- Execute only Task $Task. Do not begin the next task.
- Follow test-driven development and the task's exact acceptance conditions.
- For Task 1, intended red tests are evidence; do not change production behavior merely to make them green.
- Use real repository APIs and fixtures. Do not manufacture failures.
- Run the focused commands and affected regression checks required by the task.
- Update $ProgressLedger with exact commands, outputs, files, commit, and residual risks.
- Do not use Docker, Podman, WSL, OpenCode, cloud inference, force push, merge, git reset --hard, or git clean.
- Preserve unrelated files.
- Commit all tracked task changes locally with the prescribed message or a more accurate equivalent.
- Do not push. The controller pushes only after independent gates pass.
- Write a concise execution report to $reportRelativePath before finishing. Include the commit SHA and exact observed test outcomes.
- End your final response with one state: TASK_CANDIDATE, SESSION_BOUNDARY, or BLOCKED.
$repairSection
Begin now and continue until this bounded task has a committed candidate or a genuine evidence-backed blocker.
"@

    $sessionId = "3dmk-task-$Task-implementer-$Round-$([Guid]::NewGuid().ToString('N'))"
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
    Remove-Item -LiteralPath $reportPath -Force -ErrorAction SilentlyContinue

    $prompt = @"
You are the independent 3DMk reviewer for authoritative plan Task $Task. This is a read-only tracked-file role.

Use the shell. Your first shell command must be:
Set-Location -LiteralPath '$worktreeLiteral'

Read:
1. AGENTS.md
2. $ProgressLedger
3. the complete Task $Task section in $AuthoritativePlan
4. every commit and the full diff in $BaseCommit...HEAD
5. the implementer and repair reports under $relativeRunDirectory

Review specification compliance first, then code and test quality. Verify that claimed failures or passes match the actual task semantics. Task 1 intentionally requires red tests that reproduce real authority defects; do not demand production fixes in Task 1.

Do not modify, stage, commit, push, reset, restore, or delete tracked files. You may run read-only git commands and tests. Do not use OpenCode or a cloud model.

Write the complete review to $reportRelativePath.
The first line must be exactly VERDICT: PASS or VERDICT: FAIL.
A failing report must list each blocking or important finding with file evidence and a concrete correction. A passing report must account for every task acceptance condition.
"@

    $sessionId = "3dmk-task-$Task-reviewer-$Round-$([Guid]::NewGuid().ToString('N'))"
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
    Remove-Item -LiteralPath $reportPath -Force -ErrorAction SilentlyContinue

    $prompt = @"
You are the independent 3DMk verifier for authoritative plan Task $Task. This is a read-only tracked-file role.

Use the shell. Your first shell command must be:
Set-Location -LiteralPath '$worktreeLiteral'

Read AGENTS.md, $ProgressLedger, the full Task $Task plan section, and $BaseCommit...HEAD. Run fresh verification commands rather than trusting another model session.

Verification requirements:
- confirm branch $Branch and a clean tracked worktree;
- run every focused command specified by Task $Task;
- run directly affected regression checks;
- confirm the progress ledger records observed commands and outcomes;
- confirm the commit contains only bounded task changes;
- interpret Task 1's expected red tests correctly: PASS means they compile, execute, and fail for the intended authority gaps rather than syntax, fixture, path, or harness errors;
- reject fabricated or unexecuted evidence.

Do not modify, stage, commit, push, reset, restore, or delete tracked files. Do not use OpenCode or a cloud model.

Write the complete verification report to $reportRelativePath.
The first line must be exactly VERDICT: PASS or VERDICT: FAIL.
Include each command, exit code, and decisive output. A failure report must identify the exact repair required.
"@

    $sessionId = "3dmk-task-$Task-verifier-$Round-$([Guid]::NewGuid().ToString('N'))"
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

Executes the authoritative 3DMk VWM revision workflow through a local Mistral.rs model. This PR remains task-gated and does not claim the complete 17-task project is finished.

## Current task

Task $Task from $AuthoritativePlan.

## Execution backend

- Mistral.rs bound to 127.0.0.1
- local model: $SelectedModel
- independent implementer, reviewer, and verifier sessions
- OpenCode is not required

## Evidence

See $ProgressLedger for exact commands and outcomes.
"@ | Set-Content -LiteralPath $bodyPath -Encoding utf8

    $prCreateArguments = @(
        'pr', 'create',
        '--repo', $RepositoryFullName,
        '--draft',
        '--base', $BaseBranch,
        '--head', $Branch,
        '--title', "3DMk authoritative revision workflow: Task $Task",
        '--body-file', $bodyPath
    )
    & $gh @prCreateArguments
    if ($LASTEXITCODE -ne 0) { throw 'GitHub CLI failed to create the draft pull request.' }
}
