---
description: Execute or resume the authoritative 3DMk VWM implementation plan
agent: vwm-executor
---

Read, in order:

1. `AGENTS.md`
2. `docs/agent-execution/VWM_PROGRESS.md`
3. `docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`

Inspect the repository and resume at the first incomplete task whose dependencies are complete.

Execute one plan task at a time using the mandatory red-green-refactor, independent review, independent verification, evidence-ledger, and commit cycle. Continue across tasks while context and evidence remain reliable.

Do not add new geometry research, CUDA, CFD/FEM, Mamba, diffusion, BIM expansion, or a UI framework rewrite. Do not skip Tasks 1–4. Do not redefine “done.”

If the context window becomes constrained, leave a clean commit, update the ledger, and report `SESSION_BOUNDARY`.

If a genuine external blocker exists, record exact evidence in the ledger and report `BLOCKED`.

Report `PROJECT_COMPLETE` only after all 17 tasks and every Definition-of-Done item have fresh evidence.

Additional operator instruction: $ARGUMENTS
