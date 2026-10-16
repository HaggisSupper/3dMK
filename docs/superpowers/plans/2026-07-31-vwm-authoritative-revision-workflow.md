# 3DMk Authoritative Revision Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans`. Execute one independently reviewable task at a time. Update `docs/agent-execution/VWM_PROGRESS.md` only with observed evidence.

**Goal:** Make Rust project/revision state authoritative from every import through processing, review, save, reopen, and explicit export while preserving source evidence, attributes, measurements, provenance, and accelerator execution records.

**Architecture:** Tauri 2 and browser development use one Axum backend. Every editable scene is a Rust project with an immutable root revision. Analyses persist exact source evidence; geometry changes publish candidate child revisions; the user accepts or rejects candidates; the browser renders revision assets but never owns the document of record. The accepted CUDA foundation supplies transactional persistence, resource brokering, supervised execution, and fault-safe publication after Task 5.

**Current truth:** `docs/CURRENT_STATE.md`

**Governing standards:**

- `../specs/2026-08-07-3dmk-world-class-quality-standard.md`
- `../specs/2026-08-07-3dmk-cuda-first-system-design.md`
- `../../architecture/decisions/`

## Global constraints

- Windows 11 x64, PowerShell 7, Windows MSVC, Rust 2021 or newer.
- Tauri 2 remains the desktop shell and Axum remains the single application service.
- No Tauri-only processing fork and no React rewrite in this lane.
- Rust owns import, validation, project state, processing, persistence, revisioning, measurements, accelerator scheduling, and export.
- JavaScript owns rendering, input, bounded view/evidence capture, workflow presentation, and review interaction only.
- Original source bytes and the root measured revision are immutable.
- Every mutating request carries `expected_project_generation`; stale state fails with HTTP 409 and current-state evidence.
- A failed, cancelled, timed-out, stale-generation, out-of-memory, worker-death, device-loss, validation, or publication failure creates no authoritative output.
- Same-topology operations preserve source IDs and valid attributes exactly.
- Topology-changing operations emit an explicit source mapping, attribute-transfer report, and measured quality report.
- Automated findings remain proposals until accepted by a deterministic rule or the operator.
- AI/VLM output is advisory and cannot directly mutate or publish accepted geometry.
- No fake progress. Report a real stage and measured progress or report indeterminate.
- Existing useful viewer/import behavior remains available during migration unless a safer authoritative replacement lands in the same task.
- No Docker, Podman, WSL, Electron, cloud inference fallback, or Python production backend.

## Dependency sequence

```text
Tasks 1–5
    ↓
CUDA Foundation FB1–FB8
    ↓
Tasks 6–17
```

No Task 6 work begins until `CUDA_FOUNDATION_PROGRESS.md` is fully checked with independent review, verification, and live CUDA evidence.

## Core contracts

### Project generation

Every mutating API accepts:

```rust
pub struct ExpectedGeneration {
    pub expected_project_generation: u64,
}
```

A conflict response returns the current generation and active revision ID.

### Exact source selection

```rust
pub enum SourceDomain {
    Point,
    Vertex,
    Face,
    Primitive,
    Instance,
}

pub enum SelectionEncoding {
    RoaringBitmapV1,
}

pub struct SourceSelectionDescriptor {
    pub schema_version: u32,
    pub input_revision_id: uuid::Uuid,
    pub domain: SourceDomain,
    pub selected_count: u64,
    pub total_count: u64,
    pub encoding: SelectionEncoding,
    pub sha256: String,
}
```

Metadata is JSON; membership is a compact immutable binary asset.

### Attribute transfer

```rust
pub struct AttributeTransferReport {
    pub input_revision_id: uuid::Uuid,
    pub output_revision_id: uuid::Uuid,
    pub topology_relation: TopologyRelation,
    pub normals: TransferStatus,
    pub colors: TransferStatus,
    pub uvs: TransferStatus,
    pub material_ids: TransferStatus,
    pub textures: TransferStatus,
    pub source_ids: TransferStatus,
    pub camera_calibration: TransferStatus,
    pub measurements: TransferStatus,
    pub warnings: Vec<ProjectWarning>,
}
```

### Candidate response

```rust
pub struct CandidateRevisionResponse {
    pub project: Project,
    pub operation: OperationRecord,
    pub job: Option<JobRecord>,
    pub candidate_revision: Revision,
    pub render_asset: Asset,
    pub analysis_records: Vec<AnalysisRecord>,
    pub quality_report: Option<Asset>,
    pub attribute_transfer_report: AttributeTransferReport,
    pub source_mapping_asset: Option<Asset>,
}
```

