# 3DMk Mistral.rs Local Agent Runbook

## Purpose

This runbook replaces OpenCode as the active executor for the current experiment. It does not replace the authoritative product plan.

**Authoritative implementation plan:**

`docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`

**Implementation branch:**

`agent/vwm-authoritative-revision-implementation`

## Operating command

From any 3DMk checkout on Windows 11 with PowerShell 7:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 -Task 1
```

The script will:

1. find or install Mistral.rs;
2. prefer a native CUDA build;
3. resolve an isolated worktree for the implementation branch;
4. start Mistral.rs on `127.0.0.1`;
5. load the local coding model once;
6. execute independent implementer, reviewer, and verifier sessions;
7. repair failed review or verification findings up to three rounds;
8. push only a reviewed and verified task commit;
9. create or update a draft pull request when `gh` is authenticated.

## Default models

```text
Primary:  Qwen/Qwen3-Coder-30B-A3B-Instruct, quant 4
Fallback: Qwen/Qwen3-8B, quant 4
Smoke:    Qwen/Qwen3-4B, quant 4
```

Use a previously downloaded local model directory by passing it as `-Model`:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -Model 'D:\Models\Qwen3-Coder-30B-A3B-Instruct'
```

Use the smaller smoke model to validate the harness without committing product changes:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -Model 'Qwen/Qwen3-4B' `
  -DryRun
```

## CUDA setup

The official Windows prebuilt Mistral.rs binary is CPU-only. Native Windows CUDA therefore requires a source build with:

- Rust 1.94 or newer;
- Visual Studio 2022 Build Tools;
- an NVIDIA driver;
- the CUDA toolkit with `nvcc` on `PATH`.

Run:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\setup-mistralrs-local-agent.ps1
```

The setup script tries, in order:

1. `cuda flash-attn cudnn`;
2. `cuda` only;
3. the official CPU installer only when `-AllowCpuFallback` is supplied.

Explicit CPU fallback:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\setup-mistralrs-local-agent.ps1 -AllowCpuFallback
```

## Task controls

```powershell
# Execute Task 1 using defaults
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1

# Use the 8B fallback directly
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -Model 'Qwen/Qwen3-8B'

# Retain the model server after the task
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -KeepServer

# Do not push, even after local gates pass
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -SkipPush

# Run only the harness preflight
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -DryRun
```

## Evidence files

Transient local records are written beneath:

```text
.local-agent/task-XX/
```

They include:

- Mistral.rs stdout and stderr;
- complete Responses API payloads;
- implementer output;
- reviewer report;
- verifier report;
- repair-round reports;
- final controller summary.

This directory is ignored by git. Durable evidence belongs in:

`docs/agent-execution/VWM_PROGRESS.md`

## Required verdict format

Reviewer and verifier reports must begin with exactly one of:

```text
VERDICT: PASS
VERDICT: FAIL
```

A missing or malformed verdict is treated as failure.

## Stop conditions

The controller reports `BLOCKED` rather than guessing when:

- Mistral.rs cannot start either selected local model;
- the branch cannot be isolated safely;
- tracked user changes are present;
- the model fails three repair rounds;
- required tests cannot run because an external dependency or credential is absent;
- GitHub authentication is required to push and `-SkipPush` was not supplied.

## Prohibited behavior

- No Docker, Podman, WSL, or Electron.
- No cloud inference fallback.
- No OpenCode invocation.
- No work on `main` or `master`.
- No force push.
- No merge.
- No `git reset --hard`.
- No `git clean`.
- No deletion of unrelated user files.
- No completion claim without fresh reviewer and verifier evidence.