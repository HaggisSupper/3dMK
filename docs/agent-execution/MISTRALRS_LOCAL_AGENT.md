# 3DMk Mistral.rs Local Agent Runbook

## Purpose

This runbook replaces OpenCode as the active executor for the current experiment. It does not replace or reorder the authoritative 3DMk product plan.

**Authoritative implementation plan:**

`docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`

**Implementation branch:**

`agent/vwm-authoritative-revision-implementation`

## Non-negotiable accelerator rule

**CUDA is mandatory. CPU execution is prohibited for this experiment.**

The controller must establish all three forms of evidence before any model-driven repository work begins:

1. `mistralrs doctor` reports that the binary was compiled with the `cuda` feature;
2. `mistralrs doctor` reports a detected CUDA toolkit and NVIDIA driver;
3. after the model server becomes API-ready, `nvidia-smi` reports the exact Mistral.rs process ID in the CUDA compute-process table.

Failure of any gate stops execution as `BLOCKED`. Selecting a smaller model does not weaken the CUDA requirement.

## Operating command

From the 3DMk repository on Windows 11 with PowerShell 7:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 -Task 1
```

The controller will:

1. validate or build a CUDA-enabled Mistral.rs binary;
2. verify NVIDIA driver and CUDA-toolkit diagnostics;
3. park a clean primary checkout on `main` when necessary;
4. create or reuse an isolated worktree for the implementation branch;
5. bind Mistral.rs only to `127.0.0.1`;
6. load the selected local coding model;
7. prove that the live server process owns a CUDA compute context;
8. execute independent implementer, reviewer, and verifier model sessions;
9. repair failed review or verification findings up to three rounds;
10. push only a reviewed and verified task commit;
11. create or update a draft pull request when `gh` is authenticated.

The script never merges the pull request.

## Default models

```text
Primary:  Qwen/Qwen3-Coder-30B-A3B-Instruct, quant 4
Fallback: Qwen/Qwen3-8B, quant 4
Smoke:    Qwen/Qwen3-4B, quant 4
Context:  32,768 tokens by default
```

The primary model is sparse: 30.5 billion total parameters with 3.3 billion activated per token. On the reference RTX 4050 6 GB system, Mistral.rs may map some quantized weights outside VRAM while still executing supported operations through CUDA. The 8B and 4B choices exist only to reduce memory pressure; both remain subject to the live CUDA-process gate.

Use a previously downloaded local model directory by passing it as `-Model`:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -Model 'D:\Models\Qwen3-Coder-30B-A3B-Instruct'
```

Use the smaller model ladder when the primary model cannot become CUDA-ready:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -Model 'Qwen/Qwen3-8B' `
  -FallbackModel 'Qwen/Qwen3-4B'
```

A `-DryRun` validates the CUDA toolchain and worktree without loading a model or changing product files:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -DryRun
```

## CUDA setup

The Windows release artifact does not provide the required CUDA execution path. The harness therefore accepts a separately installed binary only when `mistralrs doctor` proves both CUDA compilation and detected CUDA hardware; otherwise it performs a native source build requiring:

- Rust 1.94 or newer;
- Visual Studio 2022 C++ Build Tools;
- an NVIDIA driver exposing `nvidia-smi`;
- the CUDA toolkit with `nvcc` on `PATH`.

Run:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\setup-mistralrs-local-agent.ps1
```

The setup script builds the current agent-capable Mistral.rs source and records the exact source commit, feature set, installed executable path, and executable SHA-256 hash. It tries, in order:

1. `cuda flash-attn cudnn`;
2. `cuda` only.

If both builds fail, the harness stops. It does not install or retain a non-CUDA binary as an acceptable executor.

Mistral.rs does not currently provide a native Windows Vulkan or WebGPU inference backend. The harness does not claim that such a path satisfies this experiment.

## Runtime CUDA proof

After `/v1/models` reports the selected model ready, the controller polls:

```powershell
nvidia-smi --query-compute-apps=pid,process_name,used_gpu_memory --format=csv,noheader,nounits
```

The exact server PID must appear. The observed row is written to:

```text
.local-agent/task-XX/cuda-runtime-evidence.json
```

The task controller does not invoke the implementer until that file has been created. This distinguishes a CUDA-capable installation from an actually CUDA-active model server.

## Task controls

```powershell
# Execute Task 1 with mandatory CUDA verification
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1

# Use the 8B model directly, still CUDA-gated
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -Model 'Qwen/Qwen3-8B'

# Reduce context if model startup is memory constrained
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -ContextLength 16384

# Retain the verified model server after the task
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -KeepServer

# Do not push, even after local gates pass
.\scripts\run-mistralrs-vwm-task.ps1 -Task 1 -SkipPush

# Run setup/worktree preflight without loading a model
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
- `cuda-runtime-evidence.json` containing the observed NVIDIA compute-process row;
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

- the installed Mistral.rs binary does not report the CUDA build feature;
- the NVIDIA driver or CUDA toolkit is not detected;
- the live server PID does not appear in the NVIDIA CUDA compute-process table;
- Mistral.rs cannot start either selected local model;
- the branch cannot be isolated safely;
- tracked or untracked user changes are present in the implementation worktree;
- the model fails three repair rounds;
- required tests cannot run because an external dependency or credential is absent;
- GitHub authentication is required to push and `-SkipPush` was not supplied.

## Prohibited behavior

- No non-CUDA model execution.
- No Docker, Podman, WSL, or Electron.
- No cloud inference path.
- No OpenCode invocation.
- No work on `main` or `master`.
- No force push.
- No merge.
- No `git reset --hard`.
- No `git clean`.
- No deletion of unrelated user files.
- No completion claim without fresh CUDA, reviewer, and verifier evidence.