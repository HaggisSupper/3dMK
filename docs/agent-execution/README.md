# 3DMk Agent Execution

This directory contains the live execution runbook, task ledgers, acceptance matrix, and risk register for the current autonomous implementation program.

## Active executor

The active executor is the local, headless, CUDA-only Mistral.rs harness:

`MISTRALRS_LOCAL_AGENT.md`

OpenCode is not part of the current workflow. No OpenCode secret, GitHub Action, model, agent, command, or configuration is required or permitted.

## Ledgers

### Authoritative product workflow

`VWM_PROGRESS.md`

Tracks Tasks 1–17 for project/revision authority, exact evidence, processing, review UX, export, measurements, perception, frontend retirement, offline packaging, and final acceptance.

### CUDA foundation

`CUDA_FOUNDATION_PROGRESS.md`

Tracks FB1–FB8 for transactional persistence, immutable asset publication, accelerator contracts, CUDA runtime verification, the GPU resource broker, supervised compute, atomic result publication, and recovery/telemetry.

Authoritative Task 6 cannot begin until every CUDA-foundation item is independently reviewed, verified, and recorded complete.

## Quality and risk evidence

- `WORLD_CLASS_ACCEPTANCE_MATRIX.md` — objective release gates and failure dispositions.
- `WORLD_CLASS_RISK_REGISTER.md` — active risks, consequences, controls, and required evidence.

## Standard task execution

From the repository root on the supported Windows/NVIDIA host:

```powershell
pwsh -NoLogo -NoProfile `
  -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1
```

The controller:

1. validates the CUDA-enabled Mistral.rs installation;
2. verifies the exact server process is using CUDA;
3. creates or reuses the isolated implementation worktree;
4. runs separate implementer, reviewer, and verifier sessions;
5. performs bounded repair rounds;
6. updates the appropriate ledger with actual evidence;
7. pushes only after both independent gates pass;
8. creates or updates a draft pull request;
9. never merges the pull request.

## Evidence rule

Only observed command output belongs in a progress ledger. A model response, code diff, route, capability flag, document, or passing static parser is not sufficient evidence that a product task or CUDA capability works.

## Stop conditions

The controller reports `BLOCKED` rather than changing architecture or weakening requirements when:

- CUDA cannot be verified;
- Mistral.rs cannot load an approved CUDA-backed model profile;
- the implementation worktree is not clean and isolated;
- required credentials or licensed model assets are absent;
- tests cannot execute because a required local toolchain is missing;
- three bounded repair rounds fail;
- the governing documents do not contain an irreversible product decision.

## Historical material

Superseded OpenCode and experiment-specific documents have been deleted. Git history is the archive; the active tree contains only the current execution system.