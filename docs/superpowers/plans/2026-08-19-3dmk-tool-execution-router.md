# 3DMK Tool Execution Router Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Rust developer CLI that detects and launches the correct local runner for named 3DMK tooling profiles.

**Architecture:** A standard-library-only Rust binary owns profile lookup, executable discovery, structured probe output, and safe process spawning. A Git Bash validator provides a portable equivalent of the toolkit content validation; PowerShell remains the native Windows validation path.

**Tech Stack:** Rust 2021 standard library, Git Bash, PowerShell 7/Windows PowerShell, Cargo.

**Spec:** `docs/superpowers/specs/2026-08-19-3dmk-tool-execution-router.md`

## Global Constraints

- Developer infrastructure only; never ship in the Tauri application.
- Windows-first; Git Bash is valid for portable shell checks.
- Rust 2021 standard library only.
- No Docker, Podman, WSL, Electron, cloud processing, or Python production backend.
- Never execute shell-concatenated commands.
- Unknown or unavailable runners must return structured `BLOCKED` evidence.

---

### Task 1: Define profiles and probe contract

**Files:**
- Create: `tools/3dmk-exec-router/Cargo.toml`
- Create: `tools/3dmk-exec-router/src/main.rs`
- Test: inline Rust unit tests in `src/main.rs`

**Interfaces:**
- Produces: `Profile::from_name(&str) -> Option<Profile>`
- Produces: `probe_profile(Profile, &Path) -> ProbeResult`

- [ ] **Step 1: Write failing profile lookup tests**
- [ ] **Step 2: Run `cargo test` and confirm profile symbols are absent**
- [ ] **Step 3: Implement typed profiles, block reasons, and JSON result serialization**
- [ ] **Step 4: Run `cargo test` and confirm lookup and JSON tests pass**
- [ ] **Step 5: Commit the typed probe contract**

### Task 2: Add safe runner selection and execution

**Files:**
- Modify: `tools/3dmk-exec-router/src/main.rs`
- Test: inline Rust unit tests in `src/main.rs`

**Interfaces:**
- Consumes: `Profile`, `ProbeResult`
- Produces: `run_profile(Profile, &Path, &[String]) -> ExitCode`

- [ ] **Step 1: Write failing tests for runner preference and blocked profiles**
- [ ] **Step 2: Run `cargo test` and confirm the tests fail**
- [ ] **Step 3: Implement PATH discovery, Windows gating, and `Command` argument spawning**
- [ ] **Step 4: Run `cargo test` and confirm all tests pass**
- [ ] **Step 5: Commit safe execution routing**

### Task 3: Add Git Bash validation parity

**Files:**
- Create: `scripts/validate-3dmk-toolkit.sh`
- Test: `scripts/validate-3dmk-toolkit.sh`

**Interfaces:**
- Produces: exit `0` and `3DMK toolkit validation: PASS` when required files and terms are present.

- [ ] **Step 1: Write a failing fixture missing `## Stop condition`**
- [ ] **Step 2: Run the validator and confirm it exits non-zero**
- [ ] **Step 3: Implement required-file, heading, transaction-boundary, and placeholder checks**
- [ ] **Step 4: Run the validator against a complete fixture and confirm PASS**
- [ ] **Step 5: Commit Git Bash validation parity**

### Task 4: Connect the toolkit and documentation

**Files:**
- Modify: `.codex/skills/3dmk-dev/SKILL.md`
- Create: `.codex/skills/3dmk-dev/references/tool-execution-router.md`
- Modify: `.github/copilot-instructions.md`

**Interfaces:**
- Documents: `probe` before `run`; profile names; blocked behavior.

- [ ] **Step 1: Add a failing static assertion that the toolkit references the router**
- [ ] **Step 2: Confirm the assertion fails on the unmodified toolkit**
- [ ] **Step 3: Add router reference and concise usage contract**
- [ ] **Step 4: Run the Git Bash validator and router `probe toolkit-check`**
- [ ] **Step 5: Commit documentation integration**

### Task 5: Verify and publish

**Files:**
- Verify: all files above

- [ ] **Step 1: Run `cargo fmt --check`**
- [ ] **Step 2: Run `cargo test`**
- [ ] **Step 3: Run `bash scripts/validate-3dmk-toolkit.sh`**
- [ ] **Step 4: Run `cargo run -- probe toolkit-check` and record structured output**
- [ ] **Step 5: Open a draft PR with commands, results, limitations, and rollback notes**
