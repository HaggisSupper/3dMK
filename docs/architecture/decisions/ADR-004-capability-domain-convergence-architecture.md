# ADR-004 — Veritas-Child Capability Domain and Convergence Architecture

**Status:** Proposed  
**Date:** 2026-08-18

## Context

3DMk has substantial geometry, VWM, perception, revision, export, CUDA-first, and workstation functionality. Continued growth through UI-bound workflows increases coupling and operator complexity.

3DMk is not a peer framework. It SHALL remain a child domain of Veritas, which owns the generic capability/runtime architecture. 3DMk contributes geometry-specific capabilities, states, evaluators, providers, evidence representations, and presentation.

The product also requires iterative processing where a later turn is not assumed to be better than an earlier candidate. Higher reasoning may be useful when lower-cost mechanisms cannot resolve a request, but probabilistic inference must never become authoritative for geometry mutation or publication.

## Decision

3DMk SHALL evolve incrementally into a Veritas-child geometry capability domain with strongly typed capability contracts, deterministic plan admission, hardware-aware execution, candidate-state processing, measurable outcome evaluation, and bounded convergence.

Normative decisions:

1. Veritas is the core; 3DMk is always its child capability domain.
2. Migrated 3DMk surfaces SHALL consume canonical, versioned Veritas contracts/runtime interfaces; Veritas SHALL NOT depend on 3DMk.
3. Generic capability/runtime/provenance/resource/planning/convergence/escalation contracts belong upstream in Veritas and SHALL NOT be shadowed locally.
4. If a required generic contract is missing upstream, implementation pauses at that boundary until Veritas supplies a versioned contract; 3DMk SHALL NOT invent a local substitute.
5. Tauri remains a first-class standalone host but SHALL NOT own domain authority.
6. Standalone 3DMk packages pinned required Veritas core components with the 3DMk domain; it does not become architecturally independent and does not require a separately installed Veritas application.
7. Capability composition derives primarily from types, predicates, preconditions, postconditions, effects, and hard constraints.
8. Planning legality SHALL remain deterministic and bounded.
9. Mutations SHALL produce candidate revisions and follow the authoritative transaction boundary.
10. `LatestState`, `BestState`, `PresentedState`, and `AuthoritativeState` SHALL remain distinct concepts.
11. Outcome evaluators SHALL measure actual candidate quality; inferior later candidates SHALL NOT displace superior incumbents.
12. Generic hardware/resource arbitration belongs to Veritas; 3DMk supplies geometry providers and domain-specific cost/parity evidence.
13. A bounded competence escalation path MAY be enabled: deterministic context → optional embedded narrow ML → optional local LLM/VLM/multimodal → optional approved OpenAI-compatible cloud model.
14. A deterministic Veritas escalation packager constructs the next tier's constrained context. Embedded-model outputs MAY inform that package; the architecture does not require the embedded model itself to be generative.
15. Every enabled tier SHALL resolve, abstain, escalate, or fail through typed dispositions and SHALL NOT modify hard constraints or authority.
16. Every higher-tier response SHALL return through schema validation, deterministic capability admission, resource admission, candidate execution, and outcome evaluation.
17. Cloud escalation SHALL be optional, policy-controlled, data-minimized, and fail-closed.
18. Exact ML architecture, adapter method, RL, IA3, LoRA/xLoRA, unsupervised learning, prompt optimization, and provider choice are not normative without separate evidence.
19. 3DMk SHALL ALWAYS remain buildable and deployable as a complete Tauri application.
20. Every proposed mechanism is subject to the epistemic challenge gate: unsupported speculation remains non-normative; high-confidence claims are attacked further until remaining uncertainty is explicit and bounded.

## Known upstream dependency

Existing Veritas governance includes managed processor-server assumptions, while the 3DMk target includes a legitimate case for approved lightweight in-process providers. 3DMk SHALL not resolve that generically downstream. Veritas must first define a canonical host/provider topology contract that can admit both managed-server and approved in-process providers while preserving Veritas resource authority.

Until that upstream contract exists and is pinned, intelligence-provider implementation remains blocked at the generic boundary; this does not block deterministic geometry capability migration that uses already-canonical Veritas contracts.

## Consequences

### Positive

- 3DMk cannot drift into a competing framework implementation.
- Geometry operations become explicit, typed, reusable Veritas capabilities.
- Algorithm and accelerator implementations can sit behind stable capability identities.
- Processing dependencies and authority become machine-verifiable.
- Iterative processing preserves the strongest measured state instead of blindly accepting recency.
- Difficult requests can escalate without giving models application authority.
- Prompt/context construction can use deterministic application knowledge rather than forcing larger models to rediscover state.
- Standalone Tauri deployment remains intact.

### Costs

- Requires disciplined upstream/downstream versioning and can deliberately block downstream work when Veritas lacks a generic contract.
- Requires contract, adapter, evaluator, convergence, escalation, provenance, fault, and standalone-package tests.
- Local higher-tier models add resource-management complexity when enabled.
- Cloud escalation adds privacy, policy, credential, and availability boundaries.

## Rejected alternatives

- **3DMk defines its own generic capability framework:** rejected because it creates drift and duplicates Veritas.
- **General LLM as primary controller:** rejected because it increases nondeterminism and weakens auditability.
- **Always invoke the largest model:** rejected because escalation must be competence- and cost-driven.
- **Require the embedded model to generate prompts:** rejected because a deterministic packager can construct bounded prompts from typed model outputs with less authority and less model complexity.
- **Enumerate every legal path ahead of time:** rejected because it is combinatorial and brittle.
- **Latest result becomes authoritative:** rejected because later processing can regress.
- **Experimental adaptation mechanisms as requirements:** rejected until measured justification exists.
- **Local 3DMk exception to Veritas processor topology:** rejected because generic topology belongs upstream.

## Compliance

Implementation SHALL comply with:

- `docs/architecture/CAPABILITY_FRAMEWORK_MASTER_SPEC.md`
- `docs/architecture/IMPLEMENTATION_GUARDRAILS.md`
- `docs/architecture/NON_NEGOTIABLE_ACCEPTANCE_GATES.md`
- `docs/architecture/OUTCOME_EVALUATION_AND_CONVERGENCE_SPEC.md`
- `docs/architecture/HARDWARE_AWARE_EXECUTION_SPEC.md`
- `docs/architecture/INTELLIGENCE_ESCALATION_SPEC.md`

Existing ADR-001 through ADR-003 remain in force. This ADR SHALL not weaken their transaction, worker, or cache guarantees.