# OpenCode VWM Execution Harness Implementation Plan

> **For agentic workers:** Execute this plan in order. Each task is independently reviewable and must finish with verification evidence.

**Goal:** Add a repository-native OpenCode harness that runs the authoritative VWM implementation plan with Big Pickle and evidence-gated completion.

**Architecture:** OpenCode receives project rules through `AGENTS.md`, project configuration through `opencode.json`, specialized agents and commands through committed `.opencode` files, and resumable state through `docs/agent-execution/VWM_PROGRESS.md`. A PowerShell launcher verifies the OpenCode installation, resolves Big Pickle from the live model catalog, prepares a non-main implementation branch, and starts or resumes the executor.

**Tech Stack:** OpenCode CLI, OpenCode Zen Big Pickle, Git, PowerShell 7, Markdown agent definitions, JSON configuration.

## Global Constraints

- Do not hard-code an unverified model catalog identifier without runtime validation.
- Do not modify `main` or `master` directly.
- Do not allow force push, destructive reset, repository cleaning, Docker, Podman, or WSL.
- Do not expose `.env`, key, certificate, or credential files to the model.
- Do not permit task completion without fresh test evidence.
- The authoritative VWM plan remains the scope source of truth.

---

### Task 1: Commit the design and repository rules

**Files:**
- Create: `docs/superpowers/specs/2026-07-31-opencode-vwm-execution-harness-design.md`
- Create: `AGENTS.md`

**Verification:**
- Confirm both files reference the authoritative VWM plan.
- Confirm `AGENTS.md` defines task, batch, session-boundary, blocker, and project-complete states.
- Confirm the file forbids premature completion and scope reduction.

### Task 2: Add OpenCode configuration and specialized agents

**Files:**
- Create: `opencode.json`
- Create: `.opencode/agents/vwm-executor.md`
- Create: `.opencode/agents/vwm-reviewer.md`
- Create: `.opencode/agents/vwm-verifier.md`
- Modify: `.gitignore`

**Verification:**
- Parse `opencode.json` as JSON.
- Confirm `default_agent` is `vwm-executor`.
- Confirm instructions include the authoritative plan and progress ledger.
- Confirm reviewer and verifier deny edits.
- Confirm force push, direct main/master push, destructive reset, Docker, Podman, WSL, and secret reads are denied.

### Task 3: Add the resumable command and progress ledger

**Files:**
- Create: `.opencode/commands/implement-vwm.md`
- Create: `docs/agent-execution/VWM_PROGRESS.md`
- Create: `docs/agent-execution/README.md`

**Verification:**
- Confirm the command resumes from the first incomplete task.
- Confirm all 17 authoritative plan tasks appear in the ledger.
- Confirm every task requires review, verification, evidence, and a commit.
- Confirm a context boundary is represented as `SESSION_BOUNDARY`, not completion.

### Task 4: Add the Windows launcher and preflight test

**Files:**
- Create: `scripts/start-opencode-vwm.ps1`
- Create: `scripts/test-opencode-vwm-harness.ps1`

**Verification:**
- The test script validates required files and JSON.
- The launcher checks `opencode`, `git`, `cargo`, `rustc`, plan presence, and repository status.
- The launcher runs `opencode models opencode --refresh` and extracts the live Big Pickle ID.
- The launcher creates or reuses `agent/vwm-authoritative-revision-implementation`.
- `Start`, `Continue`, and `Run` modes are supported.
- The launcher prints the Big Pickle data-use warning.

### Task 5: Verify and publish the harness

**Verification:**
- Run repository static harness tests.
- Inspect the branch diff for unrelated changes.
- Update draft PR #2 to describe the OpenCode harness.
- Do not claim OpenCode itself ran until the launcher has been executed on the Windows laptop.