---

## Task 1: Lock the baseline and add failure-reproducing tests

**Files**

- Create: `docs/status/authoritative-baseline.md`
- Create: `tests/export_roundtrip.rs`
- Create: `tests/frontend_contracts.rs`
- Create or update: `tests/frontend_shell.rs`
- Modify: `docs/agent-execution/VWM_PROGRESS.md`

**Acceptance**

- [ ] Record the exact base commit, current routes, state owners, persistence format, export paths, processing paths, capability truth, and known gaps.
- [ ] Add a real package export/re-import characterization test proving that current re-import creates new project/revision identity rather than rehydrating the authoritative graph.
- [ ] Add frontend contract tests proving visible browser edits can diverge from a committed Rust project and that viewport serialization remains a legacy export authority.
- [ ] Add source contracts preventing new claims that current browser undo, legacy routes, or CUDA architecture are production-complete.
- [ ] Run the tests and confirm each intended red test fails for the documented authority gap, not compilation, fixture, path, or harness errors.
- [ ] Do not change production behavior in this task merely to turn intended red evidence green.
- [ ] Record commands, exit codes, decisive failures, files, commit, review, verification, and residual risks.
- [ ] Commit: `test: lock authoritative workflow failures`.

## Task 2: Add atomic operation, analysis, and revision publication

**Files**

- Modify: `src/projects.rs`
- Modify: `src/jobs.rs`
- Modify: `src/api.rs`
- Create: `tests/operation_publication.rs`

**Acceptance**

- [ ] Add typed operation start/complete/fail/cancel records with expected generation, parameters, engine versions, seed, metrics, warnings, and error code.
- [ ] Add persisted analysis records with input revision, algorithm/version, findings, overlays, source mapping, and warnings.
- [ ] Add one publication boundary that creates staged assets, validates them, publishes a candidate revision/analysis, and updates project generation consistently.
- [ ] Reject missing parent revisions/assets, invalid topology contracts, stale generations, duplicate publication, and illegal state transitions.
- [ ] Prove failure and cancellation publish no revision or partial authoritative asset.
- [ ] Keep the current JSON store until FB1–FB3 migrate the implementation to SQLite/CAS transactions; APIs must be designed so the storage backend can change without caller changes.
- [ ] Commit: `feat: publish operations analyses and revisions atomically`.

## Task 3: Add revision read, compare, accept, and reject APIs

**Files**

- Modify: `src/projects.rs`
- Modify: `src/api.rs`
- Create: `tests/revision_api.rs`

**Routes**

```text
GET  /api/v1/projects/:project_id/revisions/:revision_id
GET  /api/v1/projects/:project_id/revisions/:revision_id/compare/:other_revision_id
POST /api/v1/projects/:project_id/revisions/:revision_id/accept
POST /api/v1/projects/:project_id/revisions/:revision_id/reject
```

**Acceptance**

- [ ] Return complete revision, asset, operation, analysis, quality, transfer, mapping, and accelerator references.
- [ ] Compare parent/candidate bounds, counts, attributes, mappings, quality metrics, and warnings without changing active state.
- [ ] Accept only a candidate from the current project generation and make it active in one publication transaction.
- [ ] Reject a candidate while retaining it and its evidence for provenance.
- [ ] Prevent accepting rejected, stale, foreign-project, missing, or structurally invalid revisions.
- [ ] Return typed conflict/error bodies and the current project state.
- [ ] Commit: `feat: add revision review APIs`.

## Task 4: Make every import create and load a Rust project session

**Files**

- Modify: `src/api.rs`
- Modify: `src/packages.rs`
- Modify: `src/projects.rs`
- Modify: `public/index.html` or its first extracted project-session module
- Create: `tests/import_session.rs`
- Update: `tests/frontend_contracts.rs`

**Acceptance**

- [ ] Direct GLB, GLTF, PLY, OBJ, STL, FBX, DAE, 3DS, 3MF, OFF, U3D, X3D, LAS, and supported package imports use Rust validation and create a project/root revision before processing is enabled.
- [ ] Preserve source package, model dependencies, textures, photos, calibration, reports, units, original names, package paths, hashes, and warnings.
- [ ] The frontend stores only `project_id`, `active_revision_id`, `generation`, bounded presentation metadata, and render-cache resources.
- [ ] Reopen loads the active revision from Rust rather than reconstructing state from a browser package.
- [ ] Ambiguous package primary-model selection remains an explicit review step.
- [ ] Unsupported or incomplete formats fail with truthful capability and dependency reasons.
- [ ] Commit: `feat: make imports authoritative project sessions`.

