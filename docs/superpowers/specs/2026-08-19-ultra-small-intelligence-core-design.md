# Ultra-Small Local Intelligence Subsystem Design

## Goal

Implement full Ultra Small App LLM capability parity as a first-class 3DMK subsystem: deterministic semantic classification, embedded/ML escalation, local Mistral.rs primary inference, local llama.cpp fallback, multimodal routing, explicit OpenAI-compatible cloud escalation, and a resource governor that prevents model work from destabilizing geometry processing or authoritative state.

## Product boundary

The subsystem is a decision-and-execution plane, not a geometry authority. It may interpret, classify, summarize, plan, and create reviewed operation candidates. Rust-owned project/revision transactions remain the only publication path.

## Capability ladder

1. **Deterministic semantic adapter** — typed operation classification, policy evaluation, input budgets, feature extraction, and evidence assembly.
2. **Embedded ML appliance** — deterministic/embedded classifiers and local perception models produce structured observations.
3. **Mistral.rs local runtime** — primary local text, coding, and multimodal-capable inference service on loopback with CUDA admission.
4. **llama.cpp local runtime** — explicit local fallback when Mistral.rs cannot satisfy a capability or resource contract.
5. **OpenAI-compatible cloud runtime** — final opt-in escalation only when the user has enabled the provider, the request policy permits external processing, and no protected project bytes are included without an explicit authorization record.

Each escalation receives a compact structured handoff from the preceding stage. Lower-stage observations become bounded evidence, not unrestricted prompt history.

## Resource governor

A single Rust `IntelligenceGovernor` owns admission for inference requests and cooperates with the future GPU broker.

- It applies per-request byte, pixel, token, wall-clock, queue, and concurrent-operation limits.
- It reserves a GPU safety margin for interactive rendering and geometry work.
- It rejects or defers work when memory, device health, cancellation, or generation state is unsafe.
- It records the selected tier, model/runtime identity, reason, limits, and outcome.
- It kills and quarantines failed runtime processes through a supervised lifecycle; partial output cannot publish.

No runtime may allocate opportunistically outside this governor.

## Immutable concrete contracts

Versioned JSON contracts define:

- `local-intelligence-routing.v1.json` — operation vocabulary, ladder, limits, fallback rules, and publication invariant;
- `local-intelligence-runtime.v1.json` — runtime identity, loopback/network policy, CUDA requirement, health/readiness protocol, and process ownership;
- `local-intelligence-cloud-egress.v1.json` — provider opt-in, permitted data classes, redaction, authorization record, and audit requirements.

Rust concrete types mirror every dynamic contract at ingress. Unknown enum values, undeclared models, missing limits, incompatible contract versions, missing CUDA evidence, or unauthorised egress block.

## Required behavior

- Mistral.rs is the primary local runtime.
- llama.cpp is an explicit fallback—not a silent substitution—and is selected only by the governor with a recorded reason.
- CUDA is mandatory for local LLM/VLM execution; CPU inference is rejected.
- Vulkan/WebGPU remains a capability-specific non-inference fallback only where separately verified.
- Cloud escalation is disabled by default and requires explicit provider configuration plus request-level egress authorization.
- AI output is advisory. It cannot mutate projects, revisions, exports, or authoritative geometry directly.
- All routes support cancellation and stale-generation rejection.
- The UI receives bounded status/evidence only; it never receives or controls secrets or raw runtime process arguments.

## Delivery slices

### Slice 1 — contracts and deterministic adapter

Typed operations, policy and budget evaluation, machine-readable contracts, CLI/API inspection route, contract validators, negative tests.

### Slice 2 — governor and supervised local runtimes

Mistral.rs and llama.cpp adapters, loopback health/readiness, CUDA admission, timeout/cancellation, explicit fallback, runtime evidence, process cleanup.

### Slice 3 — multimodal and candidate operations

Structured image/document observations, bounded multimodal handoff, candidate-operation creation, provenance, and explicit review gates.

### Slice 4 — explicit cloud escalation

OpenAI-compatible provider adapter, disabled-by-default configuration, authorization records, redaction/data classification, audit and failure behavior.

### Slice 5 — workstation integration and resilience

Tauri/Axum status surface, queue/cancellation controls, GPU broker integration, memory-pressure and device-loss tests, soak tests, and offline/deployment validation.

## Acceptance criteria

Feature parity is complete only when all five slices have fresh unit, integration, negative, regression, cancellation, memory-pressure, device-loss, and cross-runtime evidence; Mistral.rs primary and llama.cpp fallback are proven on CUDA; cloud egress remains disabled by default and auditable when enabled; and no AI path can publish authoritative state.
