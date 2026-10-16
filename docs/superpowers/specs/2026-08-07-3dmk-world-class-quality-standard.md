# 3DMk World-Class Product and Reliability Standard

**Status:** Governing quality amendment  
**Applies to:** The complete 3DMk system: Tauri 2 shell, Axum service, Rust/VWM engines, CUDA compute plane, Mistral.rs, project/revision storage, UI, export, installation, upgrade and diagnostics.  
**Authority:** Supplements the CUDA-first architecture and the authoritative 17-task revision workflow. Where a weaker criterion exists, this standard governs.

## 1. Product definition

3DMk is not considered operational merely because it launches or displays geometry. It is operational only when it can ingest real source data, preserve the source immutably, perform reviewed processing through authoritative Rust revisions, recover from faults without ambiguous state, reopen the project, and export explicit products with complete provenance.

“Best in class” is defined by measured behavior:

1. No silent data loss.
2. No false capability claims.
3. No partial revision publication.
4. Deterministic, explainable processing where the algorithm permits determinism.
5. Bounded and observable use of GPU, CPU, RAM, VRAM and storage.
6. Recovery from application, worker, CUDA and power-interruption faults.
7. Explicit, reversible user decisions.
8. A coherent workstation UX rather than an exposed collection of engine endpoints.
9. Offline operation after installation.
10. Reproducible evidence sufficient to diagnose every failed operation.

## 2. Non-negotiable system requirements

- Windows 11 x64.
- Rust 2021 or newer, with Rust as the production control and processing language.
- Tauri 2 as the application deployment shell.
- One Axum service for browser-development and Tauri runtime surfaces.
- Supported NVIDIA CUDA-capable GPU; 6 GiB usable VRAM minimum.
- CUDA-enabled Mistral.rs; no CPU LLM fallback.
- 32 GiB system RAM minimum; 48 GiB reference validation system.
- No Docker, Podman, WSL, Electron or Python production backend.
- WebGPU/Vulkan may be a verified operation-level fallback, never a substitute for system CUDA compliance.
- Bulk geometry never crosses Tauri IPC.

## 3. Reliability objectives

### 3.1 Data durability

- Source bytes and the root measured revision are immutable.
- Every durable write is atomic or recoverable through a journal.
- Package export writes to a temporary path, validates the archive, flushes it, then atomically publishes it.
- Metadata references cannot point to absent assets.
- Content-addressed assets are verified by length and SHA-256 on write, reopen, export and import.
- Project generation implements optimistic concurrency. Stale actions receive a conflict response and cannot overwrite newer state.

### 3.2 Operation publication

Every processing operation has six explicit phases:

```text
admit
→ stage
→ execute
→ validate
→ publish
→ finalize
```

Before `publish`, no output is authoritative. Cancellation, timeout, process death, CUDA error, validation failure or stale generation prevents publication. Recovery removes or quarantines abandoned staging state.

### 3.3 Recovery targets

| Fault | Required behavior |
|---|---|
| UI/webview crash | Rust service remains consistent; reopening rehydrates the authoritative active revision |
| Axum/Tauri process crash | Restart detects and recovers journals and abandoned jobs |
| Worker crash | Job becomes failed/interrupted; no partial revision exists |
| CUDA OOM | Lease is revoked, staged outputs are discarded, memory returns to baseline, user receives exact diagnosis |
| CUDA context/device loss | Current GPU work is invalidated; no output publishes; restart path is explicit |
| Mistral.rs crash | Advisory AI becomes unavailable; authoritative project operations remain consistent |
| Power interruption during save | Original project/package remains valid; temporary or journal files are recoverable |
| Disk full | The operation fails before publication and reports required versus available bytes |
| Corrupt project metadata | Open fails closed with a repair report; source assets are not deleted |
| Stale browser tab | Mutation returns HTTP 409 with current generation and active revision |

## 4. Authoritative storage architecture

- One SQLite metadata database per project is the transactional source of truth.
- SQLite runs locally with foreign keys, WAL, `synchronous=FULL`, a busy timeout and untrusted-schema protections.
- Large source and derived payloads remain immutable SHA-256 content-addressed files.
- Payloads are durably published before one database transaction creates references to them.
- A crash may create a safe unreachable payload; it may not create a metadata reference to an absent payload.
- Online backups create consistent project snapshots for package export.
- The current JSON store is a migration source, not the final reliability architecture.

## 5. CUDA fault domain

- Product CUDA work executes in a supervised worker process.
- The worker cannot mutate project metadata and writes staged outputs only.
- One long-lived product CUDA context avoids repeated context overhead and isolates native faults from the control plane.
- Mistral.rs remains separately supervised.
- The control plane alone validates and publishes results.

## 6. GPU resource governance

A Rust-owned resource broker is mandatory. It owns admission, priority, measured reservations and reclamation for:

- Mistral.rs weights and KV cache;
- viewport-interactive requirements;
- geometry and point-cloud jobs;
- CUDA perception and image jobs;
- reconstruction;
- export preparation.

Rules:

