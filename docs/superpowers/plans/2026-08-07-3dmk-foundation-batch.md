# 3DMk Foundation Batch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans`. This plan begins only after authoritative Tasks 1–5 pass.

**Goal:** Establish the transactional store, accelerator contracts, supervised CUDA runtime, resource broker, fault-safe publication path, and recovery evidence required by every later CUDA feature.

**Architecture:** One SQLite metadata database per project plus immutable CAS payloads; a Rust control plane that owns transactions; one supervised product CUDA worker; Mistral.rs as a separately supervised CUDA process; a deterministic broker using NVIDIA process/device observations; CPU reference implementations as verification oracles.

**Governing decisions:**

- `docs/architecture/decisions/ADR-001-transactional-project-store.md`
- `docs/architecture/decisions/ADR-002-supervised-cuda-worker.md`
- `docs/architecture/decisions/ADR-003-tiered-cache.md`

## Global constraints

- Do not begin this batch until authoritative Tasks 1–5 are checked complete with fresh evidence.
- No foundation task may bypass Rust revision authority or publish from a worker.
- Product CUDA work must fail closed without a verified CUDA device and known-answer result.
- Mistral.rs and product CUDA work share one brokered, measured VRAM budget.
- Normal CI must compile and test contract/fake paths without requiring CUDA libraries; live tests run on the supported NVIDIA Windows host.
- Every task uses TDD, independent review, independent verification, exact ledger evidence, and a bounded commit.

---

## Task FB1: Project database schema and migrations

**Files**

- Create: `src/project_db/mod.rs`
- Create: `src/project_db/migrations.rs`
- Create: `src/project_db/schema.rs`
- Create: `migrations/0001_project_schema.sql`
- Create: `tests/project_db.rs`
- Modify: `Cargo.toml`

**Interfaces**

```rust
pub struct ProjectDatabase {
    connection: std::sync::Mutex<rusqlite::Connection>,
}

impl ProjectDatabase {
    pub fn open(path: &std::path::Path) -> Result<Self, ProjectDbError>;
    pub fn migrate(&self) -> Result<MigrationReport, ProjectDbError>;
    pub fn integrity_check(&self) -> Result<IntegrityReport, ProjectDbError>;
    pub fn transaction<T>(
        &self,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, ProjectDbError>,
    ) -> Result<T, ProjectDbError>;
}
```

- [ ] Write a test opening an empty database and assert foreign keys, WAL, `synchronous=FULL`, busy timeout, and untrusted-schema protection.
- [ ] Run it and confirm failure because `ProjectDatabase` does not exist.
- [ ] Add a pinned SQLite dependency and the minimal `open` implementation.
- [ ] Write migration idempotency, checksum, unsupported-version, and interrupted-migration tests.
- [ ] Implement migrations in one exclusive transaction.
- [ ] Write foreign-key, uniqueness, JSON-validity, and state-transition constraint tests.
- [ ] Implement `PRAGMA quick_check` for routine startup and full `integrity_check` for diagnostics.
- [ ] Run `cargo test --test project_db -- --nocapture`.
- [ ] Commit: `feat: add transactional project database`.

## Task FB2: JSON-to-SQLite migration

**Files**

- Create: `src/project_db/import_legacy.rs`
- Create: `tests/project_migration.rs`
- Create: `fixtures/legacy-project/`

**Interface**

```rust
pub fn migrate_legacy_project(
    legacy_root: &std::path::Path,
    target_root: &std::path::Path,
) -> Result<ProjectMigrationReport, ProjectMigrationError>;
```

- [ ] Write a fixture containing a project, source asset, child revision, analysis, job, warnings, and package paths.
- [ ] Write the failing migration/reopen/round-trip test.
- [ ] Import into a new target directory without modifying the source.
- [ ] Verify every referenced payload length and SHA-256.
- [ ] Reopen through the new database API and compare identities and relationships.
- [ ] Export and re-import; assert project, revision, operation, analysis, and asset identity preservation.
- [ ] Inject failure after every migration phase; target remains absent or recoverable and source remains unchanged.
- [ ] Emit a durable migration report with warnings and hashes.
- [ ] Commit: `feat: migrate legacy projects transactionally`.

