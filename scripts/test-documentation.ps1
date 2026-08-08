[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path

function Resolve-RepositoryPath {
    param([Parameter(Mandatory)][string]$RelativePath)
    Join-Path $RepositoryRoot ($RelativePath -replace '/', [IO.Path]::DirectorySeparatorChar)
}

function Require-File {
    param([Parameter(Mandatory)][string]$RelativePath)
    if (-not (Test-Path -LiteralPath (Resolve-RepositoryPath $RelativePath) -PathType Leaf)) {
        throw "Required authoritative document is missing: $RelativePath"
    }
}

function Forbid-Path {
    param([Parameter(Mandatory)][string]$RelativePath)
    if (Test-Path -LiteralPath (Resolve-RepositoryPath $RelativePath)) {
        throw "Superseded documentation/tooling is still present: $RelativePath"
    }
}

function Get-TrackedRepositoryFiles {
    $git = (Get-Command git -ErrorAction Stop).Source
    $relativePaths = @(& $git -C $RepositoryRoot ls-files)
    if ($LASTEXITCODE -ne 0) {
        throw 'git ls-files failed while enumerating authoritative repository files.'
    }

    foreach ($relativePath in $relativePaths) {
        if ([string]::IsNullOrWhiteSpace($relativePath)) { continue }
        $resolvedPath = Resolve-RepositoryPath $relativePath
        if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf)) {
            throw "Tracked repository file is missing from the checkout: $relativePath"
        }
        Get-Item -LiteralPath $resolvedPath
    }
}

$requiredFiles = @(
    'README.md',
    'AGENTS.md',
    'docs/README.md',
    'docs/CURRENT_STATE.md',
    'docs/architecture/decisions/ADR-001-transactional-project-store.md',
    'docs/architecture/decisions/ADR-002-supervised-cuda-worker.md',
    'docs/architecture/decisions/ADR-003-tiered-cache.md',
    'docs/agent-execution/README.md',
    'docs/agent-execution/MISTRALRS_LOCAL_AGENT.md',
    'docs/agent-execution/VWM_PROGRESS.md',
    'docs/agent-execution/CUDA_FOUNDATION_PROGRESS.md',
    'docs/agent-execution/WORLD_CLASS_ACCEPTANCE_MATRIX.md',
    'docs/agent-execution/WORLD_CLASS_RISK_REGISTER.md',
    'docs/superpowers/specs/2026-08-07-3dmk-world-class-quality-standard.md',
    'docs/superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md',
    'docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md',
    'docs/superpowers/plans/2026-08-07-3dmk-foundation-batch.md',
    'docs/superpowers/plans/2026-08-07-3dmk-world-class-program.md',
    'VWM-Repo-Implicit/README.md',
    'VWM-Repo-Implicit/BUILD_STATUS.md',
    'VWM-Repo-Implicit/docs/perception-integration.md',
    'VWM-Repo-Implicit/docs/implicit-field-integration.md',
    'Open Design Prototypes/README.md',
    'spatial-engineering-platform/README.md'
)

