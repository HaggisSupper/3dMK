---
name: 3dmk-dev
description: Governed development workflow for the 3DMK Windows-first Rust/Tauri 3D workstation
compatibility: Codex or OpenCode skill loader with repository file access
metadata:
  application: 3dMK
  authority: docs/CURRENT_STATE.md
  runtime: Rust-Axum-Tauri-Three.js
---

# 3DMK Development Toolkit

Use this skill for implementation, debugging, review, planning, or release work in the 3dMK repository.

## Trigger

Load when a request changes 3dMK code, contracts, processing behavior, GPU execution, persistence, packaging, UI behavior, or development governance.

## Authority boundary

Repository documents and measured test evidence are authoritative. This skill selects and constrains a workflow; it is not persistent memory, approval, mission state, or completion evidence.

Read `AGENTS.md`, `docs/CURRENT_STATE.md`, and the governing documents they name before consequential work. When a task conflicts with those documents, stop and report the conflict.

## Deterministic evidence

Before semantic inference, inspect:

- current branch, base commit, and working-tree status;
- relevant source, tests, contracts, and active progress ledgers;
- current Rust, Tauri, Axum, frontend, CUDA, and packaging configuration;
- exact reproduction input and observed failure;
- available Windows/CUDA toolchain and model/runtime evidence.

Never infer that a capability, test, dependency, accelerator, or contract exists without repository or runtime evidence.

## Workflow

1. Restate one bounded outcome.
2. Route to exactly one primary mode:
   - `setup`: establish repository authority and missing context;
   - `spec`: define a bounded contract or implementation plan;
   - `implement`: execute an approved bounded change;
   - `diagnose`: reproduce and isolate a defect or regression;
   - `review`: independently assess a diff or candidate;
   - `release`: verify packaging, deployment, rollback, and evidence.
3. Load only the references required for that mode.
4. Preserve the transaction boundary:
   `admit → stage → execute → validate → publish → finalize`.
5. Use deterministic tests before broad semantic claims.
6. Keep implementation, review, verification, and repair evidence distinct.
7. Record exact commands, exit codes, commits, limitations, and residual risks.
8. Stop at the task boundary; do not silently widen scope.

## Non-negotiable application constraints

- Windows 11 x64 and PowerShell 7 are the reference environment.
- Rust is authoritative for geometry, persistence, processing, contracts, and durable state.
- Tauri 2 is the deployment shell; Axum is the embedded/browser-development backend.
- CUDA is primary for eligible heavy compute; Vulkan/WebGPU is a capability-specific fallback only after verification.
- Mistral.rs is the local inference runtime; llama.cpp is fallback when explicitly supported.
- No Docker, Podman, WSL, Electron, cloud processing, or Python production backend.
- JavaScript is presentation, input, rendering, and bounded evidence capture—not authoritative geometry or durable state.
- Bulk geometry must not cross Tauri IPC.
- AI/VLM output is advisory and cannot publish geometry or accepted state.
- Failed, cancelled, timed-out, out-of-memory, device-loss, or stale-generation work must not publish partial authoritative state.

## Completion language

Use `PROJECT_COMPLETE` only when the repository’s current completion gate is freshly proven. Otherwise use the precise state: `NOT_STARTED`, `IN_PROGRESS`, `TASK_CANDIDATE`, `TASK_COMPLETE`, `SESSION_BOUNDARY`, or `BLOCKED`.

## Output artifact

Return a compact work record containing: bounded outcome, selected mode, authority inspected, evidence commands, files changed, tests and exit codes, commit/PR, residual risks, and stop condition.

## Stop condition

Stop when the bounded task is verified, when a required external capability is unavailable, or when an authority/architecture conflict cannot be resolved from repository evidence. Do not claim completion from documentation, scaffolding, static checks, or partial integration alone.

For the detailed authority map and verification matrix, load:

- `.codex/skills/3dmk-dev/references/authority-map.md`
- `.codex/skills/3dmk-dev/references/verification-matrix.md`
