# 3DMk Documentation

This directory contains the current, authoritative documentation for 3DMk. Historical plans, completed phase reports, abandoned executor instructions, and prototype handoff notes are intentionally removed from the active tree; Git history remains the audit record.

## Authority order

When two documents appear to conflict, use this order:

1. `../AGENTS.md` — execution, evidence, branch, safety, disposition, and completion rules.
2. `CURRENT_STATE.md` — implementation truth: what exists now and what does not. It does not cancel an approved future target merely because that target is not implemented.
3. `superpowers/specs/2026-08-07-3dmk-world-class-quality-standard.md` — product quality and reliability standard.
4. `superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md` — mandatory CUDA system architecture.
5. `architecture/decisions/` — accepted architecture decisions, including ADR-004's permanent Veritas-child relationship and bounded intelligence-escalation decision.
6. `architecture/CAPABILITY_FRAMEWORK_AGENT_GOVERNANCE.md` — capability-domain agent rules, epistemic challenge, and disposition-to-documentation contract.
7. `architecture/CAPABILITY_FRAMEWORK_MASTER_SPEC.md` — approved Veritas-child target architecture.
8. `architecture/IMPLEMENTATION_GUARDRAILS.md` and `architecture/NON_NEGOTIABLE_ACCEPTANCE_GATES.md` — implementation prohibitions and evidence gates.
9. `architecture/OUTCOME_EVALUATION_AND_CONVERGENCE_SPEC.md`, `architecture/HARDWARE_AWARE_EXECUTION_SPEC.md`, and `architecture/INTELLIGENCE_ESCALATION_SPEC.md` — bounded domain specifications.
10. `architecture/CAPABILITY_FRAMEWORK_IMPLEMENTATION_SEQUENCE.md` — evidence-driven migration sequence.
11. `superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md` — dependency-ordered product implementation plan.
12. `superpowers/plans/2026-08-07-3dmk-foundation-batch.md` — CUDA, transactional-storage, supervision, and observability foundation inserted after authoritative Task 5.
13. `superpowers/plans/2026-08-07-3dmk-world-class-program.md` — cross-cutting reliability and release-hardening program.
14. `agent-execution/` — live runbooks, progress ledgers, risks, and acceptance evidence.
15. `../VWM-Repo-Implicit/README.md` and crate READMEs — current reusable engine boundaries.

A statement in `CURRENT_STATE.md` that a target capability is not implemented describes present truth; it SHALL NOT be read as rejecting an approved target architecture. Conversely, an approved target architecture SHALL NOT be represented as implemented until `CURRENT_STATE.md` is changed by fresh evidence.

## Current capability architecture

The approved target architecture is anchored by:

- `architecture/decisions/ADR-004-capability-domain-convergence-architecture.md`;
- `architecture/CAPABILITY_FRAMEWORK_MASTER_SPEC.md`;
- `architecture/CAPABILITY_FRAMEWORK_AGENT_GOVERNANCE.md`;
- `architecture/IMPLEMENTATION_GUARDRAILS.md`;
- `architecture/NON_NEGOTIABLE_ACCEPTANCE_GATES.md`.

3DMk SHALL remain a child capability domain of Veritas. Missing generic contracts are upstream Veritas work, not permission to create local shadow framework contracts.

## Current ledgers

- `agent-execution/VWM_PROGRESS.md` — authoritative Tasks 1–17.
- `agent-execution/CUDA_FOUNDATION_PROGRESS.md` — foundation tasks FB1–FB8.
- `agent-execution/WORLD_CLASS_ACCEPTANCE_MATRIX.md` — objective release gates.
- `agent-execution/WORLD_CLASS_RISK_REGISTER.md` — active engineering risks and controls.

## Documentation disposition policy

Repository authority SHALL reflect explicit decisions rather than leaving stronger intent only in conversation:

- approved requirements use normative `SHALL`, `MUST`, or `REQUIRED` language with scope and acceptance evidence;
- rejected mechanisms are removed or normatively prohibited where recurrence must be prevented;
- superseded material is removed from active authority or points explicitly to its replacement;
- deferred, hypothesis, and riff material remains expressly non-normative;
- contradictory active normative requirements are a blocking documentation defect;
- approval of a target does not constitute implementation evidence.

Before promoting a proposed mechanism, the capability governance requires the sanity check: **“How much of this is speculative bullshit because we enjoy the riff?”** High-confidence proposals receive stronger disconfirmation rather than automatic promotion.

## Documentation policy

A document belongs in the active tree only when it does at least one of the following:

- describes behavior that exists in the current source;
- defines an accepted governing contract;
- defines dependency-ordered work that has not yet been completed;
- records fresh verification evidence;
- documents an operational procedure that can be executed now.

Superseded plans, stale completion claims, obsolete executor material, and prototype-specific handoff notes are deleted rather than left beside authoritative documents. Significant decisions are preserved as ADRs and all deleted material remains recoverable through Git history.

## Status vocabulary

- `implemented` — present in source and covered by current evidence.
- `experimental` — present, but missing one or more production acceptance gates.
- `degraded` — usable with a known limitation that is reported to the operator.
- `planned` — approved target design or plan, not implemented.
- `hypothesis` — plausible but not approved as implementation authority.
- `riff` — exploratory speculation; never implementation authority.
- `unavailable` — not implemented or blocked by a missing dependency.

No route, button, document, crate, feature flag, approval statement, or model response by itself proves that a capability is implemented.
