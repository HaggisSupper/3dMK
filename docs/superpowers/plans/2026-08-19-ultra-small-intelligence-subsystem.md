# Ultra-Small Intelligence Subsystem Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver full Ultra Small App LLM feature parity inside 3DMK without allowing inference to destabilize GPU geometry work or publish authoritative state.

**Architecture:** The subsystem is a Rust-owned intelligence plane. A deterministic adapter and governor select an explicit tier, local runtime adapters execute only after CUDA/resource admission, and all outputs remain versioned advisory candidates. Tauri/Axum exposes bounded status and cancellation surfaces; the project/revision transaction remains the only publication authority.

**Tech Stack:** Rust 2021, Tokio, Axum, Serde, Mistral.rs CLI/OpenAI-compatible loopback, llama.cpp server/OpenAI-compatible loopback, CUDA, Tauri 2.

**Spec:** `docs/superpowers/specs/2026-08-19-ultra-small-intelligence-core-design.md`

## Global constraints

- 3DMK is a Veritas child repository.
- Rust owns contracts, policy, resource admission, process supervision, audit, and durable state.
- Mistral.rs is the primary local runtime; llama.cpp is only an explicit recorded fallback.
- Local inference requires CUDA evidence; CPU inference is rejected.
- Cloud egress is disabled by default and requires provider configuration plus request-level authorization.
- No AI output publishes authoritative project, revision, asset, export, or geometry state.
- No Docker, Podman, WSL, Electron, or Python production backend.
- Every external boundary uses a versioned machine-readable contract and typed ingress.
- Every task includes positive, negative, regression, cancellation, and evidence checks appropriate to its boundary.

---

### Task 1: Immutable contracts and typed deterministic adapter

