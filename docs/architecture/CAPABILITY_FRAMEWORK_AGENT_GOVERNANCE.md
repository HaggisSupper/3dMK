# 3DMk Capability Domain Agent Governance

This document governs any agent implementing, reviewing, or modifying the Veritas-child capability architecture.

## Governing order

Read and obey in this order:

1. `AGENTS.md`
2. `docs/CURRENT_STATE.md`
3. authoritative Veritas contracts/runtime governance pinned by 3DMk
4. `docs/architecture/decisions/ADR-004-capability-domain-convergence-architecture.md`
5. `docs/architecture/CAPABILITY_FRAMEWORK_MASTER_SPEC.md`
6. `docs/architecture/IMPLEMENTATION_GUARDRAILS.md`
7. `docs/architecture/NON_NEGOTIABLE_ACCEPTANCE_GATES.md`
8. `docs/architecture/OUTCOME_EVALUATION_AND_CONVERGENCE_SPEC.md`
9. `docs/architecture/HARDWARE_AWARE_EXECUTION_SPEC.md`
10. `docs/architecture/INTELLIGENCE_ESCALATION_SPEC.md`
11. Existing ADR-001 through ADR-003 and current revision/CUDA standards.

`docs/CURRENT_STATE.md` is implemented truth. Target architecture is not implementation evidence.

## Primary invariant

3DMk SHALL always remain a child of Veritas. Veritas is the core.

Agents SHALL NOT create a competing generic capability framework inside 3DMk. Generic concepts go upstream first; 3DMk owns geometry-specific specialization.

If a required generic contract does not yet exist in Veritas, stop at that boundary and open/resolve upstream work. Do not create a temporary local substitute.

## Mandatory epistemic challenge

Before promoting architecture, ask:

> How much of this is speculative bullshit because we enjoy the riff?

Classify claims as `ESTABLISHED`, `VALIDATED`, `HYPOTHESIS`, or `RIFF`. Only the first two may become normative.

High confidence requires additional disconfirmation: strongest countercase, boundary/fault testing, comparison with simpler alternatives, reproducibility, and explicit residual uncertainty/error bounds.

## Mandatory disposition-to-documentation rule

Conversation approval or rejection SHALL NOT remain stronger than repository documentation.

When an architecture, plan, contract, implementation decision, mechanism, or constraint receives an explicit disposition, the governing repository documents SHALL be updated in the same bounded change so that the disposition is machine- and human-unambiguous:

- `APPROVED` requirements SHALL be expressed normatively with `SHALL`, `MUST`, `REQUIRED`, or an equivalent enforceable contract plus explicit scope and acceptance evidence.
- `REJECTED` mechanisms SHALL be removed when they have no continuing historical value, or expressed as `SHALL NOT` / prohibited when future reintroduction must be prevented.
- `SUPERSEDED` material SHALL be removed from active authority or explicitly marked superseded with its authoritative replacement.
- `DEFERRED`, `HYPOTHESIS`, and `RIFF` material SHALL remain expressly non-normative and SHALL NOT be used as implementation authority.
- Approval SHALL NOT convert an unvalidated factual claim into implemented truth. `docs/CURRENT_STATE.md` changes only when implementation evidence exists.
- A later disposition SHALL update or remove contradictory earlier normative text; contradictory active requirements are a blocking defect.

An implementation agent SHALL fail closed when conversation intent and active normative repository authority disagree. The conflict must be resolved in the repository before implementation continues.

## Mandatory implementation behavior

Agents SHALL:

- preserve revision/transaction authority;
- migrate incrementally rather than rewrite working geometry wholesale;
- consume canonical pinned Veritas contracts through a thin adapter;
- reject shadow generic contracts and dependency inversion;
- keep geometry/domain logic independent of Tauri;
- preserve complete standalone Tauri packaging of required Veritas core + 3DMk child domain;
- keep capability legality/publication deterministic;
- maintain the best-known valid checkpoint where iteration is enabled;
- prove inferior later candidates cannot replace superior incumbents;
- route resource placement through Veritas authority;
- preserve CUDA-first behavior for eligible heavy workloads after parity gates;
- retain verified fallback/reference paths where required;
- preserve full provenance.

## Intelligence escalation behavior

If enabled, the approved competence order is:

```text
deterministic semantic/context normalization
→ optional embedded narrow ML
→ optional local LLM/VLM/multimodal model
→ optional approved OpenAI-compatible cloud model
```

Agents SHALL:

- require typed `RESOLVED`, `ABSTAINED`, `ESCALATE`, or `FAILED` dispositions;
- use a deterministic Veritas escalation packager to construct the next tier's bounded context;
- permit embedded-model outputs to inform that package without requiring the embedded model to generate free-form prompts;
- preserve original operator intent, deterministic facts, hard constraints, policy, and provenance;
- lazy-load higher local models only after resource admission;
- make cloud escalation optional, policy-controlled, data-minimized, and fail-closed;
- route every model response through schema validation and deterministic capability/resource admission;
- never let model confidence override hard constraints or measured outcome evidence.

Exact model families, adapters, RL, IA3, LoRA/xLoRA, unsupervised learning, or prompt-optimization techniques remain non-normative until separately validated and explicitly dispositioned.

## Forbidden shortcuts

Agents SHALL NOT:

- make 3DMk a peer or parent of Veritas;
- duplicate generic Veritas framework logic locally;
- create a local in-process-provider exception to Veritas topology policy;
- let UI code become geometry authority;
- execute unvalidated plans;
- treat newest as best;
- collapse safety/hard constraints into a soft score;
- seize accelerator resources outside central governance;
- send raw bulk point-cloud state to general LLMs by default;
- place credentials into prompts or provenance;
- make cloud or a large model mandatory for deterministic core workflows;
- implement speculative adaptation mechanisms as production requirements without challenge-gate evidence;
- call architecture scaffolding production-complete.

## Required verification

For each migrated capability: behavior characterization, canonical contract conformance, invalid/precondition tests, transaction/failure tests where mutating, provider parity where applicable, provenance, planner admission/rejection, evaluator evidence where applicable, standalone packaging path, and regression against existing behavior.

For convergence: incumbent retention, bounded stopping, cancellation, restart where required, hard-constraint dominance, evaluator failure handling, and deterministic legality.

For intelligence escalation: disposition behavior, deterministic schema-bounded package construction, prompt-injection containment, stale-revision rejection, resource admission, local-model failure/OOM containment, cloud-policy denial/offline/timeout, credential isolation, malformed-response rejection, confident-but-worse proposal rejection, and deterministic revalidation.

For documentation disposition: approved/rejected/superseded decisions are reflected normatively, contradictory active requirements are absent, and implementation-truth documents contain no unevidenced completion claims.

## Stop condition

If a proposal weakens Veritas parent authority, standalone Tauri packaging, revision safety, deterministic legality, evidence requirements, resource governance, privacy boundaries, normative documentation integrity, or the epistemic challenge gate, stop the proposal and record the conflict.