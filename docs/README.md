# 3DMk Documentation

This directory contains the current, authoritative documentation for 3DMk. Historical plans, completed phase reports, abandoned executor instructions, and prototype handoff notes are intentionally removed from the active tree; Git history remains the audit record.

## Authority order

When two documents appear to conflict, use this order:

1. `../AGENTS.md` — execution, evidence, branch, safety, and completion rules.
2. `CURRENT_STATE.md` — what is implemented now and what is not.
3. `superpowers/specs/2026-08-07-3dmk-world-class-quality-standard.md` — product quality and reliability standard.
4. `superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md` — mandatory CUDA system architecture.
5. `architecture/decisions/` — accepted implementation decisions.
6. `superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md` — dependency-ordered product implementation plan.
7. `superpowers/plans/2026-08-07-3dmk-foundation-batch.md` — CUDA, transactional-storage, supervision, and observability foundation inserted after authoritative Task 5.
8. `superpowers/plans/2026-08-07-3dmk-world-class-program.md` — cross-cutting reliability and release-hardening program.
9. `agent-execution/` — live runbooks, progress ledgers, risks, and acceptance evidence.
10. `../VWM-Repo-Implicit/README.md` and crate READMEs — current reusable engine boundaries.

## Current ledgers

- `agent-execution/VWM_PROGRESS.md` — authoritative Tasks 1–17.
- `agent-execution/CUDA_FOUNDATION_PROGRESS.md` — foundation tasks FB1–FB8.
- `agent-execution/WORLD_CLASS_ACCEPTANCE_MATRIX.md` — objective release gates.
- `agent-execution/WORLD_CLASS_RISK_REGISTER.md` — active engineering risks and controls.

## Documentation policy

A document belongs in the active tree only when it does at least one of the following:

- describes behavior that exists in the current source;
- defines an accepted governing contract;
- defines dependency-ordered work that has not yet been completed;
- records fresh verification evidence;
- documents an operational procedure that can be executed now.

Superseded plans, stale completion claims, obsolete OpenCode material, and prototype-specific handoff notes are deleted rather than left beside authoritative documents. Significant decisions are preserved as ADRs and all deleted material remains recoverable through Git history.

## Status vocabulary

- `implemented` — present in source and covered by current evidence.
- `experimental` — present, but missing one or more production acceptance gates.
- `degraded` — usable with a known limitation that is reported to the operator.
- `planned` — accepted design or plan, not implemented.
- `unavailable` — not implemented or blocked by a missing dependency.

No route, button, document, crate, or feature flag by itself proves that a capability is implemented.