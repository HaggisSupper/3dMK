# 3DMk Local Mistral.rs Agent Experiment Implementation Plan

> **For agentic workers:** Execute this plan in order. It changes the executor harness only; it must not alter or reorder the authoritative 17-task product plan.

**Goal:** Replace the blocked OpenCode execution path with a Windows-native Mistral.rs local-model harness that implements, reviews, verifies, commits, and synchronizes one authoritative plan task at a time.

**Architecture:** A small PowerShell controller creates or reuses an isolated git worktree, starts one loopback-only Mistral.rs server, and delegates runtime services and agent roles to focused modules. It submits separate implementer, reviewer, and verifier Responses API sessions with the built-in shell tool, accepts only structured pass verdicts, performs bounded repair rounds, then pushes the existing implementation branch and creates or updates a draft PR.

**Tech Stack:** PowerShell 7, git worktrees, Mistral.rs CLI and Responses API, Qwen3-Coder local model, Rust/Cargo tests, GitHub CLI, GitHub Actions Windows validation.

## Global Constraints

- Windows 11 and PowerShell 7 are the reference environment.
- Rust is primary; Tauri 2 and the 3DMk product architecture remain unchanged.
- No Docker, Podman, WSL, Electron, cloud model, or OpenCode runtime.
- Mistral.rs binds only to `127.0.0.1`.
- Prefer native CUDA; CPU fallback must be explicit and reported as degraded.
- Work only on `agent/vwm-authoritative-revision-implementation` in an isolated worktree.
- Never force push, merge, delete branches, run `git reset --hard`, or run `git clean`.
- Push only after independent review and verification pass.
- Never claim task completion without fresh command evidence.

---

### Task 1: Add Mistral.rs setup and hardware validation

**Files:**
- Create: `scripts/setup-mistralrs-local-agent.ps1`
- Test: `scripts/test-mistralrs-local-agent-harness.ps1`

**Produces:** `mistralrs` on `PATH`, with CUDA required by default and an explicit CPU fallback path.

- [ ] Detect `git`, `cargo`, `rustc`, `nvidia-smi`, `nvcc`, and Visual Studio build tooling.
- [ ] Accept an existing CUDA source build only when a source-commit/binary-hash marker or reliable doctor output confirms it.
- [ ] Clone the current agent-capable source into `%LOCALAPPDATA%` and build `mistralrs-cli` with `cuda flash-attn cudnn`.
- [ ] Retry with `cuda` only when optional CUDA integrations fail.
- [ ] Use the official Windows installer only when `-AllowCpuFallback` is explicitly supplied.
- [ ] Fail with exact missing prerequisites rather than silently changing backend.

### Task 2: Add isolated local task controller

**Files:**
- Create: `scripts/run-mistralrs-vwm-task.ps1`
- Create: `scripts/mistralrs-agent/common.ps1`
- Create: `scripts/mistralrs-agent/roles.ps1`
- Modify: `.gitignore`

**Produces:** one-command task execution through a local model with bounded, auditable responsibilities.

- [ ] Resolve the repository root from the script location.
- [ ] Locate or create the mandated implementation worktree, moving a clean primary checkout back to `main` when necessary.
- [ ] Refuse default branches and any tracked or untracked implementation-worktree changes.
- [ ] Start Mistral.rs on loopback with `pwsh` shell access scoped to the worktree.
- [ ] Require the CUDA source-build marker unless `-AllowCpuFallback` is explicitly supplied.
- [ ] Try the primary model, then the configured fallback model.
- [ ] Submit independent implementer, reviewer, and verifier sessions with fresh session IDs.
- [ ] Verify that reviewer and verifier sessions cannot change HEAD or tracked files.
- [ ] Require `VERDICT: PASS` reports and allow at most three repair rounds.
- [ ] Push only after both gates pass; create or update a draft PR through `gh`.
- [ ] Preserve all transient evidence beneath ignored `.local-agent/`.

### Task 3: Make the repository agent contract executor-neutral

**Files:**
- Modify: `AGENTS.md`
- Create: `docs/agent-execution/MISTRALRS_LOCAL_AGENT.md`

**Produces:** one authoritative contract readable by Mistral.rs or another future local executor.

- [ ] Remove OpenCode-specific naming from the governing contract.
- [ ] Record Mistral.rs as the active experiment backend without deleting retained OpenCode files.
- [ ] Preserve all product, branch, evidence, and completion gates.
- [ ] Document exact setup, model, fallback, worktree, task, and evidence behavior.

### Task 4: Add deterministic harness checks

**Files:**
- Create: `scripts/test-mistralrs-local-agent-harness.ps1`
- Create: `.github/workflows/mistralrs-harness-validation.yml`

**Produces:** a non-model Windows preflight and CI validation gate.

- [ ] Parse every PowerShell file with the PowerShell AST parser.
- [ ] Verify required files and contract strings exist.
- [ ] Verify loopback binding, worktree isolation, local model defaults, independent role prompts, bounded repairs, and draft-PR behavior.
- [ ] Reject direct OpenCode, cloud fallback, Docker, Podman, WSL, force-push, merge, hard-reset, or clean commands.
- [ ] Run the validation on `windows-latest` for branch pushes and pull requests.

### Task 5: Synchronize the experiment with GitHub

**Files:**
- Update: issue `#6`
- Create or update: draft PR from `agent/vwm-authoritative-revision-implementation` to `main`

**Produces:** visible mission state and a reviewable branch.

- [ ] Record that the OpenCode secret is no longer a blocker.
- [ ] State that local Windows/GPU execution remains required to run authoritative product Task 1.
- [ ] Link the Mistral.rs runbook and exact command.
- [ ] Inspect the Windows static validation result and repair every failure.
- [ ] Do not mark authoritative product Task 1 complete until local implementer, reviewer, and verifier evidence exists.