## Task FB3: Durable immutable asset publication

**Files**

- Create: `src/project_db/assets.rs`
- Create: `src/platform/durable_fs.rs`
- Create: `tests/asset_publication.rs`

**Interface**

```rust
pub fn publish_content_addressed(
    staging_file: &std::path::Path,
    asset_root: &std::path::Path,
) -> Result<PublishedAsset, DurableFsError>;
```

- [ ] Write tests for duplicate payload, hash mismatch, disk full, interrupted write, write-through rename failure, and an existing corrupt target.
- [ ] Stream the hash while writing staging bytes.
- [ ] Flush bytes and publish using a Windows write-through rename.
- [ ] Verify final length and digest after publication.
- [ ] Treat an identical existing target as success; quarantine conflicting content.
- [ ] Add a reachability scanner with a configurable safety grace period and no immediate destructive deletion.
- [ ] Verify package export streams reachable assets from a consistent database snapshot.
- [ ] Commit: `feat: publish immutable assets durably`.

## Task FB4: Accelerator contracts and deterministic GPU broker

**Files**

- Create: `VWM-Repo-Implicit/crates/vwm-accelerator-contracts/`
- Create: `VWM-Repo-Implicit/crates/vwm-gpu-runtime/`
- Modify: `VWM-Repo-Implicit/Cargo.toml`
- Test: serialization, invalid requests, deterministic scheduling, process reconciliation, and randomized invariants.

**Required contracts**

```rust
pub enum ComputeBackendKind {
    Cuda,
    Wgpu,
    CpuReference,
    CpuControlPlane,
}

pub struct GpuLeaseRequest {
    pub job_id: uuid::Uuid,
    pub workload: GpuWorkloadClass,
    pub minimum_bytes: u64,
    pub preferred_bytes: u64,
    pub stream_count: u16,
    pub preemptible: bool,
    pub deadline_ms: Option<u64>,
}

pub trait GpuResourceBroker: Send + Sync {
    fn snapshot(&self) -> GpuBudgetSnapshot;
    fn request(&self, request: GpuLeaseRequest) -> Result<GpuLease, LeaseError>;
    fn release(&self, lease_id: uuid::Uuid) -> Result<(), LeaseError>;
    fn reconcile(&self, observed: ObservedGpuMemory) -> ReconciliationReport;
}
```

- [ ] Write versioned serialization round-trip tests before implementation.
- [ ] Reject zero-byte leases, preferred bytes below minimum, zero streams, and unknown devices.
- [ ] Implement a simulation-only inventory source for normal CI.
- [ ] Reserve `max(512 MiB, 10% total VRAM)`.
- [ ] Use deterministic ordering: workload priority, aged wait time, request creation order.
- [ ] Test queue order, starvation aging, expiry, preemption eligibility, double release, and Mistral.rs process-memory reconciliation.
- [ ] Property-test that grants plus reserve never exceed observed available budget.
- [ ] Record stable reason codes for every grant, queue, downgrade, rejection, and recovery.
- [ ] Commit: `feat: add accelerator contracts and bounded GPU broker`.

## Task FB5: Product CUDA runtime and known-answer gate

**Files**

- Create: `VWM-Repo-Implicit/crates/vwm-cuda/`
- Create: `VWM-Repo-Implicit/crates/vwm-cuda/kernels/known_answer.cu`
- Create: `.github/workflows/cuda-acceptance.yml`

**Interface**

```rust
pub trait CudaRuntimeProbe: Send + Sync {
    fn inventory(&self) -> Result<CudaInventory, CudaRuntimeError>;
    fn run_known_answer(&self) -> Result<CudaSmokeEvidence, CudaRuntimeError>;
}
```