**Files:**
- Create: `contracts/local-intelligence-routing.v1.json`
- Create: `contracts/local-intelligence-runtime.v1.json`
- Create: `contracts/local-intelligence-cloud-egress.v1.json`
- Create: `src/local_intelligence/mod.rs`
- Create: `src/local_intelligence/contracts.rs`
- Create: `src/local_intelligence/router.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces: `IntelligenceRequest`, `IntelligenceDecision`, `IntelligenceTier`, `route_request(&IntelligenceRequest) -> IntelligenceDecision`.
- Produces: exact operation, budget, advisory-only, and fallback-reason types.

- [ ] Write failing tests for deterministic routing, Mistral.rs routing, oversized-input blocking, authoritative-state advisory marking, and no automatic llama.cpp fallback.
- [ ] Run focused tests and confirm each fails for its intended missing behavior.
- [ ] Implement contract-mirroring types and pure deterministic routing.
- [ ] Run focused and regression tests.
- [ ] Add a contract validator and negative fixtures for version, operation, and no-silent-fallback drift.
- [ ] Commit the typed routing boundary.

### Task 2: Governor, queue, cancellation, and evidence

**Files:**
- Create: `src/local_intelligence/governor.rs`
- Create: `src/local_intelligence/evidence.rs`
- Create: `src/local_intelligence/cancellation.rs`
- Modify: `src/local_intelligence/mod.rs`

**Interfaces:**
- Produces: `IntelligenceGovernor::admit`, `IntelligenceLease`, `IntelligenceEvidence`, `CancellationToken`.
- Consumes: typed routing decisions, capacity snapshot, request generation.

- [ ] Write failing tests for concurrency cap, queue cap, cancelled request, stale generation, exhausted budget, and lease release.
- [ ] Implement bounded admission and RAII lease release without global mutable state.
- [ ] Prove cancellation/staleness prevents execution publication.
- [ ] Record deterministic evidence for every admission outcome.
- [ ] Commit governor and safety boundary.

### Task 3: Mistral.rs and llama.cpp supervised adapters

**Files:**
- Create: `src/local_intelligence/runtime.rs`
- Create: `src/local_intelligence/mistralrs.rs`
- Create: `src/local_intelligence/llamacpp.rs`
- Create: `scripts/setup-local-intelligence-runtimes.ps1`
- Create: `scripts/test-local-intelligence-runtimes.ps1`

**Interfaces:**
- Produces: `RuntimeAdapter::health`, `RuntimeAdapter::invoke`, `RuntimeSupervisor::start`, `RuntimeSupervisor::stop`.
- Consumes: admitted lease, runtime contract, CUDA evidence, loopback endpoint.

- [ ] Write failing adapter tests for non-loopback rejection, missing CUDA evidence, timeout, failed primary, explicit llama.cpp fallback, and child-process cleanup.
- [ ] Implement Mistral.rs primary adapter with loopback readiness and CUDA process verification.
- [ ] Implement llama.cpp adapter behind an explicit fallback decision.
- [ ] Implement supervisor ownership, timeout, cancellation, and cleanup.
- [ ] Run Windows runtime harness with a small CUDA model profile and record process evidence.
- [ ] Commit supervised local runtimes.

### Task 4: Multimodal candidates and provenance

**Files:**
- Create: `src/local_intelligence/multimodal.rs`
- Create: `src/local_intelligence/candidate.rs`
- Modify: `src/ai_vision.rs`
- Modify: `src/perception.rs`

**Interfaces:**
- Produces: `ObservationCandidate`, `CandidateProvenance`, `CandidateDisposition`.
- Consumes: bounded image/document evidence and runtime output.

- [ ] Write failing tests for image/pixel limit rejection, malformed model JSON, candidate provenance, and publication denial.
- [ ] Implement bounded multimodal handoff and structured response validation.
- [ ] Route existing VLM usage through the governor and advisory candidate boundary.
- [ ] Run perception regression tests and verify no direct authoritative mutation exists.
- [ ] Commit multimodal integration.

### Task 5: Explicit OpenAI-compatible cloud escalation

**Files:**
- Create: `src/local_intelligence/cloud.rs`
- Create: `src/local_intelligence/redaction.rs`
- Modify: `src/local_intelligence/runtime.rs`
- Modify: `contracts/local-intelligence-cloud-egress.v1.json`

**Interfaces:**
- Produces: `CloudAuthorization`, `RedactionReport`, `CloudEgressDecision`.
- Consumes: explicit provider configuration and request-level authorization.

- [ ] Write failing tests for disabled-by-default behavior, missing authorization, protected-data rejection, endpoint validation, redaction, and audit evidence.
- [ ] Implement typed egress policy and OpenAI-compatible adapter.
- [ ] Ensure secrets remain server-owned and responses are advisory candidates.
- [ ] Verify no network request occurs for blocked or unauthorized egress.
- [ ] Commit cloud escalation.

### Task 6: Axum/Tauri surfaces, GPU broker integration, and full verification

**Files:**
- Modify: `src/api.rs`
- Modify: `src-tauri/src/lib.rs` or the active Tauri command module
- Create: `src/local_intelligence/status.rs`
- Create: `scripts/test-local-intelligence-integration.ps1`
- Modify: `docs/CURRENT_STATE.md`

**Interfaces:**
- Produces: typed route/status/cancel endpoints and bounded UI DTOs.
- Consumes: governor, supervisor, candidate state, GPU broker.

- [ ] Write failing integration tests for status, cancellation, non-publication, concurrent geometry/inference pressure, memory pressure, and device loss.
- [ ] Implement loopback-only API/Tauri commands with no secret exposure.
- [ ] Integrate GPU broker reservations and release on all exit paths.
- [ ] Run unit, integration, regression, CUDA, device-loss, soak, and clean-machine verification.
- [ ] Run independent review and verifier gates; repair every blocking finding.
- [ ] Update contracts, runbooks, current-state truth, rollback procedures, and PR evidence.
- [ ] Commit and publish only after all gates pass.
