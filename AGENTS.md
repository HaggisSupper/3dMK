# 3DMk Agent Contract

## Mission

Implement the authoritative revision workflow:

`docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`

Apply the governing CUDA-first system amendment:

`docs/superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md`

The goal is not to produce more scaffolding. The goal is a verified 3DMk system in which Rust project/revision state is authoritative from import through processing, review, save, reopen, and export, and in which CUDA is the primary execution substrate for eligible compute across the complete product.

Where the older revision-workflow plan or related documents describe CUDA as outside the product scope or optional only for the coding executor, the CUDA-first system design governs.

## Active execution backend

The active executor experiment is the local Mistral.rs harness documented in:

`docs/agent-execution/MISTRALRS_LOCAL_AGENT.md`

Mistral.rs runs a local model and shell loop against an isolated worktree. OpenCode files remain in the repository as inactive historical tooling; they are not required by, and must not be invoked during, this experiment.

The local Mistral.rs executor must run with CUDA acceleration. A binary that is not compiled with CUDA, a host that does not expose an NVIDIA CUDA device, or a model server process that is not observed using the GPU is a hard blocker.

The Mistral.rs requirement establishes CUDA-capable NVIDIA hardware as a minimum requirement for the complete 3DMk system. A CPU-only machine may display hardware diagnostics, but it is not a compliant operational installation.

Changing the executor does not remove or relax product architecture, task order, evidence gates, branch rules, revision authority, or completion criteria.

## Mandatory context order

At the beginning of every implementation, review, or verification session:

1. Read this file.
2. Read `docs/agent-execution/VWM_PROGRESS.md`.
3. Read the authoritative revision-workflow plan.
4. Read the CUDA-first system design.
5. Read only the current task and its directly referenced files in detail.
6. Inspect `git status`, the current branch, recent commits, and the relevant existing tests.
7. Resume from the first incomplete task whose dependencies are complete.

Do not re-plan the product. Do not replace either governing document with a smaller interpretation.

## Platform and stack

- Windows 11 and PowerShell 7 are the reference environment.
- Rust is primary.
- Tauri 2 is the desktop framework.
- Axum remains the single browser/Tauri backend.
- The existing Three.js UI is modularized incrementally; no framework rewrite in the authoritative revision lane.
- A supported NVIDIA CUDA-capable GPU is a minimum system requirement.
- CUDA is the primary backend for eligible geometry, point-cloud, reconstruction, vision, inference, image-processing, and future simulation workloads.
- Mistral.rs must remain CUDA-backed; CPU LLM inference is prohibited.
- WebGPU/Vulkan may be used as a verified fallback for individual non-LLM capabilities but does not satisfy the system CUDA requirement.
- CPU remains the control, I/O, persistence, and deterministic-reference plane; it is not the normal production backend for heavy compute after CUDA parity is accepted.
- No Docker, Podman, WSL, Electron, or Python production backend.
- No Apple-specific development chain.

## Accelerator rules

- The complete system does not enable product workflows until NVIDIA driver, CUDA device, CUDA-enabled Mistral.rs process, product CUDA smoke kernel, and GPU resource-broker gates pass.
- Compute-heavy operations select CUDA first and record the device, backend, kernel or engine version, precision, memory high-water mark, transfer counts, timings, and fallback reason.
- The resource broker coordinates Mistral.rs and product workloads against observed VRAM; independent optimistic allocation is forbidden.
- Bulk geometry never crosses Tauri IPC. The frontend exchanges IDs, parameters, summaries, progress, and bounded evidence assets.
- GPU buffers and caches are derived and disposable. Canonical project assets, stable source IDs, revisions, analyses, and provenance remain CPU-authoritative and durable.
- A failed, cancelled, timed-out, or out-of-memory GPU operation publishes no derived revision or partial asset.
- WebGPU/Vulkan fallback is allowed only where the capability contract lists it and tests prove it.
- CPU reference implementations remain available for differential testing, deterministic fixtures, and diagnostics until the corresponding CUDA path passes parity and failure gates.
- A CUDA feature flag, crate dependency, or route is not evidence that a capability is available. Live execution and benchmark evidence are required.

## Authority rules

- Original source bytes and root measured revisions are immutable.
- Rust owns authoritative geometry, operations, analyses, revisions, persistence, accelerator scheduling, and export.
- JavaScript owns rendering, input, evidence capture, workflow presentation, and review.
- `loadedGeometryStore` is a render cache, never the document of record.
- Every geometry mutation creates a candidate child revision.
- Analysis does not masquerade as geometry.
- AI or VLM output is advisory and cannot directly mutate accepted geometry.
- Exact point, vertex, face, primitive, or instance membership replaces bounding-box deletion.
- No operation may silently discard normals, colors, UVs, materials, textures, source IDs, calibration, measurements, accelerator provenance, or operation history.

## Execution method

Work one authoritative plan task at a time.

For every task:

1. Re-state the task's concrete acceptance conditions in the progress ledger.
2. Include all applicable CUDA-first requirements from the governing design.
3. Write the smallest failing test that proves the missing behavior.
4. Run it and confirm it fails for the intended reason.
5. Implement the smallest coherent production change.
6. Run the focused test.
7. Run affected regression, CPU-reference parity, and CUDA-specific tests where applicable.
8. Run an independent reviewer session against the task requirements and complete task diff.
9. Run an independent verifier session with the exact verification commands.
10. Fix every blocking or important finding.
11. Re-run review and verification after repairs.
12. Update `docs/agent-execution/VWM_PROGRESS.md` with commands, results, files, commit, accelerator evidence, and residual risks.
13. Commit the task with the plan's prescribed message or a more accurate equivalent.
14. Push only after the task's independent review and verification gates pass.
15. Continue only when the task's dependency gate is satisfied.

The implementer, reviewer, and verifier must use separate model sessions. Reviewer and verifier sessions are read-only with respect to tracked repository files.

Never combine unrelated plan tasks into one unreviewable change. Do not bypass revision authority in order to accelerate CUDA work.

## Sequencing rule

- Authoritative Tasks 1-4 remain first.
- Task 5 stable source IDs remains the data-identity prerequisite.
- The CUDA foundation batch defined by the CUDA-first design is introduced after Task 5 without renumbering the authoritative tasks.
- Tasks 6-14 consume that foundation.
- Task 15 retires browser-owned production compute paths.
- Task 16 packages CUDA hardware/runtime diagnostics and offline delivery.
- Task 17 includes CUDA coexistence, performance, memory, fault, and clean-machine evidence.

## Autonomy and questions

Resolve implementation details from the repository, tests, official documentation, benchmarks, and the governing documents.

Ask the user only when one of these is true:

- a credential or licensed model asset is required;
- an irreversible product decision is absent from the governing documents;
- three evidence-based fix attempts expose a genuine architecture conflict;
- local-only files differ materially from GitHub and the correct source cannot be inferred.

Do not ask for approval of ordinary code, test, refactor, CUDA-backed model fallback, kernel implementation, memory-budget tuning, or branch decisions already governed by the plans and runbook.

## Git discipline

- Never implement on `main` or `master`.
- Use `agent/vwm-authoritative-revision-implementation`.
- Use an isolated git worktree for model-driven edits.
- Do not force push.
- Do not use `git reset --hard` or `git clean`.
- Preserve unrelated user changes.
- Commit after each verified task.
- Pushing the implementation branch is permitted only after the corresponding task is independently reviewed and verified.
- Never merge directly into `main`; open or update a draft PR.

## Completion vocabulary

Use these exact states:

- `TASK_CANDIDATE` — implementation is committed locally but independent gates or GitHub synchronization remain.
- `TASK_COMPLETE` — one authoritative plan task is independently reviewed, verified, recorded, and pushed.
- `BATCH_COMPLETE` — the plan's required first execution batch is fully verified.
- `SESSION_BOUNDARY` — work is committed and resumable, but the plan remains incomplete.
- `BLOCKED` — external evidence shows progress cannot continue without a credential, licensed asset, architecture decision, supported CUDA hardware, or required local toolchain.
- `PROJECT_COMPLETE` — all authoritative tasks, the CUDA-first augmentation, and every Definition-of-Done item have fresh evidence.

Do not say “done,” “complete,” “finished,” or give a project percentage unless the state is `PROJECT_COMPLETE`.

## Project completion gate

`PROJECT_COMPLETE` requires all of the following:

- all 17 authoritative task boxes and every CUDA-foundation task are checked;
- the full Rust and VWM workspace tests pass;
- strict Clippy and formatting pass;
- the Tauri application builds and launches offline on the reference NVIDIA system;
- startup proves the CUDA-enabled Mistral.rs process, product CUDA runtime, known-answer smoke kernel, GPU resource broker, and safety reserve;
- the real vertical slice passes: import → analyze → exact cleanup candidate → compare → accept → close → reopen → save → re-import → explicit GLB and PLY export;
- representative geometry, reconstruction, vision, and inference operations use CUDA as their primary backend or have an explicitly approved exception;
- Mistral.rs and product GPU jobs coexist without uncontrolled CUDA OOM under the reference acceptance scenario;
- CUDA and CPU-reference outputs pass declared equivalence tolerances and deterministic ordering gates;
- exported packages preserve accepted revision identity, operation history, analyses, source selections, attribute contracts, accelerator provenance, and asset digests;
- exports exclude viewport helpers by default;
- failure and cancellation publish no partial revision;
- UI state and active backend truth are visible at all times;
- every advertised WebGPU/Vulkan fallback is implemented and verified;
- the completion report contains commands, versions, fixtures, timings, hashes, CUDA device evidence, VRAM high-water marks, benchmark comparisons, screenshots, and limitations.

Evidence before assertion. No exceptions.