## Task 5: Introduce stable source IDs and exact selection assets

**Files**

- Modify: `VWM-Repo-Implicit/crates/vwm-core/src/scene.rs`
- Modify: relevant `vwm-io` converters
- Modify: `src/projects.rs`
- Create: `src/selections.rs`
- Create: `tests/source_identity.rs`
- Create: `tests/selection_assets.rs`

**Acceptance**

- [ ] Assign stable point, vertex, face, primitive, or instance IDs during canonical import.
- [ ] Preserve IDs through same-topology operations.
- [ ] Define versioned Roaring-bitmap selection payloads and descriptors.
- [ ] Validate selected count, total count, domain, input revision, encoding, byte length, and hash.
- [ ] Add deterministic gather/scatter utilities and exact source membership queries.
- [ ] Reject selections from another revision/domain or malformed payloads.
- [ ] Emit mappings/coverage metrics when topology changes.
- [ ] Commit: `feat: add stable source identity and selections`.

---

## Mandatory CUDA foundation gate

Execute every task in:

`2026-08-07-3dmk-foundation-batch.md`

Update:

`docs/agent-execution/CUDA_FOUNDATION_PROGRESS.md`

No later product task may substitute the executor's CUDA proof for product CUDA runtime, broker, worker, transactional publication, or recovery evidence.

---

## Task 6: Persist structure analysis as a project analysis

**Files**

- Modify: `src/scene.rs`
- Modify: `src/api.rs`
- Add CUDA/backend integration through the foundation contracts
- Create: `tests/structure_analysis.rs`
- Modify frontend analysis/overlay module

**Acceptance**

- [ ] Load the authoritative input revision by ID.
- [ ] Persist planes, components, structural relationships, scores, exact source selections, overlays, parameters, engine versions, and accelerator evidence.
- [ ] Separate deterministic structure categories from semantic recognition.
- [ ] Canonicalize parallel result ordering before hashing/persistence.
- [ ] Return a project analysis without mutating geometry.
- [ ] Display structural evidence as revision-bound toggleable overlays.
- [ ] CPU/CUDA differential fixtures meet declared tolerances and performance gates before CUDA status becomes available.
- [ ] Commit: `feat: persist structural analysis evidence`.

## Task 7: Implement exact cleanup candidate revisions

**Files**

- Modify: `src/scene.rs`
- Modify: `src/projects.rs`
- Modify: `src/api.rs`
- Create: `tests/cleanup_candidates.rs`
- Modify frontend cleanup/review modules

**Acceptance**

- [ ] Build Keep, Remove, and Uncertain exact selection assets from persisted structure analysis.
- [ ] Never use bounding boxes as membership.
- [ ] Create a staged filtered scene and candidate child revision without changing the accepted parent.
- [ ] Preserve all valid attributes and source IDs; emit an explicit transfer report.
- [ ] Display exact candidate overlays and allow operator edits to classifications before execution.
- [ ] Accept/reject through Task 3 APIs.
- [ ] Cancellation, OOM, worker failure, stale generation, and validation failure publish nothing.
- [ ] Commit: `feat: add exact cleanup candidate revisions`.

## Task 8: Add revision review UX and state transparency

**Files**

- Create focused frontend project-session, revision-review, comparison, and jobs modules
- Modify: `public/index.html`
- Create JavaScript unit/contract tests
- Update Rust API tests as required

**Acceptance**

- [ ] Always display project ID/name, generation, accepted active revision, viewport revision, and accepted/candidate/comparison/transient state.
- [ ] Provide parent/candidate side-by-side or overlay comparison with counts, bounds, attributes, quality, mappings, warnings, and backend evidence.
- [ ] Provide explicit Accept, Reject, Return to accepted, and Inspect evidence actions.
- [ ] Show real job stage, measured progress or indeterminate state, cancellation, lease wait, and blockers.
- [ ] Prevent processing or export when the viewport/session is stale or ambiguous.
- [ ] Preserve keyboard access, focus visibility, scalable layout, and clear destructive-action semantics.
- [ ] Commit: `feat: add authoritative revision review workspace`.

## Task 9: Revision-back leveling and smoothing

**Files**

- Modify Rust geometry/operation APIs
- Add CUDA-primary transforms and same-topology smoothing through foundation backends
- Create: `tests/leveling_revision.rs`
- Create: `tests/smoothing_revision.rs`
- Remove corresponding browser production mutations

**Acceptance**

