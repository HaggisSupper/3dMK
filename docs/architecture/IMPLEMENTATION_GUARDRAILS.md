# 3DMk Capability Domain Implementation Guardrails

**Status:** Non-negotiable implementation constraints.

## 1. Veritas parent authority

1. 3DMk SHALL always be a child of Veritas. Veritas is the core.
2. Migrated 3DMk surfaces SHALL consume canonical versioned Veritas contracts/runtime interfaces; Veritas SHALL NOT depend on 3DMk.
3. Generic capability/runtime/provenance/planning/resource/convergence/escalation contracts SHALL be owned upstream by Veritas.
4. 3DMk SHALL NOT create shadow copies, local forks, or temporary generic substitutes.
5. If a required generic contract is absent upstream, work SHALL stop at that boundary until Veritas provides it.
6. Geometry-specific extensions MAY live in 3DMk only when demonstrably domain-specific.
7. Tauri SHALL remain presentation/deployment infrastructure, never domain authority.

## 2. Epistemic challenge gate

Before any proposed mechanism becomes normative, ask:

> How much of this is speculative bullshit because we enjoy the riff?

Every claim SHALL be classified `ESTABLISHED`, `VALIDATED`, `HYPOTHESIS`, or `RIFF`.

- `HYPOTHESIS` and `RIFF` SHALL NOT become required architecture.
- High confidence triggers additional attack, not automatic promotion.
- Test the strongest countercase, boundary/failure conditions, simpler alternatives, reproducibility, and domain-appropriate error bounds.
- Promotion requires zero known unsupported assertion inside the declared uncertainty envelope.
- If evidence does not justify complexity, use the simpler mechanism.

## 3. Deterministic authority

1. Capability legality, transaction authority, revision publication, hard constraints, and resource admission SHALL be deterministic.
2. Every plan SHALL be independently validated before execution.
3. Every mutation SHALL produce candidate state before publication.
4. Newest SHALL NOT imply best.
5. Hard constraints SHALL remain separate from optimization objectives.
6. Probabilistic subsystems may propose/classify/rank/summarize only; they cannot weaken deterministic authority.

## 4. Capability contracts

Every migrated capability SHALL satisfy canonical Veritas contracts and declare geometry-specific information required for stable identity/version, typed I/O, pre/postconditions, effects, hard constraints, typed/unit-bound parameters, risk/side effects, providers/resources, evaluator requirements, cancellation/failure behavior, and provenance.

Composition SHALL derive primarily from types and predicates, not pairwise successor tables.

## 5. Transaction and revision authority

1. Original imported bytes and authoritative parent revisions remain immutable.
2. Mutations SHALL follow `admit → stage → execute → validate → publish → finalize`.
3. Pre-publication output is never authoritative.
4. Topology changes SHALL preserve source mappings and attribute-transfer evidence.
5. Cancellation, timeout, error/panic, OOM, device loss, disk-full, or validation failure SHALL leave prior authoritative state intact.
6. Project-store locks SHALL NOT be held while waiting on long accelerator work.

## 6. Hardware/resource authority

1. Generic discovery, provider admission, reservation, placement, and provider topology belong to Veritas core.
2. 3DMk providers declare supported paths and geometry-specific cost/parity evidence.
3. Capabilities SHALL NOT seize accelerators outside Veritas resource authority.
4. CUDA remains primary for eligible heavy workloads on supported NVIDIA hardware after parity/reliability gates.
5. Vulkan/WebGPU fallback SHALL be verified per capability.
6. CPU reference paths SHALL exist where required for correctness evidence.
7. Correctness and memory safety override benchmark preference.
8. An in-process provider SHALL NOT be introduced as a 3DMk-local exception; it requires an upstream Veritas contract that explicitly admits that topology.

## 7. Planning and sequencing

1. Use the simplest bounded deterministic planning mechanism that meets measured requirements.
2. Emit only registered compatible capabilities.
3. Search SHALL have explicit bounds.
4. Contract-proven impossible branches SHALL not be expanded.
5. Execution SHALL use an explicit DAG or equivalent dependency representation.
6. Concurrency SHALL preserve transaction, memory, and resource safety.
7. Planner failure SHALL fail closed rather than invoke ad hoc imperative execution.

