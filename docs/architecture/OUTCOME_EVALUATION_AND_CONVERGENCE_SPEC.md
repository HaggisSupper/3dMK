# 3DMk Outcome Evaluation and Convergence Specification

## 1. Objective

Turn capability execution into a bounded search for the best valid application state rather than a blind sequence that assumes the newest result is superior.

The framework SHALL preserve the strongest valid checkpoint discovered so far and SHALL be able to reject a later regression without losing provenance.

## 2. Core state identities

The runtime SHALL maintain distinct identities for:

- `LatestState` — newest candidate generated;
- `BestState` — best currently known valid candidate under the active goal;
- `PresentedState` — revision shown to the operator;
- `AuthoritativeState` — published project revision under the transaction model.

These MAY reference the same revision, but SHALL NOT be aliases by definition.

## 3. Goal evaluation contract

Every convergence-capable goal SHALL define hard constraints, objective evaluators, promotion policy, stop policy, and explicit search/resource budget through canonical Veritas contracts.

3DMk SHALL add only geometry-specific evaluator definitions.

## 4. Hard constraints versus objectives

Hard constraints are admissibility rules. Objective values are optimization signals.

Representative hard constraints:

- finite coordinates only;
- no self-intersections where prohibited;
- watertight mesh when required;
- maximum source deviation;
- minimum source coverage;
- mandatory primitive preservation;
- source-mapping completeness;
- legal coordinate-frame relationship;
- package/provenance integrity.

Representative objectives:

- geometric fidelity;
- completeness;
- topology quality;
- primitive fit;
- structural consistency;
- processing latency;
- memory cost.

A candidate that violates a hard constraint SHALL NOT be rescued by a high aggregate objective score.

## 5. Objective vector and error bounds

The evaluator SHALL preserve the full vector of measured values and declared tolerances/uncertainties where meaningful.

The goal policy determines whether a candidate improves the incumbent. No universal scalar quality score is assumed.

## 6. Promotion policy

Candidate promotion SHALL require:

1. all hard constraints pass;
2. evaluator evidence is complete enough for the goal;
3. measured improvement satisfies the declared promotion policy within its tolerance/error bounds;
4. the publication transaction passes.

A valid but inferior/non-improving candidate MAY remain inspectable but SHALL NOT silently replace the incumbent.

## 7. Initial search model

The initial production implementation SHALL use the simplest bounded deterministic search/iteration strategy that satisfies the demonstrated use case.

Conceptually:

```text
incumbent state
  → select an admissible transformation
  → execute candidate
  → measure actual result
  → promote or retain incumbent
  → stop or continue within explicit budget
```

A*, branch-and-bound, learned heuristics, RL, automatic policy learning, or other advanced search machinery are not requirements. They SHALL be introduced only after the epistemic challenge gate demonstrates that the simpler strategy is inadequate and the proposed mechanism materially improves outcomes.

## 8. Incumbent guarantee

At no point may a failed, invalid, or inferior candidate destroy the best known valid checkpoint.

A mandatory test SHALL create a sequence in which candidate N is better than N+1 and prove:

- N+1 remains recorded;
- N remains `BestState`;
- the operator may inspect N+1;
- authoritative publication remains explicit;
- subsequent processing cannot silently overwrite N merely because N+1 is newer.

## 9. Search history

The system SHALL preserve enough parent/candidate history to establish provenance and rollback. A full general-purpose search tree is not required unless the selected bounded strategy demonstrably needs one.

Each candidate record SHALL identify parent revision, capability/transformation, parameters, provider, measured evaluation, and disposition.

## 10. Stop policy

Every iterative goal SHALL define explicit stopping criteria appropriate to the goal, including applicable:

- hard constraints satisfied;
- measured improvement below epsilon/tolerance for a bounded number of accepted turns;
- time budget reached;
- compute/iteration budget reached;
- memory/resource safety veto;
- repeated state/operation cycle detected;
- operator cancellation;
- evaluator uncertainty outside acceptable bounds.

The exact stop mechanism SHALL be no more complex than required by evidence.

## 11. Cost-aware selection

The planner MAY use deterministic measured cost information such as runtime, memory, transfer/setup overhead, and provider availability to choose between otherwise admissible options.

Cost preferences derive from the active goal and Veritas resource policy, not universal hard-coded assumptions.

## 12. Intelligence escalation

If deterministic planning/evaluation cannot resolve an ambiguity, no legal path exists, or interpretation beyond the current tier's competence is required, the process MAY enter the bounded hierarchy defined in `INTELLIGENCE_ESCALATION_SPEC.md`.

A higher reasoning tier may propose a typed goal, constraint interpretation, or candidate action. It SHALL NOT determine authoritative truth by assertion. Any proposal re-enters deterministic contract validation, candidate execution, and measured outcome evaluation.

## 13. Provenance

Every evaluation record SHALL contain evaluator ID/version, candidate revision/hash, goal ID/version, metric vector, hard-constraint results, declared uncertainty/tolerance where applicable, comparison to incumbent, promotion/rejection reason, timestamp, and relevant hardware/provider evidence.

## 14. Recovery

Persistent iterative jobs SHALL be resumable only from explicit validated checkpoints. Recovery SHALL never infer success from temporary files.

On restart, the system SHALL reconcile authoritative project state, persisted candidate/evaluation ledger, active goal/budget, and best-known checkpoint. Mismatch SHALL fail closed or require reviewed recovery.

## 15. Challenge rule

Every extension to convergence mechanics SHALL answer:

> How much of this is speculative bullshit because we enjoy the riff?

High-confidence additions require stronger disconfirmation and explicit residual error bounds before they become normative.