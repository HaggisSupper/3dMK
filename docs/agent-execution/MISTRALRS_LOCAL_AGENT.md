# 3DMk Mistral.rs Local Agent Runbook

## Purpose

This runbook replaces OpenCode as the active executor for the current experiment. It does not replace or reorder the authoritative 3DMk product plan.

**Authoritative implementation plan:**

`docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`

**Implementation branch:**

`agent/vwm-authoritative-revision-implementation`

## Operating command

From the 3DMk repository on Windows 11 with PowerShell 7:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 -Task 1
```

The controller will:

1. validate or build Mistral.rs;
2. require a native CUDA build by default;
3. park a clean primary checkout on `main` when necessary;
4. create or reuse an isolated worktree for the implementation branch;
5. bind Mistral.rs only to `127.0.0.1`;
6. load the selected local coding model once;
7. execute independent implementer, reviewer, and verifier model sessions;
8. repair failed review or verification findings up to three rounds;
9. push only a reviewed and verified task commit;
10. create or update a draft pull request when `gh` is authenticated.

The script never merges the pull request.

## Default models

```text
Primary:  Qwen/Qwen3-Coder-30B-A3B-Instruct, quant 4
Fallback: Qwen/Qwen3-8B, quant 4
Smoke:    Qwen/Qwen3-4B, quant 4
Context:  32,768 tokens by default
```

The primary model is sparse: 30.5 billion total parameters with 3.3 billion activated per token. On the reference RTX 4050 6 GB system, Mistral.rs may place quantized weights outside VRAM. The 8B fallback is retained because the primary model may still exceed practical startup or memory limits.

Use a previously downloaded local model directory by passing it as `-Model`:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -Model 'D:\Models\Qwen3-Coder-30B-A3B-Instruct'
```

Use the smaller model directly when the 30B-A3B model is not viable:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -Model 'Qwen/Qwen3-8B' `
  -FallbackModel 'Qwen/Qwen3-4B'
```

A `-DryRun` validates the local toolchain and worktree without loading a model or changing product files:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -DryRun
```

## CUDA setup

The official Windows prebuilt Mistral.rs binary is CPU-only. Native Windows CUDA therefore requires a source build with:

- Rust 1.94 or newer;
- Visual Studio 2022 C++ Build Tools;
- an NVIDIA driver;
- the CUDA toolkit with `nvcc` on `PATH`.

Run:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\setup-mistralrs-local-agent.ps1
```

The setup script builds the current agent-capable Mistral.rs source and records the exact source commit and installed binary hash. It tries, in order:

1. `cuda flash-attn cudnn`;
2. `cuda` only.

CPU is not an automatic fallback. Permit it explicitly only when the degraded speed is acceptable:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\setup-mistralrs-local-agent.ps1 -AllowCpuFallback
```

The task controller requires the same explicit switch when using an unconfirmed CPU installation:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -AllowCpuFallback
```

Mistral.rs does not currently provide a native Windows Vulkan or WebGPU inference fallback. The harness does not claim otherwise.

## Task controls

```powershell
# Execute Task 1 using the CUDA-first defaults
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1

# Use the 8B model directly
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -Model 'Qwen/Qwen3-8B'

# Reduce context if model startup is memory constrained
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -ContextLength 16384

# Retain the model server after the task
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -KeepServer

# Do not push, even after local gates pass
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -SkipPush

# Run only setup/worktree preflight
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -DryRun
```

## Worktree behavior

The local model is never allowed to edit `main` or `master`.

- If the implementation branch already has a linked worktree, the controller reuses it.
- If the script is launched while the implementation branch is checked out in the primary repository, the controller first requires that checkout to be clean, switches the primary checkout to `main`, and then creates a linked worktree.
- If the primary checkout has tracked or untracked changes, the controller stops rather than moving or deleting them.
- The implementation worktree lives under `%LOCALAPPDATA%\3DMk\worktrees` when available.

## Execution roles

### Implementer

The implementer reads `AGENTS.md`, the progress ledger, and the selected authoritative plan task. It uses the Mistral.rs shell tool to write tests, observe the required failure, implement bounded changes where the task requires production code, run checks, update the ledger, and create a local commit. It does not push.

### Reviewer

The reviewer is a separate model request. It reads the complete task diff and requirements and writes a specification/code-quality verdict. The controller rejects any reviewer session that changes HEAD or tracked files.

### Verifier

The verifier is another separate model request. It reruns the task commands and records exit codes and decisive output. The controller rejects any verifier session that changes HEAD or tracked files.

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

This directory is ignored by git. Durable task evidence belongs in:

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
- tracked or untracked user changes are present in the implementation worktree;
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
