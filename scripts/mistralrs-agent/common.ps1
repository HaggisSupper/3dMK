# Common functions for the 3DMk local Mistral.rs task controller.
# Dot-source this file from scripts/run-mistralrs-vwm-task.ps1.

function Write-Step {
    param([Parameter(Mandatory)][string]$Message)
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

function Test-IsLinkedWorktree {
    param([Parameter(Mandatory)][string]$WorkingDirectory)

    $gitDirectory = (Invoke-Git $WorkingDirectory rev-parse --path-format=absolute --git-dir | Select-Object -First 1).Trim()
    $commonDirectory = (Invoke-Git $WorkingDirectory rev-parse --path-format=absolute --git-common-dir | Select-Object -First 1).Trim()
    return -not [string]::Equals(
        [IO.Path]::GetFullPath($gitDirectory).TrimEnd('\'),
        [IO.Path]::GetFullPath($commonDirectory).TrimEnd('\'),
        [StringComparison]::OrdinalIgnoreCase
    )
}

function Get-FreeTcpPort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try {
        return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    }
    finally {
        $listener.Stop()
    }
}

function ConvertTo-NativeArgument {
    param([Parameter(Mandatory)][string]$Value)

    if ($Value -notmatch '[\s"]') { return $Value }
    $escaped = $Value -replace '(\\*)"', '$1$1\"'
    return '"' + $escaped + '"'
}

function ConvertTo-PowerShellSingleQuotedLiteral {
    param([Parameter(Mandatory)][string]$Value)
    return $Value.Replace("'", "''")
}

function Get-ExistingBranchWorktree {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$BranchName
    )

    $lines = Invoke-Git $RepositoryRoot worktree list --porcelain
    $candidatePath = $null
    foreach ($line in @($lines) + '') {
        if ($line -like 'worktree *') {
            $candidatePath = $line.Substring(9)
            continue
        }
        if ($line -eq "branch refs/heads/$BranchName" -and $candidatePath) {
            return $candidatePath
        }
        if ([string]::IsNullOrWhiteSpace($line)) {
            $candidatePath = $null
        }
    }
    return $null
}

function Assert-CleanWorktree {
    param(
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$Purpose,
        [switch]$IgnoreUntracked
    )

    $arguments = @('status', '--porcelain')
    if ($IgnoreUntracked) { $arguments += '--untracked-files=no' }
    $status = @(Invoke-Git $Worktree @arguments)
    if ($status.Count -gt 0 -and ($status -join '').Trim()) {
        throw "$Purpose requires a clean tracked state:`n$($status -join [Environment]::NewLine)"
    }
}

function Resolve-AgentWorktree {
    param([Parameter(Mandatory)][string]$RepositoryRoot)

    Write-Step 'Fetching the implementation branch from GitHub.'
    Invoke-Git $RepositoryRoot fetch --prune origin | Out-Null

    $existing = Get-ExistingBranchWorktree -RepositoryRoot $RepositoryRoot -BranchName $Branch
    if ($existing) {
        $resolvedExisting = (Resolve-Path -LiteralPath $existing).Path
        $resolvedRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path

        if ([string]::Equals($resolvedExisting, $resolvedRoot, [StringComparison]::OrdinalIgnoreCase)) {
            if (Test-IsLinkedWorktree -WorkingDirectory $resolvedRoot) {
                Write-Step "The controller is already running from the isolated implementation worktree: $resolvedRoot"
                Invoke-Git $resolvedRoot pull --ff-only origin $Branch | Out-Null
                return $resolvedRoot
            }

            Assert-CleanWorktree -Worktree $resolvedRoot -Purpose 'Relocating the implementation branch from the primary checkout'
            Write-Step "Parking the clean primary checkout on $BaseBranch before creating the isolated implementation worktree."
            Invoke-Git $resolvedRoot switch $BaseBranch | Out-Null
            Invoke-Git $resolvedRoot pull --ff-only origin $BaseBranch | Out-Null
            $existing = $null
        }
        else {
            Write-Step "Using existing implementation worktree: $resolvedExisting"
            Invoke-Git $resolvedExisting pull --ff-only origin $Branch | Out-Null
            return $resolvedExisting
        }
    }

    $worktreeBase = if ($env:LOCALAPPDATA) {
        Join-Path $env:LOCALAPPDATA '3DMk\worktrees'
    }
    else {
        Join-Path $env:TEMP '3DMk\worktrees'
    }
    $worktreePath = Join-Path $worktreeBase 'vwm-authoritative-revision-implementation'
    New-Item -ItemType Directory -Force -Path $worktreeBase | Out-Null

    if (Test-Path -LiteralPath $worktreePath) {
        throw "The intended worktree path exists but is not registered with git. It was not deleted automatically: $worktreePath"
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

    Invoke-Git $worktreePath pull --ff-only origin $Branch | Out-Null
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
    if (-not (Test-IsLinkedWorktree -WorkingDirectory $Worktree)) {
        throw 'The implementation branch is not isolated in a linked git worktree.'
    }
    Assert-CleanWorktree -Worktree $Worktree -Purpose 'Local-agent execution'
}

function Get-MistralStateRoot {
    if ($env:LOCALAPPDATA) { return (Join-Path $env:LOCALAPPDATA 'mistralrs-source') }
    return (Join-Path $env:TEMP 'mistralrs-source')
}

function Test-CudaMarker {
    param([Parameter(Mandatory)][string]$MistralPath)

    $markerPath = Join-Path (Get-MistralStateRoot) 'cuda-install.json'
    if (-not (Test-Path -LiteralPath $markerPath)) { return $false }

    try {
        $marker = Get-Content -LiteralPath $markerPath -Raw | ConvertFrom-Json
        $resolvedBinary = (Resolve-Path -LiteralPath $MistralPath).Path
        $binaryHash = (Get-FileHash -LiteralPath $resolvedBinary -Algorithm SHA256).Hash
        return $marker.binary_path -eq $resolvedBinary -and $marker.binary_sha256 -eq $binaryHash
    }
    catch {
        return $false
    }
}

function Ensure-MistralRs {
    param([Parameter(Mandatory)][string]$RepositoryRoot)

    $pwsh = Get-CommandPath 'pwsh'
    if (-not $pwsh) { throw 'PowerShell 7 (pwsh) is required.' }

    $setup = Join-Path $RepositoryRoot 'scripts\setup-mistralrs-local-agent.ps1'
    if (-not (Test-Path -LiteralPath $setup)) {
        throw "Mistral.rs setup script is missing: $setup"
    }

    $setupArguments = @('-NoLogo', '-NoProfile', '-File', $setup)
    if ($AllowCpuFallback) { $setupArguments += '-AllowCpuFallback' }
    & $pwsh @setupArguments
    if ($LASTEXITCODE -ne 0) { throw 'Mistral.rs setup failed.' }

    $env:PATH = "$(Join-Path $env:USERPROFILE '.cargo\bin');$(Join-Path $env:USERPROFILE '.local\bin');$env:PATH"
    $mistral = Get-CommandPath 'mistralrs'
    if (-not $mistral) { throw 'mistralrs is unavailable after setup.' }

    $doctor = (& $mistral doctor 2>&1 | Out-String)
    if ($doctor) { Write-Host $doctor.TrimEnd() }

    $cudaConfirmed = Test-CudaMarker -MistralPath $mistral
    if (-not $cudaConfirmed -and -not $AllowCpuFallback) {
        throw 'The installed Mistral.rs binary is not confirmed by the CUDA source-build marker. Rerun setup or explicitly pass -AllowCpuFallback.'
    }
    if (-not $cudaConfirmed) {
        Write-Warning 'Mistral.rs is running in explicitly allowed degraded CPU mode.'
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
            $stdout = if (Test-Path -LiteralPath $StdoutPath) { Get-Content -LiteralPath $StdoutPath -Raw } else { '' }
            $stderr = if (Test-Path -LiteralPath $StderrPath) { Get-Content -LiteralPath $StderrPath -Raw } else { '' }
            throw "Mistral.rs exited before becoming ready.`nSTDOUT:`n$stdout`nSTDERR:`n$stderr"
        }

        try {
            $models = Invoke-RestMethod -Method Get -Uri "$BaseUri/v1/models" -TimeoutSec 3
            if ($models) { return }
        }
        catch {
            Start-Sleep -Seconds 2
        }
    }

    Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
    throw "Mistral.rs did not become ready within $TimeoutSeconds seconds. Inspect $StdoutPath and $StderrPath."
}

function Start-MistralServer {
    param(
        [Parameter(Mandatory)][string]$MistralPath,
        [Parameter(Mandatory)][string]$ModelId,
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$RunDirectory,
        [Parameter(Mandatory)][int]$ListenPort,
        [Parameter(Mandatory)][string]$AttemptName
    )

    $stdoutPath = Join-Path $RunDirectory "mistralrs-$AttemptName.stdout.log"
    $stderrPath = Join-Path $RunDirectory "mistralrs-$AttemptName.stderr.log"
    $pwsh = Get-CommandPath 'pwsh'
    if (-not $pwsh) { throw 'PowerShell 7 (pwsh) is required for the local shell executor.' }

    $arguments = @(
        'serve',
        '--host', '127.0.0.1',
        '--port', [string]$ListenPort,
        '--quant', [string]$Quant,
        '--max-seq-len', [string]$ContextLength,
        '--max-tool-rounds', [string]$MaxToolRounds,
        '-m', $ModelId,
        '--enable-shell',
        '--shell-path', $pwsh,
        '--shell-workdir', $Worktree,
        '--shell-timeout', '1800',
        '--agent-permission', 'auto',
        '--sandbox', 'off',
        '--no-ui'
    )

    $quotedArguments = $arguments | ForEach-Object { ConvertTo-NativeArgument -Value $_ }
    Write-Step "Starting Mistral.rs with local model '$ModelId'."
    $startProcessArguments = @{
        FilePath = $MistralPath
        ArgumentList = $quotedArguments
        PassThru = $true
        WindowStyle = 'Hidden'
        RedirectStandardOutput = $stdoutPath
        RedirectStandardError = $stderrPath
    }
    $process = Start-Process @startProcessArguments

    $baseUri = "http://127.0.0.1:$ListenPort"
    $waitArguments = @{
        Process = $process
        BaseUri = $baseUri
        TimeoutSeconds = $ServerStartTimeoutSeconds
        StdoutPath = $stdoutPath
        StderrPath = $stderrPath
    }
    Wait-MistralServer @waitArguments

    $process.Id | Set-Content -LiteralPath (Join-Path $RunDirectory 'mistralrs.pid') -Encoding ascii
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
        session_id = $SessionId
        tools = @(
            [ordered]@{
                type = 'shell'
                environment = [ordered]@{ type = 'container_auto' }
            }
        )
        tool_choice = 'auto'
        max_tool_rounds = $MaxToolRounds
        max_output_tokens = 16384
    }

    $body = $payload | ConvertTo-Json -Depth 20
    $requestArguments = @{
        Method = 'Post'
        Uri = "$BaseUri/v1/responses"
        ContentType = 'application/json'
        Body = $body
        TimeoutSec = 7200
    }
    $response = Invoke-RestMethod @requestArguments

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

function Assert-ReadOnlyRoleState {
    param(
        [Parameter(Mandatory)][string]$Worktree,
        [Parameter(Mandatory)][string]$Role,
        [Parameter(Mandatory)][string]$ExpectedHead
    )

    $head = (Invoke-Git $Worktree rev-parse HEAD | Select-Object -First 1).Trim()
    if ($head -ne $ExpectedHead) {
        throw "$Role changed HEAD from $ExpectedHead to $head despite its read-only contract."
    }
    Assert-CleanWorktree -Worktree $Worktree -Purpose "$Role read-only verification" -IgnoreUntracked
}
