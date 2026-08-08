# 3DMk Local Mistral.rs Agent Experiment Design

**Status:** Approved by operator on 2026-08-07  
**Scope:** Agent execution only  
**Product architecture:** Unchanged

## Decision

For this experiment, OpenCode is removed from the active execution path. The authoritative 3DMk implementation plan, branch discipline, Tauri 2 application architecture, Rust ownership rules, VWM boundaries, test requirements, and GitHub evidence requirements remain unchanged.

The replacement executor is a local Mistral.rs runtime using a locally cached model and Mistral.rs's server-side shell tool loop.

## Runtime architecture

```mermaid
flowchart LR
    PS[PowerShell controller] --> WT[Isolated git worktree]
    PS --> MR[Mistral.rs loopback server]
    MR --> MODEL[Local coding model]
    MR -->|shell tool| WT
    WT --> TESTS[Cargo and repository tests]
    WT --> GIT[Task commits]
    GIT --> GH[GitHub branch and draft PR]
    PS --> I[Implementer session]
    PS --> R[Independent reviewer session]
    PS --> V[Independent verifier session]
```

## Model policy

1. **Primary:** `Qwen/Qwen3-Coder-30B-A3B-Instruct`, 4-bit quantization.
   - Selected for agentic coding and tool calling.
   - The model has 30.5B total parameters and 3.3B activated parameters.
   - On the reference RTX 4050 6 GB system, Mistral.rs may place part of the quantized weights in system RAM.
2. **Fallback:** `Qwen/Qwen3-8B`, 4-bit quantization.
   - Used automatically when the primary model cannot start on the available hardware.
3. **Smoke-test override:** `Qwen/Qwen3-4B`, 4-bit quantization.
   - Suitable for validating the harness, not the preferred implementation model.
4. A local model directory may replace a Hugging Face model ID through a script parameter.

## Mistral.rs policy

- Bind only to `127.0.0.1`.
- Use the OpenAI-compatible Responses API and its `shell` tool.
- Use `pwsh.exe` as the shell on Windows.
- Set the shell working directory to the isolated 3DMk worktree.
- Run headlessly; the built-in web UI is disabled.
- Use bounded tool rounds and command timeouts.
- Keep the server alive for implementer, reviewer, and verifier sessions so the model is loaded once.
- Prefer a native Windows CUDA source build using `cuda flash-attn cudnn`.
- Retry with the `cuda` feature alone if optional CUDA integrations do not compile.
- Permit an explicitly reported CPU fallback for the experiment when the CUDA toolchain is absent.
- Do not use Docker, Podman, WSL, Electron, or a cloud model.

Mistral.rs does not currently provide a native Vulkan/WebGPU inference backend. The harness must not claim that such a fallback exists. CPU is the only automatic Mistral.rs fallback in this experiment.

## Execution topology

Each plan task uses three independent Mistral.rs sessions:

### Implementer

- Reads `AGENTS.md`, the progress ledger, and the authoritative plan.
- Executes only the selected dependency-ready task.
- Uses test-driven development.
- Updates the evidence ledger.
- Commits the task locally.
- Does not merge or force-push.

### Reviewer

- Reads the task requirements and complete task diff.
- Verifies specification compliance and code quality.
- Must not modify tracked repository files.
- Writes a structured `VERDICT: PASS` or `VERDICT: FAIL` report under the ignored local-agent workspace.

### Verifier

- Runs fresh commands from the task and governing repository contracts.
- Checks branch, cleanliness, tests, formatting, and task-specific evidence.
- Must not modify tracked repository files.
- Writes a structured `VERDICT: PASS` or `VERDICT: FAIL` report.

A failed review or verification result is returned to a fresh implementer repair session. The controller permits at most three repair rounds before reporting `BLOCKED`.

## Isolation and GitHub synchronization

- The controller resolves or creates the mandated branch `agent/vwm-authoritative-revision-implementation` in an isolated worktree.
- It refuses to run on `main` or `master`.
- It refuses to start with tracked, uncommitted changes.
- It records the base commit before implementation.
- A task is pushed only after reviewer and verifier verdicts pass.
- The controller creates or updates a draft PR through the authenticated GitHub CLI when available.
- It never merges the PR.

## Safety controls

The local model receives arbitrary shell access inside the dedicated worktree. Safety therefore comes from containment and evidence rather than from trusting the model:

- dedicated worktree;
- non-default implementation branch;
- loopback-only inference server;
- bounded tool rounds and timeouts;
- pre- and post-session git status checks;
- independent reviewer and verifier sessions;
- no direct merge;
- no `git reset --hard`, `git clean`, force push, or branch deletion;
- no secrets in prompts or repository files.

## Experiment success criteria

The executor experiment passes when all of the following are evidenced:

1. Mistral.rs reports the selected local model ready on the loopback endpoint.
2. The implementer reads the repository contracts and produces a task commit on the mandated branch.
3. The reviewer inspects the complete task diff and emits a structured verdict.
4. The verifier runs the required commands and emits a structured verdict.
5. The progress ledger contains commands and observed outcomes.
6. The branch is pushed and a draft PR exists.
7. No OpenCode credential or OpenCode runtime is required.

The experiment does not itself prove the broader 17-task plan complete.