$forbiddenPaths = @(
    'opencode.json',
    '.github/workflows/opencode-big-pickle.yml',
    '.opencode/agents/vwm-executor.md',
    '.opencode/agents/vwm-reviewer.md',
    '.opencode/agents/vwm-verifier.md',
    '.opencode/commands/implement-vwm.md',
    'scripts/start-opencode-vwm.ps1',
    'scripts/test-opencode-vwm-harness.ps1',
    'docs/3dmk-vwm-feature-merge-plan.md',
    'docs/status/phase-0-completion.md',
    'docs/status/phase-1-completion.md',
    'docs/status/phase-2-package-ingest.md',
    'docs/baseline/2026-07-18-pre-merge.md',
    'docs/integration/spatial-platform-boundary.md',
    'docs/superpowers/plans/2026-07-31-opencode-vwm-execution-harness.md',
    'docs/superpowers/specs/2026-07-31-opencode-vwm-execution-harness-design.md',
    'docs/superpowers/plans/2026-08-07-mistralrs-local-agent-experiment.md',
    'docs/superpowers/specs/2026-08-07-mistralrs-local-agent-experiment-design.md',
    'VWM-Repo-Implicit/docs/3dmk-implicit-engine-port-plan.md',
    'VWM-Repo-Implicit/docs/superpowers/plans/2026-07-18-vwm-implicit-fields.md',
    'VWM-Repo-Implicit/PACKAGE_MANIFEST.sha256',
    'VWM-Repo-Implicit/STATIC_VALIDATION.json',
    'spatial-engineering-platform/INTEGRATION_PLAN.md',
    'spatial-engineering-platform/bootstrap/README.md',
    'spatial-engineering-platform/bootstrap/PR_BODY.md',
    'spatial-engineering-platform/bootstrap/VERIFY_AFTER_EXTRACT.txt',
    'spatial-engineering-platform/bootstrap/BRANCH_COMPLETE.txt',
    'spatial-engineering-platform/bootstrap/LAST_FILE.txt',
    'spatial-engineering-platform/bootstrap/PUBLISH_STATUS.md',
    'spatial-engineering-platform/bootstrap/NO_MORE_FILES.txt',
    'spatial-engineering-platform/bootstrap/NEXT_ACTION.txt',
    'spatial-engineering-platform/bootstrap/FINAL_NOTE.txt',
    'spatial-engineering-platform/bootstrap/OPEN_PR_NOW.txt',
    'spatial-engineering-platform/bootstrap/install-spatial-platform.ps1',
    'Open Design Prototypes/3DMk-UI_UX-2026-07-17/DESIGN-HANDOFF.md',
    'Open Design Prototypes/3DMk-UI_UX-2026-07-17/DESIGN-MANIFEST.json',
    'Open Design Prototypes/3DMk-photo-package-import/DESIGN-HANDOFF.md',
    'Open Design Prototypes/3DMk-photo-package-import/DESIGN-MANIFEST.json',
    'Open Design Prototypes/3DMk-photo-package-import/brand-spec.md',
    'Open Design Prototypes/3DMk-VWM-Perception-Port-UI-Plan/DESIGN-HANDOFF.md',
    'Open Design Prototypes/3DMk-VWM-Perception-Port-UI-Plan/DESIGN-MANIFEST.json',
    'Open Design Prototypes/3DMk-VWM-Perception-Port-UI-Plan/brand-spec.md',
    'Open Design Prototypes/3DMk-Implicit-Reconstruction-Workflow/DESIGN-HANDOFF.md',
    'Open Design Prototypes/3DMk-Implicit-Reconstruction-Workflow/DESIGN-MANIFEST.json',
    'Open Design Prototypes/3DMk-Implicit-Reconstruction-Workflow/brand-spec.md'
)

$requiredFiles | ForEach-Object { Require-File $_ }
$forbiddenPaths | ForEach-Object { Forbid-Path $_ }

$rootReadme = Get-Content -LiteralPath (Resolve-RepositoryPath 'README.md') -Raw
foreach ($required in @(
    'docs/CURRENT_STATE.md',
    'docs/README.md',
    'CUDA requirement',
    'not yet deployment-ready'
)) {
    if (-not $rootReadme.Contains($required, [StringComparison]::OrdinalIgnoreCase)) {
        throw "README.md is missing current-state contract text: $required"
    }
}

$currentState = Get-Content -LiteralPath (Resolve-RepositoryPath 'docs/CURRENT_STATE.md') -Raw
foreach ($required in @(
    'not deployment-ready',
    'recoverable JSON',
    'product CUDA runtime',
    'not implemented',
    'PROJECT_COMPLETE'
)) {
    if (-not $currentState.Contains($required, [StringComparison]::OrdinalIgnoreCase)) {
        throw "CURRENT_STATE.md is missing implementation-truth text: $required"
    }
}

