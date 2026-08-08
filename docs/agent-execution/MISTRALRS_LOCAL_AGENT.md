# 3DMk Local Mistral.rs Execution Runbook

## Purpose

This is the active autonomous development executor for 3DMk. It runs a local coding model through Mistral.rs against an isolated Git worktree and enforces dependency admission, separate implementation/review/verification sessions, bounded repairs, and gated publication.

It is not the product implementation itself. Product runtime integration of Mistral.rs, the GPU broker, and the supervised CUDA worker remains tracked in `../CURRENT_STATE.md` and the CUDA-foundation plan.

## Mandatory environment

- Windows 11 x64;
- PowerShell 7;
- Git and an authenticated GitHub remote;
- the Rust toolchain required by current Mistral.rs source;
- Visual Studio C++ Build Tools;
- a supported NVIDIA driver and CUDA toolkit with `nvcc` on `PATH`;
- a supported NVIDIA GPU;
- sufficient RAM, VRAM, and NVMe capacity for the selected model and task.

CPU inference and cloud inference fallback are prohibited.

## Governing branch

Model-driven product and foundation edits use an isolated linked worktree on:

```text
agent/vwm-authoritative-revision-implementation
```

The controller refuses `main` or `master`, an unclean tracked worktree, a non-linked worktree, unmet task dependencies, force pushes, destructive resets, and repository cleaning.

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

## Task selection

### Authoritative product task

```powershell
pwsh -NoLogo -NoProfile `
  -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -Task 1
```

Valid product tasks are `1` through `17`.

### CUDA foundation task

```powershell
pwsh -NoLogo -NoProfile `
  -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -FoundationTask FB1
```

Valid foundation tasks are `FB1` through `FB8`.

The controller selects the corresponding plan, ledger, task heading, report directory, session IDs, evidence file, and pull-request metadata. It does not treat foundation work as an unnumbered side activity.

## Dependency admission

Dependency checks execute before Mistral.rs setup or model loading:

- authoritative Task 1 has no predecessor;
- authoritative Task N requires every authoritative Task 1 through N−1 checked complete;
- FB1 requires authoritative Tasks 1–5 checked complete;
- FBN requires authoritative Tasks 1–5 and every previous foundation task checked complete;
- authoritative Task 6 and every later product task require FB1–FB8 checked complete in addition to their authoritative predecessors.

A missing or unchecked ledger entry returns `BLOCKED`. The controller does not start CUDA/Mistral.rs to work around an incomplete dependency.

## Model profiles

Current defaults:

```text
Primary:  Qwen/Qwen3-Coder-30B-A3B-Instruct, 4-bit
Fallback: Qwen/Qwen3-8B, 4-bit
Smoke:    Qwen/Qwen3-4B, 4-bit
Context:  32,768 tokens by default
```

A smaller model or shorter context may be selected to fit the CUDA budget. The fallback changes model size, not accelerator policy: every model remains CUDA-backed.

Example:

```powershell
pwsh -NoLogo -NoProfile `
  -File .\scripts\run-mistralrs-vwm-task.ps1 `
  -FoundationTask FB1 `
  -Model 'Qwen/Qwen3-8B' `
  -FallbackModel 'Qwen/Qwen3-4B' `
  -ContextLength 16384
```

## CUDA gates

After dependency admission and before the implementer session:

1. the installed executable must match the recorded source-build hash;
2. Mistral.rs must report a CUDA-capable build;
3. the NVIDIA host must be visible and usable;
4. the selected model endpoint must become ready;
5. the exact Mistral.rs process ID must appear in NVIDIA's active CUDA compute-process inventory.

Runtime evidence is written to:

```text
.local-agent/task-XX/cuda-runtime-evidence.json
.local-agent/foundation-fbX/cuda-runtime-evidence.json
```

Missing evidence is a hard blocker and prevents publication.

## Role topology

### Implementer

- reads `AGENTS.md`, `docs/CURRENT_STATE.md`, both progress ledgers, the selected plan, and the selected task section;
- uses test-driven development;
- executes shell commands rather than describing them;
- updates the selected task ledger and current-state document when truth changes;
- commits a bounded candidate locally;
- does not push.

### Reviewer

- uses a separate model session;
- reads the selected task requirements, complete diff, role reports, both ledgers, and CUDA runtime evidence;
- verifies specification, architecture, safety, tests, documentation truth, and scope;
- writes `VERDICT: PASS` or `VERDICT: FAIL`;
- cannot modify tracked files or HEAD.

### Verifier

- uses another separate model session;
- reruns fresh selected-task and regression commands;
- verifies predecessor ledgers and CUDA evidence;
- records exact exit codes and decisive output;
- cannot modify tracked files or HEAD.

### Repair

A failed review or verification report is passed to a fresh implementer repair session. The controller permits at most three repair rounds before reporting `BLOCKED`.

## Evidence locations

Transient evidence:

```text
.local-agent/task-XX/
.local-agent/foundation-fbX/
```

Durable evidence:

```text
docs/agent-execution/VWM_PROGRESS.md
docs/agent-execution/CUDA_FOUNDATION_PROGRESS.md
```

The selected task ledger receives the durable command, exit-code, commit, accelerator, review, verification, risk, and limitation evidence. Transient reports remain ignored by Git.

## Publication

After independent review and verification pass, the controller:

- confirms the branch and tracked worktree are clean;
- confirms CUDA runtime evidence exists;
- pushes the active implementation branch;
- creates or finds the draft pull request through authenticated GitHub CLI;
- names the selected authoritative or foundation task in PR metadata;
- never merges the pull request.

## Stop conditions

The controller fails closed when:

- a predecessor ledger entry is incomplete;
- CUDA cannot be proven;
- neither approved model profile can start on CUDA;
- the branch cannot be isolated safely;
- user changes are present;
- required tests or native toolchains are unavailable;
- a credential or licensed model asset is required;
- reviewer or verifier does not pass after three repair rounds;
- GitHub publication is required but authentication is unavailable.

## Security

- bind Mistral.rs only to `127.0.0.1`;
- do not place secrets, credentials, personal data, proprietary third-party content, or unredacted support material in prompts;
- do not grant the model access outside the dedicated worktree;
- do not weaken repository or product requirements to accommodate model limitations;
- do not treat a model response as verification evidence.