## 8. Outcome evaluation and incumbent preservation

1. Evaluators SHALL be geometry-goal-specific or explicitly reusable.
2. Hard constraints SHALL remain independently visible.
3. Objective dimensions SHALL remain auditable.
4. `LatestState`, `BestState`, `PresentedState`, and `AuthoritativeState` are distinct concepts.
5. Regression SHALL preserve the incumbent.
6. Evaluators SHALL declare measurement tolerance/uncertainty where meaningful.
7. Promotion requires measured evidence plus the authoritative publication transaction.

## 9. Bounded convergence

1. Iterative convergence SHALL be introduced only where validated benefit exists.
2. Every job SHALL define iteration/time/compute/memory limits.
3. Infinite loops are prohibited.
4. Best-known valid state survives failed/inferior iterations.
5. Cancellation and restart behavior SHALL be explicit and tested where persistence is required.
6. Plateau/oscillation handling SHALL be no more complex than required by the selected strategy.
7. A*, RL, learned ranking, IA3, LoRA/xLoRA, automatic adaptation, or other advanced mechanisms require separate challenge-gate evidence.

## 10. Intelligence escalation

1. Escalation plumbing belongs to Veritas core; 3DMk provides geometry context/evidence and typed domain goals.
2. A deterministic escalation packager SHALL construct higher-tier context and preserve original intent, hard constraints, policy, and provenance.
3. Embedded-model outputs MAY inform that package but free-form model text is not authoritative.
4. Every enabled tier SHALL return typed `RESOLVED`, `ABSTAINED`, `ESCALATE`, or `FAILED` disposition.
5. Higher local models SHALL be lazy-loaded and resource-admitted.
6. Cloud use SHALL be optional, explicitly policy-admitted, data-minimized, and restricted to approved OpenAI-compatible endpoints.
7. Credentials SHALL never enter prompts, project payloads, model artifacts, or provenance logs.
8. Raw bulk geometry SHALL not enter general-model context by default; derived multimodal evidence requires typed provenance.
9. Every model response SHALL re-enter schema validation and deterministic capability/resource admission.
10. Model confidence SHALL NOT override legality, hard constraints, or measured outcome evidence.
11. Every probabilistic tier can be disabled without breaking deterministic core geometry workflows.

## 11. Standalone Tauri deployment

1. 3DMk SHALL ALWAYS remain buildable and deployable as Tauri 2.
2. Standalone packages pinned required Veritas core components with the 3DMk child domain; it does not mean architectural independence.
3. No separately installed Veritas application or cloud service is required for deterministic core workflows.
4. Geometry/domain crates remain independent of Tauri.
5. Standalone and Veritas-hosted calls preserve equivalent domain authority semantics.

## 12. Drift prevention

1. Generic changes go upstream first.
2. 3DMk SHALL pin released Veritas versions and contract fingerprints.
3. Never copy mutable Veritas files from `main` as a synchronization strategy.
4. CI SHALL reject incompatible pins, dependency cycles, and shadow generic contracts.
5. Adapter code SHALL remain thin and contain no duplicated domain logic.

## 13. Migration discipline

Characterize before wrapping. Migrate incrementally. Preserve VWM, RANSAC-LP, GON, import/export, measurement, revision, package, and viewer behavior until replacements are proven. Do not remove a legacy path without replacement, migration, rollback, and regression evidence. Documentation/scaffolding is never production-complete evidence.

## 14. Evidence before promotion

Applicable evidence includes contract/negative tests, deterministic replay, fault injection, CPU/GPU parity, cancellation/OOM/device-loss, persistence/recovery, representative geometry fixtures, dependency/shadow-contract checks, escalation adversarial tests, standalone clean-machine/offline tests, regression against current behavior, and explicit countercase results.

Evidence before assertion. No exceptions.