- [ ] Write fake-probe tests for no device, insufficient VRAM, duplicate UUID, unsupported capability, and wrong kernel output.
- [ ] Use dynamic loading so normal Windows CI can compile and run fake/contract tests without CUDA DLLs.
- [ ] Execute `out[i] = 2 * input[i] + 1` on the selected device.
- [ ] Verify output values, device UUID, driver/runtime versions, compute capability, memory, elapsed time, and kernel source/PTX hash.
- [ ] Fail closed on wrong result, missing CUDA, or device mismatch.
- [ ] Run live tests on the supported NVIDIA Windows machine or explicitly labeled self-hosted runner.
- [ ] Commit: `feat: add verified product CUDA runtime`.

## Task FB6: Supervised product CUDA compute worker

**Files**

- Create: `src/bin/3dmk-compute-worker.rs`
- Create: `src/compute_protocol.rs`
- Create: `src/compute_supervisor.rs`
- Create: `tests/compute_worker.rs`

- [ ] Write versioned protocol serialization, maximum-frame, malformed-frame, deadline, replay, and authentication-negative tests.
- [ ] Start the worker on a restricted authenticated Windows named pipe.
- [ ] Prevent the worker from opening the project database or authoritative project namespace.
- [ ] Add heartbeat, phase, lease, memory, and device evidence.
- [ ] Kill the worker during every phase and prove no project mutation.
- [ ] Add bounded exponential restart, a circuit breaker, and operator-visible diagnosis.
- [ ] Confirm one long-lived product CUDA context and bounded allocator/pool lifetime.
- [ ] Commit: `feat: supervise isolated CUDA worker`.

## Task FB7: Atomic compute-result publication

**Files**

- Modify: project database and durable-asset modules.
- Create: `tests/compute_publication.rs`.

- [ ] Produce a valid staged result through a fake worker.
- [ ] Validate geometry, attributes, mappings, quality reports, hashes, and expected project generation.
- [ ] Durably publish CAS payloads first.
- [ ] Commit asset, revision, operation, analysis, job, event, and project-generation changes in one SQLite transaction.
- [ ] Inject failure before and after every boundary.
- [ ] Verify no database reference points to an absent payload.
- [ ] Verify post-CAS/pre-database crashes create only safe unreachable assets.
- [ ] Make cancellation irrevocably prevent publication even when late worker output arrives.
- [ ] Commit: `feat: publish compute results atomically`.

## Task FB8: Telemetry, recovery, and support bundle

**Files**

- Create: `src/telemetry.rs`
- Create: `src/recovery.rs`
- Create: `src/support_bundle.rs`
- Create: `tests/recovery.rs`
- Create: `tests/support_bundle.rs`

- [ ] Add versioned JSONL events and bounded rotation.
- [ ] Correlate request, project, job, operation, revision, lease, worker, Mistral.rs, and CUDA events.
- [ ] Recover WAL state, abandoned jobs, staging directories, leases, worker state, and safe unreachable assets.
- [ ] Redact secrets, authorization headers, prompt content, user paths, and project payloads.
- [ ] Produce a deterministic support-bundle manifest containing hashes, diagnostics, crash records, capability inventory, and selected logs.
- [ ] Inject a failure and prove the support bundle reconstructs the event sequence.
- [ ] Commit: `feat: add recovery flight recorder`.

## Foundation exit gate

- SQLite migration, constraints, backup, and integrity tests pass.
- Durable asset-publication fault matrix passes.
- Broker contract and randomized invariant tests pass.
- Live CUDA known-answer test passes on the reference NVIDIA machine.
- Worker-kill and cancellation matrices publish nothing.
- Atomic result-publication failure matrix passes.
- Mistral.rs, rendering, and product worker coexist within the measured VRAM budget and safety reserve.
- Recovery returns RAM, pinned memory, VRAM, handles, leases, and staging state to a bounded baseline.
- A redacted support bundle reconstructs an injected failure.
- `CUDA_FOUNDATION_PROGRESS.md` contains exact commands, verdicts, commits, accelerator evidence, and residual limitations.
- No authoritative Task 6 work begins before this gate.
