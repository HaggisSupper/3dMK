# 3DMk Capability Domain Master Specification

**Status:** Proposed authoritative target architecture  
**Primary invariant:** 3DMk is always a child capability domain of Veritas. Veritas is the core.

## 1. Purpose

3DMk SHALL expose geometry functionality as strongly typed Veritas capabilities while preserving a complete standalone Tauri deployment.

Standalone packaging does not create an independent framework. A standalone release SHALL package the required Veritas core components together with the 3DMk domain and Tauri host. It SHALL NOT require a separately installed Veritas application.

3DMk owns geometry-domain behavior only. Generic capability, execution, provenance, resource, planning, checkpoint, convergence, and intelligence-escalation contracts belong upstream in Veritas.

## 2. Authority, ownership, and dependency direction

Architectural ownership flows from Veritas into 3DMk:

```text
Veritas core contracts/policy
        ↓ specializes
3DMk geometry child domain
        ↓ presented by
Tauri host
```

Compile/runtime dependencies point inward toward the core:

```text
Tauri host → 3DMk geometry domain/adapter → pinned Veritas core contracts/runtime
```

Hard rules:

- 3DMk SHALL consume canonical, versioned Veritas core contracts/runtime surfaces for migrated capabilities.
- Veritas SHALL NOT depend on 3DMk.
- 3DMk SHALL NOT clone, fork, or redefine a generic Veritas contract locally.
- 3DMk-specific extensions SHALL remain demonstrably geometry-specific.
- Tauri SHALL NOT be authoritative for geometry business logic.
- If Veritas does not yet expose a required generic contract, the contract SHALL be created and validated upstream before 3DMk consumes it.

## 3. Epistemic challenge gate

Before a proposed architectural mechanism becomes normative, apply:

> How much of this is speculative bullshit because we enjoy the riff?

Every non-trivial architectural claim SHALL be classified as:

- `ESTABLISHED` — directly evidenced or inherited from authoritative Veritas contracts;
- `VALIDATED` — tested against representative 3DMk requirements with reproducible evidence;
- `HYPOTHESIS` — plausible but insufficiently evidenced;
- `RIFF` — speculative and ineligible for normative architecture.

`HYPOTHESIS` and `RIFF` material SHALL NOT appear as required production architecture.

High confidence triggers additional disconfirmation: identify residual uncertainty, test the strongest countercase, exercise boundary/failure conditions, compare simpler alternatives, reproduce results, and state remaining error bounds. Promotion requires zero known unsupported assertion within declared domain-appropriate uncertainty.

## 4. Capability ownership

Veritas SHALL own canonical generic contracts for capability identity/versioning, typed execution envelopes, state/effect/precondition representation, planning/admission interfaces, provider/resource descriptions, provenance, checkpoints, generic evaluation/convergence, and intelligence escalation.

3DMk SHALL own geometry specializations including point-cloud and mesh states, VWM/GON/RANSAC-LP providers, geometry constraints/objectives/evaluators, geometry goals, derived evidence renderers, and geometry presentation/expert controls.

Representative domain capabilities include:

- `pointcloud.validate`
- `pointcloud.remove_invalid`
- `pointcloud.remove_outliers`
- `pointcloud.downsample`
- `geometry.estimate_normals`
- `geometry.register`
- `geometry.detect_planes`
- `geometry.detect_cylinders`
- `geometry.fit_primitive`
- `geometry.align_reference_frame`
- `vwm.interpret_scene`
- `mesh.reconstruct_surface`
- `mesh.repair`
- `mesh.validate`
- `geometry.compare`
- `geometry.measure_clearance`
- `export.glb`
- `export.ply`

Algorithms and accelerator implementations SHOULD remain providers behind stable capability identities unless expert mode intentionally exposes them.

## 5. Capability composition and deterministic authority

Capabilities SHALL compose through typed inputs/outputs, predicates, preconditions, postconditions, effects, and hard constraints. Pairwise successor lists SHALL NOT define the general graph.

Normal authority remains deterministic:

```text
typed goal
  → Veritas capability admission
  → independently validated execution plan/DAG
  → resource placement
  → candidate execution
  → geometry evaluation
  → incumbent comparison
  → explicit publish/finalize
```

Probabilistic intelligence MAY propose, classify, rank, summarize, or escalate; it SHALL NOT make illegal actions legal or publish authoritative state.

## 6. Candidate and best-state rule

Processing SHALL distinguish:

- `LatestState` — most recently generated candidate;
- `BestState` — strongest valid candidate under the active goal;
- `PresentedState` — candidate shown to the operator;
- `AuthoritativeState` — revision published through the authoritative transaction model.

A later turn SHALL NOT replace an earlier superior turn merely because it is newer.

All mutating capabilities SHALL follow:

```text
admit → stage → execute → validate → publish → finalize
```

## 7. Outcome evaluation

Geometry goals SHALL define hard constraints separately from optimization objectives.

Representative hard constraints include finite coordinates, topology validity where required, source-mapping integrity, coordinate-frame legality, maximum deviation, minimum coverage, and primitive preservation.

Representative objectives include fidelity, completeness, topology quality, primitive fit, structural consistency, runtime, and memory cost.

Candidate promotion requires measured evidence. Aggregate scores SHALL NOT conceal failed hard constraints.