- [ ] Leveling records the reviewed support plane, rigid transform, datum change, camera/calibration transform, and candidate revision.
- [ ] Smoothing preserves topology, stable source IDs, and all non-position attributes exactly.
- [ ] CPU/CUDA differential results meet declared tolerances and deterministic ordering.
- [ ] User parameters are versioned and included in operation provenance.
- [ ] No legacy upload of an ad-hoc browser PLY remains as the production path.
- [ ] Commit: `feat: revision-back leveling and smoothing`.

## Task 10: Revision-back mesh/point conversion

**Files**

- Add Rust/CUDA area-weighted sampling and conversion APIs
- Create: `tests/mesh_point_conversion.rs`
- Remove browser production `MeshSurfaceSampler` conversion path

**Acceptance**

- [ ] Point-cloud-to-mesh uses the reviewed reconstruction workflow rather than an unqualified route success.
- [ ] Mesh-to-point sampling is area-weighted, seeded, deterministic, chunked, and revision-backed.
- [ ] Preserve or explicitly transfer normals, colors, textures, UV-derived color, source-face IDs, confidence, units, and provenance.
- [ ] Emit source mapping and coverage metrics.
- [ ] Meet CPU/CUDA parity and end-to-end performance gates.
- [ ] Commit: `feat: revision-back mesh and point conversion`.

## Task 11: Revision-back reconstruction with measured quality gates

**Files**

- Integrate CUDA preparation/implicit backends
- Modify reconstruction API/job/worker code
- Create: `tests/reconstruction_quality.rs`
- Add representative real fixtures and benchmark definitions

**Acceptance**

- [ ] Prepare oriented points with stable source IDs, confidence, normals, units, and bounded chunking.
- [ ] Run reconstruction under a broker lease in the supervised worker.
- [ ] Measure residuals, source coverage, components, triangles, normals, degeneracy, boundaries/manifold indicators, attribute transfer, memory, transfers, and timings.
- [ ] Publish only a reviewable candidate with a quality report; generating triangles alone is not success.
- [ ] Compare candidate against source and parent in the UI.
- [ ] Cancellation/worker death/OOM/stale generation/validation failure publish nothing.
- [ ] Meet the declared end-to-end improvement gate on representative fixtures.
- [ ] Commit: `feat: add quality-gated reconstruction revisions`.

## Task 12: Build explicit Rust-owned export products

**Files**

- Add Rust export-product contracts and serializers
- Modify: `src/api.rs`
- Modify project/package export code
- Create: `tests/export_products.rs`
- Remove browser viewport serialization from authoritative export actions

**Routes**

```text
POST /api/v1/projects/:project_id/exports
GET  /api/v1/projects/:project_id/exports/:export_id
```

**Acceptance**

- [ ] Require explicit project, target revision/object, format, units, coordinate system, attributes, review-asset options, and expected generation.
- [ ] Produce explicit GLB, PLY, OBJ, STL, package, and approved drawing/report products through Rust-owned records.
- [ ] Exclude grid, lights, clipping planes, measurements, guides, selections, and overlays unless explicitly selected as review assets.
- [ ] Preserve attributes/mappings truthfully and report unsupported transfers.
- [ ] Stream output with bounded memory and durable publication.
- [ ] Package round-trip preserves project/revision/operation/analysis/measurement/selection/accelerator identity.
- [ ] Commit: `feat: add explicit authoritative export products`.

## Task 13: Persist measurements and remove incomplete browser undo claims

**Files**

- Add measurement-set and anchor contracts/storage/API
- Modify measurement frontend module
- Create: `tests/measurements.rs`
- Update frontend contract tests

**Acceptance**

- [ ] Persist mode, label, units, points, uncertainty, input revision, source primitive anchors, created/updated timestamps, and review state.
- [ ] Reopen and package round-trip measurements exactly.
- [ ] Same-topology operations preserve anchors; topology-changing operations remap or explicitly invalidate them through transfer evidence.
- [ ] Replace browser geometry undo/redo with accepted-revision navigation and candidate rejection where durable history is required.
- [ ] Keep transient camera/UI undo separate and labelled accurately.
- [ ] Commit: `feat: persist revision-aware measurements`.

## Task 14: Wire production perception, object records, source evidence, and advisory VLM adjudication

**Files**

- Extend `vwm-perception` production backend contracts
- Add packaged model manifests and license/hash validation
- Add view/depth/source-ID evidence APIs and object records
- Create: `tests/perception_pipeline.rs`
- Modify frontend perception/review modules

**Acceptance**

