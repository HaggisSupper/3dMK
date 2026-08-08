# 3DMk Local Mistral.rs Execution Runbook

## Purpose

This is the active autonomous development executor for 3DMk. It runs a local coding model through Mistral.rs against an isolated Git worktree and enforces separate implementation, review, verification, repair, and publication gates.

It is not the product implementation itself. Product runtime integration of Mistral.rs, the shared GPU broker, and the product CUDA worker remain planned work recorded in `../CURRENT_STATE.md`.

## Mandatory environment

- Windows 11 x64;
- PowerShell 7;
- Git and an authenticated GitHub remote;
- the Rust toolchain required by the current Mistral.rs source;
- Visual Studio C++ Build Tools;
- a supported NVIDIA driver and CUDA toolkit with `nvcc` on `PATH`;
- a supported NVIDIA GPU;
- sufficient RAM, VRAM, and NVMe capacity for the selected local model and repository workload.

CPU inference and cloud inference fallback are prohibited.

## Governing branch

Model-driven edits use an isolated linked worktree on:

```text
agent/vwm-authoritative-revision-implementation
```

The controller refuses `main` or `master`, uncommitted tracked changes, a non-linked worktree, force pushes, destructive resets, and repository cleaning.

## Setup

From the repository root:

```powershell
pwsh -NoLogo -NoProfile `
  -File .\scripts\setup-mistralrs-local-agent.ps1
```

The setup path:

1. checks Git, Cargo, Rust, `nvidia-smi`, `nvcc`, and Visual Studio Build Tools;
2. accepts only a CUDA-confirmed Mistral.rs source build;
3. first attempts `cuda flash-attn cudnn`;
4. retries with the mandatory `cuda` feature when optional integrations fail;
5. records the source commit, feature set, executable path, and executable SHA-256;
6. rejects any CPU-only installation.

## Run one authoritative task

```powershell
pwsh -NoLogo -NoProfile `
  -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1
```

The controller starts Mistral.rs on `127.0.0.1`, disables its web UI, scopes the PowerShell shell tool to the isolated worktree, and keeps one model server alive while using fresh model sessions for each role.

## Model profiles

The repository currently exposes these default profiles:

```text
Primary:  Qwen/Qwen3-Coder-30B-A3B-Instruct, 4-bit
Fallback: Qwen/Qwen3-8B, 4-bit
Smoke:    Qwen/Qwen3-4B, 4-bit
Context:  32,768 tokens by default
```

A smaller model or shorter context may be selected to fit the CUDA budget. The fallback changes model size, not accelerator policy: every model must remain CUDA-backed.

Example:

```powershell
pwsh -NoLogo -NoProfile `
  -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1 `
  -Model 'Qwen/Qwen3-8B' `
  -FallbackModel 'Qwen/Qwen3-4B' `
  -ContextLength 16384
```

## CUDA gates

Before the implementer session starts, all of these must pass:

1. the installed executable matches the recorded source-build hash;
2. Mistral.rs reports a CUDA-capable build;
3. the NVIDIA host is visible and usable;
4. the model endpoint becomes ready;
5. the exact Mistral.rs process ID appears in NVIDIA's active CUDA compute-process inventory.

Runtime evidence is written beneath:

```text
.local-agent/task-XX/cuda-runtime-evidence.json
```

Missing evidence is a hard blocker and prevents publication.

## Role topology

### Implementer

- reads `AGENTS.md`, `docs/CURRENT_STATE.md`, both progress ledgers, the governing standards, and only the current task's referenced source;
- uses test-driven development;
- executes shell commands rather than describing them;
- updates the evidence ledger;
- commits a bounded candidate locally;
- does not push.

### Reviewer

- uses a separate model session;
- reads the complete requirements and diff;
- verifies specification compliance and code/test quality;
- writes `VERDICT: PASS` or `VERDICT: FAIL`;
- may run tests but cannot modify tracked files or HEAD.

### Verifier

- uses another separate session;
- reruns fresh task and regression commands;
- records exit codes and decisive output;
- checks CUDA evidence where applicable;
- cannot modify tracked files or HEAD.

### Repair

A failed review or verification report is passed to a fresh implementer repair session. The controller permits at most three repair rounds before reporting `BLOCKED`.

## Evidence locations

Transient evidence:

```text
.local-agent/task-XX/
```

Durable evidence:

```text
docs/agent-execution/VWM_PROGRESS.md
docs/agent-execution/CUDA_FOUNDATION_PROGRESS.md
```

Typical transient records include:

- Mistral.rs stdout/stderr;
- exact request/response payloads;
- implementer and repair reports;
- reviewer and verifier reports;
- GitHub authentication and PR metadata;
- controller summary;
- CUDA process evidence.

The transient directory is ignored by Git. Required evidence is summarized into the appropriate durable ledger before a task can pass.

## Publication

After independent review and verification pass, the controller:

- confirms the branch and tracked worktree are clean;
- confirms the CUDA evidence exists;
- pushes the implementation branch;
- creates or finds the draft pull request through the authenticated GitHub CLI;
- never merges the pull request.

## Stop conditions

The controller fails closed when:

- CUDA cannot be proven;
- neither approved model profile can start on CUDA;
- the branch cannot be isolated safely;
- user changes are present;
- required tests or native toolchains are unavailable;
- a credential or licensed model asset is required;
- reviewer or verifier does not pass after three repair rounds;
- GitHub publication is required but authentication is unavailable.

## Security

- bind only to loopback;
- do not put secrets, credentials, personal data, proprietary third-party content, or unredacted support material in prompts;
- do not grant the model access outside the dedicated worktree;
- do not weaken repository or product requirements to accommodate model limitations;
- do not treat a model response as verification evidence.
