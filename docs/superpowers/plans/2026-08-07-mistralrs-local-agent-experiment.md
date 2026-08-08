# 3DMk Local Mistral.rs Agent Experiment Implementation Plan

> **For agentic workers:** Execute this plan in order. It changes the executor harness only; it must not alter or reorder the authoritative 17-task product plan.

**Goal:** Replace the blocked OpenCode execution path with a Windows-native, CUDA-backed Mistral.rs local-model harness that implements, reviews, verifies, commits, and synchronizes one authoritative plan task at a time.

**Architecture:** A small PowerShell controller creates or reuses an isolated git worktree, validates a CUDA-enabled Mistral.rs binary and NVIDIA host, starts one loopback-only model server, verifies the live server PID through `nvidia-smi`, and delegates runtime services and agent roles to focused modules. It submits separate implementer, reviewer, and verifier Responses API sessions with the built-in shell tool, accepts only structured pass verdicts, performs bounded repair rounds, then pushes the existing implementation branch and creates or updates a draft PR.

**Tech Stack:** PowerShell 7, git worktrees, CUDA-enabled Mistral.rs CLI and Responses API, NVIDIA CUDA tooling, Qwen3-Coder local model, Rust/Cargo tests, GitHub CLI, GitHub Actions Windows validation.

## Global Constraints

- Windows 11 and PowerShell 7 are the reference environment.
- Rust is primary; Tauri 2 and the 3DMk product architecture remain unchanged.
- No Docker, Podman, WSL, Electron, cloud model, or OpenCode runtime.
- Mistral.rs binds only to `127.0.0.1`.
- **CUDA is mandatory. No CPU fallback is permitted.**
- The controller must obtain compiled-feature, host-detection, and live CUDA process evidence before invoking the model as an agent.
- Work only on `agent/vwm-authoritative-revision-implementation` in an isolated worktree.
- Never force push, merge, delete branches, run `git reset --hard`, or run `git clean`.
- Push only after CUDA runtime, independent review, and independent verification pass.
- Never claim task completion without fresh command evidence.

---

### Task 1: Add Mistral.rs setup and hardware validation

**Files:**
- Create: `scripts/setup-mistralrs-local-agent.ps1`
- Test: `scripts/test-mistralrs-local-agent-harness.ps1`

**Produces:** `mistralrs` on `PATH`, accepted only when CUDA compilation and NVIDIA host detection are proven.

- [ ] Detect `git`, `cargo`, `rustc`, `nvidia-smi`, `nvcc`, and Visual Studio build tooling.
- [ ] Accept an existing Mistral.rs binary only when `mistralrs doctor` reports the `cuda` build feature and a detected CUDA toolkit/driver.
- [ ] Verify a source-commit/binary-hash marker when the binary was built by this harness.
- [ ] Clone the current agent-capable source into `%LOCALAPPDATA%` and build `mistralrs-cli` with `cuda flash-attn cudnn`.
- [ ] Retry with `cuda` only when optional CUDA integrations fail.
- [ ] Stop if both CUDA builds fail; do not install a non-CUDA executor.
- [ ] Fail with exact missing prerequisites rather than silently changing backend.

### Task 2: Add isolated local task controller

**Files:**
- Create: `scripts/run-mistralrs-vwm-task.ps1`
- Create: `scripts/mistralrs-agent/common.ps1`
- Create: `scripts/mistralrs-agent/roles.ps1`
- Modify: `.gitignore`

**Produces:** one-command task execution through a local model with bounded, auditable responsibilities and live CUDA process evidence.

- [ ] Resolve the repository root from the script location.
- [ ] Locate or create the mandated implementation worktree, moving a clean primary checkout back to `main` when necessary.
- [ ] Refuse default branches and any tracked or untracked implementation-worktree changes.
- [ ] Start Mistral.rs on loopback with `pwsh` shell access scoped to the worktree.
- [ ] Re-run the mandatory CUDA diagnostic gate before server startup.
- [ ] Try the primary model, then the configured smaller model while preserving the CUDA requirement.
- [ ] Wait for `/v1/models`, then require the exact server PID in `nvidia-smi --query-compute-apps` output.
- [ ] Persist live CUDA process evidence to `.local-agent/task-XX/cuda-runtime-evidence.json`.
- [ ] Submit independent implementer, reviewer, and verifier sessions with fresh session IDs only after the CUDA runtime gate passes.
- [ ] Verify that reviewer and verifier sessions cannot change HEAD or tracked files.
- [ ] Require `VERDICT: PASS` reports and allow at most three repair rounds.
- [ ] Push only after CUDA, reviewer, and verifier gates pass; create or update a draft PR through `gh`.
- [ ] Preserve all transient evidence beneath ignored `.local-agent/`.

### Task 3: Make the repository agent contract executor-neutral

**Files:**
- Modify: `AGENTS.md`
- Create: `docs/agent-execution/MISTRALRS_LOCAL_AGENT.md`

**Produces:** one authoritative contract readable by Mistral.rs or another future local executor.

- [ ] Remove OpenCode-specific naming from the governing contract.
- [ ] Record Mistral.rs as the active experiment backend without deleting retained OpenCode files.
- [ ] Preserve all product, branch, evidence, and completion gates.
- [ ] Declare CUDA acceleration and live CUDA process evidence as non-negotiable executor invariants.
- [ ] Document exact setup, model, worktree, task, evidence, and stop behavior.

### Task 4: Add deterministic harness checks

**Files:**
- Create: `scripts/test-mistralrs-local-agent-harness.ps1`
- Create: `.github/workflows/mistralrs-harness-validation.yml`

**Produces:** a non-model Windows preflight and CI validation gate.

- [ ] Parse every PowerShell file with the PowerShell AST parser.
- [ ] Verify required files and contract strings exist.
- [ ] Verify loopback binding, worktree isolation, local model defaults, independent role prompts, bounded repairs, and draft-PR behavior.
- [ ] Require CUDA setup, doctor validation, live PID verification, and `cuda-runtime-evidence.json` contracts.
- [ ] Reject non-CUDA execution paths, direct OpenCode, cloud fallback, Docker, Podman, WSL, force-push, merge, hard-reset, or clean commands.
- [ ] Run the validation on `windows-latest` for branch pushes and pull requests.

### Task 5: Synchronize the experiment with GitHub

**Files:**
- Update: issue `#6`
- Create or update: draft PR from `agent/vwm-authoritative-revision-implementation` to `main`

**Produces:** visible mission state and a reviewable branch.

- [ ] Record that the OpenCode secret is no longer a blocker.
- [ ] State that local Windows/NVIDIA execution remains required to run authoritative product Task 1.
- [ ] Link the Mistral.rs runbook and exact command.
- [ ] Inspect the Windows static validation result and repair every failure.
- [ ] Record that static CI validates the contract but cannot provide live CUDA process evidence on the repository runner.
- [ ] Do not mark authoritative product Task 1 complete until local CUDA, implementer, reviewer, and verifier evidence exists.