- Reserve `max(512 MiB, 10% total VRAM)` as safety headroom.
- Admit against observed free memory, not theoretical capacity.
- A lease declares minimum/preferred bytes, workload class, stream count, preemptibility and deadline.
- Interactive work outranks batch work.
- Queueing is preferred to optimistic allocation.
- Mistral.rs uses a versioned model profile with explicit context, quantization, paged-attention and device-map budgets.
- A smaller CUDA model is preferable to a larger model that starves product compute.
- Every lease and allocation high-water mark is recorded.
- Soak tests must prove memory returns to a stable baseline.

## 7. Determinism and numerical integrity

- All stochastic operations require an explicit persisted seed.
- Parallel results are canonicalized before hashing or persistence.
- Stable source IDs survive same-topology operations exactly.
- Topology-changing operations emit a source-mapping and attribute-transfer report.
- CPU reference implementations remain available for deterministic fixtures and differential tests.
- Float tolerances are operation-specific, versioned and included in quality reports.
- Project transforms, datum, units and measurement metadata use `f64`.
- Local-origin `f32` geometry is permitted only when error is below a declared tolerance.
- Reduced precision on CUDA is opt-in by operation contract and recorded in provenance.

## 8. Fault containment

Fault domains are separated:

```text
Tauri/webview
Axum control plane
project store
job supervisor
CUDA worker/context
Mistral.rs process
package/export worker
```

A fault in one domain may reduce capability but cannot silently corrupt another domain. Heavy or cancellation-sensitive CUDA operations execute in supervised workers when an in-process context would make recovery unsafe.

## 9. Observability

The system emits structured JSONL events with:

- correlation ID;
- project, operation and job IDs;
- source and target revision IDs;
- accelerator backend/device UUID;
- model/kernel/engine versions;
- state transition;
- queue/transfer/kernel/validation/publish timings;
- host, RAM, VRAM and disk high-water marks;
- transferred bytes and copy count;
- warning/error codes;
- fallback decision and reason;
- cancellation source;
- recovery action.

A rotating local flight recorder retains enough history to reconstruct the last failed workflow. A support bundle redacts secrets and packages logs, configuration, manifests, capability inventory, crash reports and relevant hashes.

## 10. Security and supply chain

- Loopback services bind to `127.0.0.1` only.
- Tauri/Axum use a per-launch session secret and origin validation.
- Model, executable, kernel and native-library artifacts have pinned identities and SHA-256 evidence.
- Release builds produce an SBOM and dependency/license inventory.
- ZIP and package ingestion remains bounded against traversal, links, compression bombs and malformed entry metadata.
- External model or plugin content cannot execute arbitrary code.
- Update packages are signed, staged, verified and rollback-capable.
- Secrets never enter project files, prompts, logs or support bundles.

## 11. UX standard

The UI is a workstation, not a route tester. It must expose:

- project/revision status at all times;
- accepted, candidate, comparison and transient-overlay states;
- scene tree with stable identities;
- properties and provenance inspector;
- job queue, stage, measured progress and cancellation;
- CUDA device, VRAM pressure and Mistral.rs status;
- capability truth with reason codes;
- clear compare/accept/reject workflow;
- explicit save-project versus export-product actions;
- crash-recovery and unsaved-state indicators;
- keyboard operation, accessible focus and scalable layout;
- no destructive action without preview or recoverable revision.

## 12. Performance objectives

All gates include complete end-to-end time, not kernel-only timing.

| Workload | Initial acceptance |
|---|---|
| One-million-element transforms/reductions | ≥5× CPU-reference throughput after transfers |
| Normals/filtering/sampling | ≥3× CPU-reference throughput |
| Heavy multi-stage pipeline | ≥2× retained CPU baseline |
| Transfer share | ≤25% sustained-pipeline elapsed time unless declared I/O-bound |
| UI interaction | p95 command acknowledgement <100 ms |
| Job cancellation | immediately non-publishable; bounded resource reclamation |
| Project reopen | no full reprocessing; load manifests/indexes incrementally |
| Save/export | streaming, bounded memory; no whole-package duplication |
| Soak | no unbounded RAM/VRAM/handle/temp-file growth |

A faster kernel that worsens end-to-end latency, reliability or memory pressure does not pass.

## 13. Release acceptance

A release candidate must pass:

1. Clean-machine offline installation.
2. CUDA inventory and known-answer kernel.
3. CUDA-backed Mistral.rs live-process evidence.
4. Import → analyze → exact cleanup candidate → compare → accept → close → reopen → save → re-import → explicit GLB and PLY export.
5. Crash/cancellation/OOM/disk-full fault matrix.
6. Twelve-hour mixed-workload soak on the reference system.
7. Package round-trip identity and digest checks.
8. No external CDN dependency.
9. Upgrade and rollback test.
10. Complete support bundle and diagnostics.
11. Strict formatting, lint, unit, integration and regression gates.
12. Written limitations; no unsupported capability represented as available.

## 14. Definition of exceptional

The product is exceptional when a knowledgeable operator can trust it with valuable scan data, understand every automated decision, recover from foreseeable faults, reproduce accepted results, diagnose failures without guesswork, and complete the principal workflow faster and with less friction than competing technical viewers and fragmented processing toolchains.