## 8. Bounded convergence

Where iterative processing is enabled, search SHALL be bounded and preserve the incumbent. Initial production implementations SHALL prefer the simplest deterministic strategy that satisfies measured requirements.

Required properties:

- explicit iteration/time/compute/memory budgets;
- deterministic legality;
- regression rejection;
- incumbent persistence;
- cancellation safety;
- bounded plateau/oscillation handling where applicable;
- reproducible provenance.

Advanced search or learned ranking is not a requirement unless evidence proves simpler mechanisms insufficient.

## 9. Hardware-aware execution

Capabilities declare supported providers and resource requirements. Veritas owns generic hardware/runtime arbitration; 3DMk supplies geometry providers and domain-specific cost/parity evidence.

CUDA remains primary for eligible heavy workloads on supported NVIDIA hardware. Verified Vulkan/WebGPU fallback and CPU reference paths remain capability-specific.

Capabilities SHALL NOT independently seize accelerator resources outside the governing authority.

## 10. Intelligence escalation

The retained architecture supports a bounded competence ladder:

```text
deterministic semantic/context normalization
  → optional embedded narrow ML
  → optional local LLM/VLM/multimodal tier
  → optional approved OpenAI-compatible cloud tier
  → typed proposal
  → Veritas deterministic validation
```

The escalation contract belongs to Veritas. 3DMk supplies geometry context/evidence and typed domain goals.

The escalation packager SHALL be deterministic and MAY use embedded-model outputs to construct the next tier's constrained context. This specification does not require the embedded model itself to be generative.

All model tiers remain optional to deterministic core workflows. Exact models, adapters, learning methods, and providers remain non-normative until separately validated.

See `INTELLIGENCE_ESCALATION_SPEC.md`.

## 11. Standalone Tauri deployment

3DMk SHALL ALWAYS remain deployable as a Tauri 2 application.

```text
3DMk standalone package
  = pinned required Veritas core components
  + 3DMk geometry child domain
  + Tauri host
```

The standalone artifact SHALL NOT require a separately installed Veritas application, cloud service, or external intelligence service for deterministic core geometry workflows.

Core geometry/domain code SHALL remain independent of Tauri.

## 12. Upstream-first drift prevention

When 3DMk needs a new concept:

1. determine whether it is genuinely geometry-specific;
2. if generic, implement/version it upstream in Veritas;
3. validate and fingerprint it there;
4. consume the released contract/runtime surface from 3DMk;
5. never create a temporary local generic equivalent.

3DMk SHALL pin compatible Veritas versions/fingerprints rather than track mutable `main` semantics.

## 13. Known upstream compatibility gap

An approved 3DMK product direction is application-wide domain selection. The selected default domain SHALL be an application preference, and a later deployment profile MAY lock that selection. Domain definitions SHALL describe class and instance hierarchy, typed properties, embedded 3D asset references, and typed dimensional inputs. A validated domain feature configuration SHALL select workstation presentation only after intersection with registered capabilities and active policy. Generic domain-object and composition contracts SHALL be defined, versioned, and fingerprinted in Veritas before 3DMK implements the consuming adapter. Existing project objects SHALL retain explicit class identity so an application preference change cannot reinterpret their geometry or provenance. The proposed contract and acceptance sequence are specified in `docs/superpowers/specs/2026-09-22-configurable-domain-objects-design.md`; the exact schema shape remains a hypothesis until upstream validation.

Current 3DMk target requirements include in-process execution for lightweight embedded intelligence, while existing Veritas governance also contains processor-server lifecycle assumptions. 3DMk SHALL NOT locally override or fork that generic policy.

Before intelligence implementation begins, Veritas SHALL resolve the generic host contract so a child domain can use an approved in-process provider where justified while preserving Veritas resource governance. Until then, the escalation architecture is a target boundary, not implemented compatibility.

## 14. Migration rule

Existing behavior SHALL be characterized and wrapped incrementally. Wholesale rewrite is prohibited unless separately justified and approved.

Migration priority:

1. establish upstream Veritas contracts and exact compatibility pin;
2. map existing operations to domain capabilities;
3. migrate read-only operations first;
4. migrate authoritative mutations through candidate revisions;
5. centralize provider/resource selection;
6. add geometry evaluators;
7. prove latest/best-state separation;
8. enable narrowly bounded deterministic convergence only where justified;
9. add intelligence escalation plumbing only after Veritas contracts exist and deterministic core remains independent;
10. preserve Tauri standalone packaging throughout.

## 15. Architecture-complete gate

This architecture is not complete until:

- the Veritas parent relationship is mechanically enforced;
- no shadow generic contracts exist in 3DMk;
- migrated capabilities use canonical versioned contracts;
- the upstream host/provider contract supports the approved standalone topology;
- authoritative transaction rules remain intact;
- hardware placement is centrally governed;
- a worse later candidate cannot displace the incumbent;
- bounded convergence cannot run indefinitely;
- intelligence escalation fails closed and cannot bypass authority if enabled;
- provenance is complete;
- standalone Tauri packaging passes clean-machine/offline verification for deterministic core workflows;
- all applicable non-negotiable acceptance gates pass freshly;
- every normative architectural claim has passed the epistemic challenge gate.