- [ ] Capture RGBA, depth, camera, and face/point-ID buffers tied to an exact revision and viewport configuration.
- [ ] Run packaged, approved, CUDA-backed segmentation/classification models through explicit ABI manifests.
- [ ] Fuse multiview proposals deterministically and retain contributing pixels/views/source selections.
- [ ] Persist object records, alternatives, confidence, model identity, evidence, and operator decision.
- [ ] VLM/Mistral.rs adjudication runs only under policy, returns strict typed output, and cannot mutate geometry.
- [ ] Accepted object extraction creates a candidate derived-object revision with exact source mapping.
- [ ] CPU inference is not a production fallback.
- [ ] Commit: `feat: add evidence-backed perception workflow`.

## Task 15: Modularize the frontend and retire browser-owned production paths

**Files**

- Split `public/index.html` into focused ES modules for API, session, revisions, scene, measurements, processing, jobs, accelerator state, and exports
- Add package-local JavaScript tests
- Update frontend shell/source-contract tests

**Acceptance**

- [ ] One typed request/cancellation/error layer and one project-session store.
- [ ] No browser production geometry transformation, package construction, or authoritative export remains.
- [ ] Bulk geometry stays in Rust assets/render buffers; IPC exchanges IDs, parameters, summaries, progress, and bounded evidence.
- [ ] Vendor or localize all runtime JavaScript, decoder, font, and style assets.
- [ ] Preserve current useful viewer interactions while exposing backend/revision truth.
- [ ] No inactive feature is enabled by route existence alone.
- [ ] Commit: `refactor: retire browser compute authority`.

## Task 16: Vendor assets and harden offline Tauri installation, security, diagnostics, upgrade, and rollback

**Files**

- Modify Tauri configuration/capabilities/bundling
- Add session/origin protection to Axum/Tauri startup
- Add hardware/dependency diagnostics
- Create clean-machine and upgrade/rollback PowerShell verification scripts
- Add operations/recovery documentation and SBOM/license generation

**Acceptance**

- [ ] Network-denied clean-machine install and launch on the reference NVIDIA system.
- [ ] No external CDN or remote runtime asset.
- [ ] Per-launch secret, origin validation, restrictive CSP, and least-privilege Tauri capabilities.
- [ ] Validate CUDA driver/runtime, native libraries, Mistral.rs, model, kernel, and executable hashes.
- [ ] Precise blocking diagnostics without exposing secrets.
- [ ] Signed, staged update and forced rollback preserve project readability.
- [ ] Complete SBOM and license inventory.
- [ ] Commit: `build: harden offline CUDA Tauri delivery`.

## Task 17: End-to-end acceptance, fault matrix, performance, soak, and migration closure

**Files**

- Create final acceptance/fault/benchmark/soak/clean-machine suites
- Create: `docs/status/world-class-completion.md`
- Create hashed evidence package
- Update all user, developer, recovery, API, capability, and limitations documentation

**Acceptance**

- [ ] All Tasks 1–16 and FB1–FB8 are checked complete with fresh independent evidence.
- [ ] Full root/VWM formatting, compilation, strict lint, unit, integration, regression, and documentation-validation gates pass.
- [ ] The real vertical slice passes: import → analyze → exact cleanup candidate → compare → accept → close → reopen → save → re-import → explicit GLB and PLY export.
- [ ] Fault matrix covers UI, service, database, disk, worker, Mistral.rs, CUDA, OOM, cancellation, stale generation, save, update, and power interruption without silent loss or partial publication.
- [ ] CUDA/CPU differential, deterministic repeated-run, memory, concurrency, and performance gates pass.
- [ ] Mistral.rs, rendering, and product compute coexist within the brokered reference budget.
- [ ] Twelve-hour mixed-workload soak shows bounded RAM, VRAM, handles, queues, leases, caches, and temporary files.
- [ ] Clean-machine offline install, update, rollback, support bundle, accessibility, capability truth, and documentation audit pass.
- [ ] Evidence contains exact commands, versions, fixtures, hashes, timings, device evidence, screenshots, limitations, and reviewer/verifier verdicts.
- [ ] Report `PROJECT_COMPLETE` only after every gate above passes freshly.
- [ ] Commit: `release: close authoritative 3DMk migration`.

## Definition of done

This workflow is complete only when the active tree contains no competing authority, every durable state transition is transactional and recoverable, every heavy production capability is CUDA-primary or explicitly excepted, every automated result is evidence-backed and reviewable, the complete product works offline on the supported system, and the full acceptance package proves it.
