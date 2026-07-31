# 3DMk OpenCode Agent Contract

## Mission

Implement the authoritative plan:

`docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`

The goal is not to produce more scaffolding. The goal is a verified 3DMk application in which Rust project/revision state is authoritative from import through processing, review, save, reopen, and export.

## Mandatory context order

At the beginning of every session:

1. Read this file.
2. Read `docs/agent-execution/VWM_PROGRESS.md`.
3. Read the authoritative implementation plan.
4. Read only the current task and its directly referenced files in detail.
5. Inspect `git status`, the current branch, recent commits, and the relevant existing tests.
6. Resume from the first incomplete task whose dependencies are complete.

Do not re-plan the product. Do not replace the plan with a smaller interpretation.

## Platform and stack

- Windows 11 and PowerShell 7 are the reference environment.
- Rust is primary.
- Tauri 2 is the desktop framework.
- Axum remains the single browser/Tauri backend.
- The existing Three.js UI is modularized incrementally; no framework rewrite in this lane.
- CUDA is not part of this plan.
- No Docker, Podman, WSL, Electron, or Python production backend.
- No Apple-specific development chain.

## Authority rules

- Original source bytes and root measured revisions are immutable.
- Rust owns authoritative geometry, operations, analyses, revisions, persistence, and export.
- JavaScript owns rendering, input, evidence capture, workflow presentation, and review.
- `loadedGeometryStore` is a render cache, never the document of record.
- Every geometry mutation creates a candidate child revision.
- Analysis does not masquerade as geometry.
- AI or VLM output is advisory and cannot directly mutate accepted geometry.
- Exact point, vertex, face, primitive, or instance membership replaces bounding-box deletion.
- No operation may silently discard normals, colors, UVs, materials, textures, source IDs, calibration, measurements, or provenance.

## Execution method

Work one authoritative plan task at a time.

For every task:

1. Re-state the task's concrete acceptance conditions in the progress ledger.
2. Write the smallest failing test that proves the missing behavior.
3. Run it and confirm it fails for the intended reason.
4. Implement the smallest coherent production change.
5. Run the focused test.
6. Run affected regression tests.
7. Invoke `vwm-reviewer` with the task, plan section, and complete diff.
8. Invoke `vwm-verifier` with the exact verification commands.
9. Fix every blocking or major finding.
10. Re-run verification.
11. Update `docs/agent-execution/VWM_PROGRESS.md` with commands, results, files, commit, and residual risks.
12. Commit the task with the plan's prescribed commit message or a more accurate equivalent.
13. Continue only when the task's dependency gate is satisfied.

Never combine unrelated plan tasks into one unreviewable change.

## Autonomy and questions

Resolve implementation details from the repository, tests, official documentation, and the plan.

Ask the user only when one of these is true:

- a credential or licensed model asset is required;
- an irreversible product decision is absent from the plan;
- three evidence-based fix attempts expose a genuine architecture conflict;
- local-only files differ materially from GitHub and the correct source cannot be inferred.

Do not ask for approval of ordinary code, test, refactor, or branch decisions already governed by the plan.

## Git discipline

- Never implement on `main` or `master`.
- Use `agent/vwm-authoritative-revision-implementation`.
- Do not force push.
- Do not use `git reset --hard` or `git clean`.
- Preserve unrelated user changes.
- Commit after each verified task.
- Pushing the implementation branch is permitted only after the corresponding task is verified.
- Never merge directly into `main`; open or update a PR.

## Completion vocabulary

Use these exact states:

- `TASK_COMPLETE` — one authoritative plan task is fully verified.
- `BATCH_COMPLETE` — the plan's required first execution batch is fully verified.
- `SESSION_BOUNDARY` — work is committed and resumable, but the plan remains incomplete.
- `BLOCKED` — external evidence shows progress cannot continue without a credential, licensed asset, or architecture decision.
- `PROJECT_COMPLETE` — all 17 tasks and every Definition-of-Done item have fresh evidence.

Do not say “done,” “complete,” “finished,” or give a project percentage unless the state is `PROJECT_COMPLETE`.

## Project completion gate

`PROJECT_COMPLETE` requires all of the following:

- all 17 task boxes in the progress ledger are checked;
- the full Rust and VWM workspace tests pass;
- strict Clippy and formatting pass;
- the Tauri application builds and launches offline;
- the real vertical slice passes: import → analyze → exact cleanup candidate → compare → accept → close → reopen → save → re-import → explicit GLB and PLY export;
- exported packages preserve accepted revision identity, operation history, analyses, source selections, attribute contracts, and asset digests;
- exports exclude viewport helpers by default;
- failure and cancellation publish no partial revision;
- UI state truth is visible at all times;
- the completion report contains commands, versions, fixtures, timings, hashes, screenshots, and limitations.

Evidence before assertion. No exceptions.