$agents = Get-Content -LiteralPath (Resolve-RepositoryPath 'AGENTS.md') -Raw
foreach ($required in @(
    'docs/CURRENT_STATE.md',
    'CUDA Foundation FB1–FB8',
    'CPU LLM inference and cloud inference fallback are prohibited.',
    'Reviewer and verifier sessions are read-only',
    'Evidence before assertion. No exceptions.'
)) {
    if (-not $agents.Contains($required, [StringComparison]::Ordinal)) {
        throw "AGENTS.md is missing governing text: $required"
    }
}

$probeDirectory = Resolve-RepositoryPath '.local-agent/documentation-validation'
$probePath = Join-Path $probeDirectory ("untracked-{0}.md" -f [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $probeDirectory | Out-Null
Set-Content -LiteralPath $probePath -Encoding utf8 -Value 'OPENCODE_API_KEY [broken](missing-transient-target.md)'
try {
    $trackedFiles = @(Get-TrackedRepositoryFiles)
    if ($trackedFiles.FullName -contains (Get-Item -LiteralPath $probePath).FullName) {
        throw 'git ls-files unexpectedly returned an ignored transient documentation probe.'
    }
}
finally {
    Remove-Item -LiteralPath $probePath -Force -ErrorAction SilentlyContinue
}

$activeTextFiles = $trackedFiles |
    Where-Object { $_.Extension -in @('.md', '.json', '.yml', '.yaml', '.ps1') }

$forbiddenText = @(
    'OPENCODE_API_KEY',
    'opencode/big-pickle',
    'anomalyco/opencode',
    'docs/3dmk-vwm-feature-merge-plan.md',
    '2026-07-31-opencode-vwm-execution-harness',
    '2026-08-07-mistralrs-local-agent-experiment',
    'Phase 0 completion —',
    'Phase 1 completion —',
    'Phase 2 package ingest and review status'
)
$scanExclusions = @(
    (Resolve-RepositoryPath 'scripts/test-documentation.ps1'),
    (Resolve-RepositoryPath 'scripts/test-mistralrs-local-agent-harness.ps1')
)
foreach ($file in $activeTextFiles) {
    if ($scanExclusions -contains $file.FullName) { continue }
    $content = Get-Content -LiteralPath $file.FullName -Raw
    foreach ($forbidden in $forbiddenText) {
        if ($content.Contains($forbidden, [StringComparison]::OrdinalIgnoreCase)) {
            $relative = [IO.Path]::GetRelativePath($RepositoryRoot, $file.FullName)
            throw "$relative contains superseded reference: $forbidden"
        }
    }
}

$markdownFiles = $trackedFiles | Where-Object { $_.Extension -eq '.md' }
$linkPattern = [regex]'\[[^\]]*\]\((?<target>[^)]+)\)'
foreach ($file in $markdownFiles) {
    $content = Get-Content -LiteralPath $file.FullName -Raw
    foreach ($match in $linkPattern.Matches($content)) {
        $target = $match.Groups['target'].Value.Trim()
        if (-not $target -or $target.StartsWith('#')) { continue }
        if ($target -match '^(?i:https?|mailto|file|data|javascript):') { continue }
        $target = ($target -split '#', 2)[0]
        $target = [Uri]::UnescapeDataString($target)
        if (-not $target) { continue }
        $resolved = if ([IO.Path]::IsPathRooted($target)) {
            $target
        } else {
            Join-Path $file.DirectoryName ($target -replace '/', [IO.Path]::DirectorySeparatorChar)
        }
        if (-not (Test-Path -LiteralPath $resolved)) {
            $relative = [IO.Path]::GetRelativePath($RepositoryRoot, $file.FullName)
            throw "$relative contains a broken local link: $target"
        }
    }
}

Write-Host '3DMK_DOCUMENTATION_OK'
