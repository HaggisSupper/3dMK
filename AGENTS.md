# 3DMk Agent Contract

3DMK is a child repository of the Veritas capability framework. Veritas guardrails, capability specifications, validation and release expectations, and immutable concrete-contract culture are mandatory. Stricter 3DMK invariants remain in force; any divergence must be explicit, justified, versioned, documented, and machine-detectable where practical.

## Mission

Deliver a production-grade 3DMk system in which:

- Rust project, asset, revision, operation, analysis, job, measurement, persistence, and export state is authoritative;
- the complete supported system is CUDA-first and requires supported NVIDIA hardware;
- Mistral.rs is local, headless, CUDA-backed, and advisory;
- every mutation is reviewable, recoverable, provenance-complete, and verified before publication;
- the Tauri application works offline after installation for deterministic core workflows;
- optional product-runtime intelligence escalation may use explicitly approved local or cloud providers only under the Veritas policy, privacy, resource, and authority contracts defined by ADR-004;
- completion is based on fresh evidence, not scaffolding, routes, prose, or agent assertions.

## Governing documents

Read these in order before implementation, review, or verification:

1. `contracts/veritas-child-conformance.v1.json`
2. `docs/CURRENT_STATE.md`
2. `docs/superpowers/specs/2026-08-07-3dmk-world-class-quality-standard.md`
3. `docs/superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md`
4. `docs/architecture/decisions/`
5. `docs/architecture/CAPABILITY_FRAMEWORK_AGENT_GOVERNANCE.md`
6. `docs/architecture/CAPABILITY_FRAMEWORK_MASTER_SPEC.md`
7. `docs/architecture/IMPLEMENTATION_GUARDRAILS.md`
8. `docs/architecture/NON_NEGOTIABLE_ACCEPTANCE_GATES.md`
9. `docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`
10. `docs/superpowers/plans/2026-08-07-3dmk-foundation-batch.md`
11. `docs/superpowers/plans/2026-08-07-3dmk-world-class-program.md`
12. `docs/agent-execution/VWM_PROGRESS.md`
13. `docs/agent-execution/CUDA_FOUNDATION_PROGRESS.md`

When an older implementation detail conflicts with a governing document, the higher document in this list controls. Do not re-plan the product into a smaller interpretation.

Explicitly approved, rejected, superseded, deferred, hypothesis, and riff dispositions SHALL be reflected in governing repository documentation according to `docs/architecture/CAPABILITY_FRAMEWORK_AGENT_GOVERNANCE.md`. Conversation state SHALL NOT remain stronger than repository authority.

## Active executor

The active autonomous executor is the local Mistral.rs harness documented in:

`docs/agent-execution/MISTRALRS_LOCAL_AGENT.md`

OpenCode is removed from the active repository workflow and must not be invoked. No OpenCode API key, action, agent, command, configuration, or runbook is part of the current execution path.

Before model-driven work begins, the controller must prove:

- the implementation worktree is clean and linked;
- current `origin/main` is an ancestor of the active implementation branch;
- a branch that is strictly behind `origin/main` is safely fast-forwarded before model startup;
- a branch that has diverged from `origin/main` is blocked for reviewed reconciliation;
- all selected task predecessors are checked complete;
- the Mistral.rs binary was built with CUDA;
- the NVIDIA host and driver are usable;
- the exact Mistral.rs process is observed as a CUDA compute process;
- the model profile fits the approved VRAM budget.

CPU LLM inference and cloud inference fallback are prohibited for the active autonomous development executor.

This development-executor prohibition SHALL NOT be interpreted as a product-runtime prohibition. Product-runtime intelligence escalation, when separately implemented and validated under ADR-004, MAY escalate from deterministic context to optional embedded ML, optional local LLM/VLM/multimodal inference, and then an approved OpenAI-compatible cloud endpoint. Such escalation SHALL remain optional, policy-controlled, data-minimized, fail-closed, and non-authoritative; deterministic 3DMk core workflows SHALL remain usable without cloud access.

## Platform and stack

- Windows 11 x64 and PowerShell 7 are the reference environment.
- Rust 2021 or newer is the production language.
- Tauri 2 is the desktop shell.
- Axum is the single browser-development and Tauri backend.
- The current Three.js UI is modularized incrementally; no framework rewrite is authorized in the revision-authority lane.
- A supported NVIDIA CUDA-capable GPU with at least 6 GiB usable VRAM is a minimum operational requirement.
- CUDA is primary for eligible heavy geometry, point-cloud, reconstruction, image, perception, inference, and future simulation work.
- WebGPU/Vulkan may be used only as a verified capability-specific fallback; it does not satisfy system CUDA compliance.
- CPU owns control, I/O, transactions, serialization, audit, and deterministic reference implementations.
- No Docker, Podman, WSL, Electron, or Python production backend.
- No Apple-specific development chain.

## Authority and integrity rules

- Original imported bytes and the root measured revision are immutable.
- Rust owns authoritative geometry and all durable state.
- JavaScript owns rendering, input, bounded evidence capture, and review presentation only.
- `loadedGeometryStore` and viewport objects are render caches, never documents of record.
- Every geometry mutation produces a traceable candidate child revision.
- Analysis remains separate from geometry mutation.
- Exact point, vertex, face, primitive, or instance membership replaces bounding-box deletion.
- Same-topology operations preserve stable source IDs and attributes exactly.
- Topology-changing operations emit source mappings, attribute-transfer reports, and quality evidence.
- No operation may silently discard normals, colors, UVs, materials, textures, source IDs, calibration, measurements, accelerator evidence, or history.
- AI/VLM output is advisory and cannot directly publish geometry or accepted state.
- Bulk geometry does not cross Tauri IPC.

