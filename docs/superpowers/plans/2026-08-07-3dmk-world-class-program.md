# 3DMk World-Class Delivery Program

> **For agentic workers:** Execute the authoritative revision workflow and CUDA foundation in dependency order. This program defines the cross-cutting quality, fault, performance, security, UX, and release obligations; it does not create a competing task ledger.

**Goal:** Convert the current working but split-authority 3DMk application into a production-grade, fault-contained, measurable, recoverable, offline CUDA-first engineering workstation.

**Primary implementation plans:**

- `2026-07-31-vwm-authoritative-revision-workflow.md`
- `2026-08-07-3dmk-foundation-batch.md`

**Governing standards:**

- `../specs/2026-08-07-3dmk-world-class-quality-standard.md`
- `../specs/2026-08-07-3dmk-cuda-first-system-design.md`
- `../../architecture/decisions/`

## 1. Program sequence

```text
Stage A — authoritative foundation
  Tasks 1–5
        ↓
Stage B — transactional/CUDA foundation
  FB1–FB8
        ↓
Stage C — reviewed CUDA-primary product workflows
  Tasks 6–14
        ↓
Stage D — frontend authority retirement and offline product hardening
  Tasks 15–16
        ↓
Stage E — world-class closure
  Task 17 plus complete release evidence
```

No stage may be skipped because a lower-level engine or prototype already exists. Existing code is reusable evidence and implementation material, not proof that the current contract is satisfied.

## 2. Stage A — authoritative foundation

Tasks 1–5 establish the minimum trustworthy data model before accelerator work can publish results.

Required outcomes:

- a commit-specific baseline and failure-reproducing tests;
- atomic operation, analysis, and revision publication contracts;
- revision read, compare, accept, and reject APIs;
- a Rust project session for every import path;
- stable source IDs and exact binary selection assets;
- optimistic project-generation concurrency;
- source bytes and root revision preserved immutably;
- frontend controls disabled until a valid project/revision session exists.

Exit evidence:

- intended red tests fail for real authority gaps rather than harness errors;
- implemented contracts pass focused and regression tests;
- package/export tests prove current limitations honestly;
- independent reviewer and verifier verdicts;
- exact ledger entries and pushed commits.

## 3. Stage B — transactional and CUDA foundation

Execute FB1–FB8 exactly as defined in the foundation plan.

Required outcomes:

- one SQLite metadata database per project;
- immutable SHA-256 asset payloads with durable publication;
- legacy JSON migration and round-trip evidence;
- accelerator contracts and deterministic GPU resource broker;
- live product CUDA inventory and known-answer kernel;
- supervised product CUDA worker;
- atomic compute-result publication;
- correlated telemetry, recovery, and redacted support bundle;
- Mistral.rs and product-worker coexistence under the measured VRAM budget.

Stage B is complete only when `CUDA_FOUNDATION_PROGRESS.md` is fully checked with live NVIDIA evidence.

## 4. Stage C — reviewed CUDA-primary workflows

Authoritative Tasks 6–14 consume the foundation. Each capability follows the same product pattern:

```text
accepted input revision
→ operation and job
→ broker lease
→ supervised execution
→ staged output and evidence
→ deterministic validation
→ candidate revision or analysis
→ compare / accept / reject
→ accepted active revision or retained rejection
```

### Structure and cleanup

- persist planes, components, structural relationships, exact source memberships, scores, and overlays;
- distinguish deterministic structure analysis from semantic object recognition;
- produce exact cleanup candidates rather than browser bounding-box deletion;
- retain Keep, Remove, and Uncertain decisions as reviewable evidence.

### Leveling and smoothing

- apply transforms and same-topology smoothing in Rust/CUDA;
- preserve source IDs, normals, colors, UVs, materials, textures, calibration, measurements, and provenance as permitted by topology;
- publish candidate revisions rather than mutating browser geometry;
- use CPU-reference differential tests and deterministic ordering.

### Mesh/point conversion

- area-weighted, seeded, deterministic sampling;
- explicit texture/color transfer and source-face mapping;
- exact attribute-transfer report;
- CUDA primary after parity and performance gates.

### Reconstruction

- prepare oriented points with source IDs and confidence;
- use brokered, chunked CUDA processing;
- treat triangle generation as an intermediate, not success;
- require residual, coverage, component, normal, attribute, manifold, and visual review gates;
- publish a candidate only when the result is structurally valid enough to review.

### Export

- explicit target revision or derived object;
- explicit format and options;
- Rust-owned serializers and product records;
- no viewport helpers, selection markers, clipping planes, or transient overlays unless explicitly requested as review assets;
- package round-trip preserves project/revision/operation/analysis/measurement/source-selection/accelerator identity.

### Measurements

- anchor to revision and source primitive identity;
- preserve units, uncertainty, labels, points, and review state;
- invalidate or remap through explicit topology-transfer evidence;
- remove browser undo claims that do not correspond to durable revisions.

### Perception and advisory AI

- packaged, licensed, hash-pinned model manifests;
- CUDA-backed production inference;
- RGBA, depth, and source-ID view buffers tied to exact cameras/revisions;
- multiview fusion and exact contributing source geometry;
- object candidates remain reviewable and reversible;
- Mistral.rs/VLM output remains advisory and cannot publish geometry.

## 5. Stage D — frontend, security, and offline product hardening

### Frontend authority

- split the monolithic frontend into focused ES modules without changing the Tauri/Axum boundary;
- one project-session store and one request/cancellation layer;
- viewport contains render caches only;
- remove browser-owned production transformations, package building, and geometry export;
- display accepted/candidate/comparison/transient states at all times;
- provide a conventional scene tree, properties/provenance inspector, jobs/leases surface, and compare/accept/reject flow;
- preserve keyboard operation, focus visibility, scalable layout, and readable status messages.

### Offline and security

- vendor all frontend and decoder assets;
- bind to loopback only;
- add a per-launch session secret, origin validation, restrictive Tauri capabilities, and restrictive CSP;
- package CUDA/native dependencies and validate them at startup;
- validate model, kernel, executable, and native-library hashes;
- produce SBOM and license inventory;
- stage, sign, verify, and rollback updates;
- prove network-denied clean-machine operation.

### Diagnostics and recovery UX

- exact hardware and dependency blockers;
- CUDA device, memory pressure, Mistral.rs, worker, broker, and lease state;
- measured stages/progress or honest indeterminate state;
- crash-recovery report and safe repair options;
- one-click redacted support bundle.

## 6. Stage E — release closure

Task 17 cannot pass until all release blockers in `WORLD_CLASS_ACCEPTANCE_MATRIX.md` pass with fresh evidence.

Required closure package:

```text
docs/status/world-class-completion.md
evidence/
├── command-log.jsonl
├── accelerator-inventory.json
├── cuda-known-answer.json
├── mistral-runtime.json
├── broker-invariants.json
├── fault-matrix.json
├── benchmark-results.json
├── soak-results.json
├── package-roundtrip.json
├── clean-machine.json
├── upgrade-rollback.json
├── support-bundle-manifest.json
└── screenshots/
```

Required tests:

- strict format, compile, lint, unit, integration, and regression gates;
- real vertical slice;
- process, worker, CUDA, OOM, cancellation, stale-generation, disk-full, corrupt-metadata, save, and power-interruption fault matrix;
- CUDA/CPU differential corpus and deterministic repeated runs;
- end-to-end benchmarks, not kernel-only benchmarks;
- Mistral.rs/rendering/product-worker coexistence;
- twelve-hour mixed-workload soak;
- clean-machine offline installation;
- update and forced rollback;
- support-bundle reconstruction of an injected fault;
- accessibility and capability-truth audit.

## 7. Performance gates

| Workload | Initial acceptance |
|---|---|
| One-million-element transforms/reductions | at least 5× CPU-reference throughput after transfers |
| Normals/filtering/sampling | at least 3× CPU-reference throughput |
| Heavy multi-stage pipeline | at least 2× retained CPU baseline |
| Sustained transfer share | no more than 25% of elapsed time unless declared I/O-bound |
| UI command acknowledgement | p95 below 100 ms |
| Cancellation | immediately non-publishable; resources reclaimed within the operation-specific bound |
| Reopen | no full reprocessing; incremental index/manifest loading |
| Save/export | streaming and bounded-memory; no full-package duplication |
| Soak | no unbounded RAM, VRAM, handle, queue, or temporary-file growth |

A kernel that is faster in isolation but worsens the complete workflow, determinism, memory pressure, or reliability fails.

## 8. Documentation and capability truth

For every completed task:

- update `docs/CURRENT_STATE.md` in the same change when implementation truth changes;
- update the appropriate progress ledger with actual evidence;
- update affected README, API, operational, and recovery documents;
- remove or rewrite any superseded claim;
- add no completion language unsupported by fresh commands;
- expose runtime status as `available`, `experimental`, `degraded`, or `unavailable` with dependency evidence and a reason.

Git history is the archive. The active tree contains only current instructions, accepted decisions, live plans, and fresh evidence.

## 9. Program completion

The program reaches `PROJECT_COMPLETE` only when:

- every authoritative task and FB1–FB8 is checked complete;
- the world-class acceptance matrix is fully satisfied;
- the application passes the complete supported workflow offline on the reference Windows/NVIDIA machine;
- foreseeable faults cannot silently lose or corrupt project data;
- every automated decision is reviewable and provenance-complete;
- accepted results are reproducible;
- unsupported capabilities are not advertised;
- the evidence package is complete, hashed, reviewed, and synchronized.