## Transaction and failure rules

Every operation follows:

```text
admit → stage → execute → validate → publish → finalize
```

Before `publish`, no output is authoritative.

Cancellation, timeout, worker death, CUDA error, device loss, out-of-memory, validation failure, disk-full, stale project generation, or publication failure must produce no partial revision and no dangling authoritative asset reference.

Never hold a project-store lock while waiting for a GPU lease or kernel completion. Stage outputs outside the authoritative namespace and publish only through the accepted transaction boundary.

## Dependency sequence

```text
Authoritative Tasks 1–5
        ↓
CUDA Foundation FB1–FB8
        ↓
Authoritative Tasks 6–14
        ↓
Tasks 15–16
        ↓
Task 17 and world-class release closure
```

Do not begin authoritative Task 6 until every foundation task is independently reviewed, verified, recorded, and checked complete.

## Per-task execution method

For every task:

1. Read the current-state document, governing standards, current plan section, and both progress ledgers.
2. Inspect the current `origin/main`, active branch, base commit, relevant source, tests, and existing contracts.
3. Require `origin/main` ancestry or perform only a safe fast-forward; block on divergence.
4. Restate concrete acceptance conditions in the appropriate ledger.
5. Write the smallest failure-reproducing or contract test.
6. Run it and prove that it fails for the intended reason.
7. Implement the smallest coherent production change.
8. Run focused tests, affected regression tests, and applicable CPU/CUDA differential tests.
9. Run an independent reviewer session against requirements and the complete diff.
10. Run an independent verifier session using fresh commands.
11. Repair every blocking or important finding and repeat both gates.
12. Record exact commands, exit codes, decisive output, files, commit, accelerator evidence, residual risks, and limitations.
13. Commit and push only after both independent gates pass.
14. Stop at the task boundary; do not combine unrelated work.

The implementer, reviewer, verifier, and repair roles use separate model sessions. Reviewer and verifier sessions are read-only with respect to tracked files and HEAD.

## Git discipline

- The active product implementation branch is `agent/vwm-authoritative-revision-implementation`.
- Before model startup, `origin/main` must be an ancestor of the active implementation branch.
- If the active branch is strictly behind main and has no unique commits, the controller may fast-forward it with `pull --ff-only`.
- If main and the active branch have both advanced, report `BLOCKED`; reconcile through reviewed Git history rather than rebasing, force-pushing, or silently merging in the model harness.
- Never implement on `main` or `master`.
- Use a focused `agent/<description>` branch for separately scoped governance, documentation, or support work.
- Use an isolated linked worktree for model-driven implementation.
- Never force-push.
- Never run `git reset --hard` or `git clean`.
- Preserve unrelated user changes.
- Commit coherent, independently reviewable tasks.
- Open or update a pull request; do not merge from the local model harness.
- Delete stale documentation only when an authoritative replacement exists or Git history is the appropriate archive.

## Autonomy and questions

Proceed autonomously using repository evidence, tests, accepted designs, official primary documentation, and measured behavior.

Ask the operator only when:

- a credential or licensed model asset is required;
- an irreversible product decision is absent from the governing documents;
- three evidence-based repair attempts expose a genuine architecture conflict;
- local-only files differ materially from GitHub and the authoritative source cannot be inferred.

Ordinary implementation, test, refactor, kernel, model-profile, memory-budget, documentation, branch, and deletion decisions already governed here do not require approval.

## State vocabulary

- `NOT_STARTED` — no task evidence.
- `IN_PROGRESS` — bounded work has begun but no candidate commit has passed both gates.
- `TASK_CANDIDATE` — a local task commit exists; one or more gates or synchronization steps remain.
- `TASK_COMPLETE` — one task is reviewed, verified, recorded, and pushed.
- `BATCH_COMPLETE` — a defined dependency batch is complete.
- `SESSION_BOUNDARY` — work is safely resumable but the program remains incomplete.
- `BLOCKED` — external evidence proves a credential, licensed asset, supported CUDA environment, missing toolchain, stale/diverged mainline, or unresolved architecture decision is required.
- `PROJECT_COMPLETE` — every authoritative, foundation, reliability, release, and evidence gate has passed freshly.

Do not say “done,” “complete,” “finished,” or give a project percentage unless the state is `PROJECT_COMPLETE`.

## Project completion gate

`PROJECT_COMPLETE` requires, at minimum:

- all 17 authoritative tasks and FB1–FB8 checked complete;
- full root and VWM formatting, compilation, strict lint, unit, integration, and regression gates;
- verified transactional persistence, crash recovery, package round-trip identity, and explicit export truth;
- Tauri clean-machine offline installation and launch on the reference NVIDIA system;
- CUDA-enabled Mistral.rs and product CUDA known-answer evidence;
- a functioning GPU resource broker and safety reserve;
- CUDA-primary representative geometry, reconstruction, vision, and inference operations or explicit approved exceptions;
- CPU-reference parity, deterministic ordering, memory, concurrency, cancellation, OOM, device-loss, and worker-failure gates;
- the real vertical slice: import → analyze → exact candidate → compare → accept → close → reopen → save → re-import → explicit GLB and PLY export;
- no viewport helper or transient browser state in authoritative exports by default;
- complete fault matrix, performance results, twelve-hour mixed-workload soak, upgrade/rollback, support bundle, screenshots, hashes, limitations, and recovery evidence.

Evidence before assertion. No exceptions.
