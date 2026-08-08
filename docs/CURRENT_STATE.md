# 3DMk Current Implementation State

**Audit date:** 2026-08-07  
**Audited runtime/product baseline:** source behavior through PR #10 at `e42cb081e0d940c686ed8c6309ad339e51f55e9d`; subsequent merged changes are documentation-only  
**Branch alignment:** `agent/vwm-authoritative-revision-implementation` is synchronized with current `main`  
**Product state:** active implementation; not deployment-ready; `PROJECT_COMPLETE` has not been reached

## 1. What 3DMk is now

3DMk is a Windows-first local 3D engineering workstation composed of:

- a Tauri 2 desktop shell;
- one embedded Axum service bound to an ephemeral loopback port;
- a single-page Three.js workstation UI;
- a Rust CLI and reusable Rust backend library;
- a filesystem-backed project, asset, revision, job, and package subsystem;
- the canonical `VWM-Repo-Implicit` Rust workspace for I/O, geometry, perception, and implicit reconstruction.

The application is local-first. Cloud accounts and hosted processing are outside the current product.

## 2. Implemented runtime surfaces

### Desktop shell

Implemented:

- Tauri 2 starts the same Axum router used by browser development.
- The backend binds to `127.0.0.1` on an available port.
- The current `public/index.html` is embedded and written to the application data directory.
- A startup failure is written to `3DMk-startup.log` in the system temporary directory.

Current limitation:

- release-grade offline asset vendoring, per-launch session protection, installer validation, upgrade/rollback, and clean-machine acceptance are not complete.

### CLI

Implemented commands:

```text
agentic-cad-backend pdf-to3d
agentic-cad-backend poisson-reconstruct
agentic-cad-backend serve
```

The CLI can extrude a detected floorplan boundary to STEP, reconstruct a point cloud through PDAL when present or the Rust implicit fallback, and start the browser-development server.

### Axum API

Implemented v1 surfaces include:

- capability inventory;
- project create/list/read;
- direct model import;
- safe ZIP inspection and package import;
- asset listing and streaming;
- project package export;
- revision listing and active-revision switching;
- job read and cancellation.

Legacy processing routes remain for floorplan extrusion, point-cloud reconstruction and analysis, VWM perception, rigid leveling, smoothing, image assessment/refinement, and model floorplan extraction. Their existence does not establish production readiness; the capability response reports them as available, experimental, degraded, or unavailable with reasons.

### Project and package layer

Implemented:

- immutable source asset storage keyed by SHA-256;
- projects, root revisions, child revisions, operations, analyses, jobs, and active-revision state;
- bounded upload streaming;
- bounded ZIP inspection against unsafe paths, links, unsupported compression, entry limits, expanded-size limits, and compression-ratio abuse;
- package asset classification and digest verification;
- Rust-owned package export.

Current limitation:

- authoritative metadata is still stored as recoverable JSON files rather than the accepted SQLite transaction model;
- the full operation publication, review, accept/reject, generation-concurrency, and recovery contracts are not implemented.

### Viewer and workstation UI

Implemented:

- mesh and point-cloud loading for the browser-supported formats;
- ZIP package inspection and committed Rust package loading;
- scene tree, perspective/orthographic cameras, orbit controls, render modes, clipping, lighting, point-size and stride controls;
- textures, blueprint/reference images, measurement objects, crop UI, room-plan overlays, structure/fragment overlays, context help, and export controls;
- browser-side undo/redo for transient geometry states;
- raw GLB, OBJ, PLY, and STL serialization plus browser-built packages and drawing exports.

Current limitations:

- `loadedGeometryStore`, browser undo stacks, overlays, measurements, and browser-built ZIP output can diverge from the Rust project record;
- post-import geometry operations are not consistently revision-backed;
- browser viewport traversal is still used by legacy export paths;
- the single-file frontend is not yet modularized and release assets are not fully offline-vendored.

## 3. VWM engine state

### Implemented reusable crates

- `vwm-core` — canonical scene and shared contracts.
- `vwm-io` — validated model conversion to canonical scenes.
- `vwm-geometry` — structural and geometric analysis primitives.
- `vwm-implicit-core`, `vwm-implicit-poisson`, `vwm-implicit-surface-nets`, `vwm-implicit` — CPU/Rust implicit reconstruction stack.
- `vwm-perception` — image validation, deterministic region/shape fallbacks, current tract-based ONNX paths, projection/source-slicing contracts, and optional VLM adjudication hooks.

### Not yet production-integrated

- exact persisted source selections for cleanup and perception;
- revision-backed structure analysis, cleanup, smoothing, leveling, conversion, reconstruction, image projection, and object extraction;
- packaged licensed ONNX models and production multiview evidence flow;
- CUDA-primary VWM kernels and ONNX Runtime CUDA execution;
- quality-gated reconstruction acceptance.

## 4. CUDA and Mistral.rs truth

Accepted system requirement:

- the complete target system requires a supported NVIDIA CUDA-capable GPU;
- Mistral.rs must be CUDA-backed; CPU LLM inference is prohibited;
- heavy product compute is CUDA-primary once each capability passes parity, fault, memory, and performance gates.

Implemented now:

- a CUDA-only local Mistral.rs development-agent setup/controller;
- source-build provenance and executable-hash checking;
- local server CUDA-process observation through NVIDIA telemetry;
- separate authoritative Task 1–17 and CUDA Foundation FB1–FB8 selectors;
- fail-closed predecessor admission before Mistral.rs setup or model loading;
- selected plan, ledger, report, evidence, session, and pull-request routing for both task families;
- static Windows validation of executor and documentation contracts.

Not implemented in the product runtime:

- Tauri/Axum ownership of the Mistral.rs lifecycle;
- product CUDA inventory and known-answer kernel;
- the shared GPU resource broker;
- the supervised product CUDA worker;
- pinned staging/device cache infrastructure;
- CUDA geometry, reconstruction, image, and perception kernels;
- product-visible accelerator APIs and UI.

Therefore, the repository has an accepted CUDA-first architecture and a dependency-aware CUDA-only development executor, but it does not yet contain a complete CUDA-first product implementation.

## 5. Active implementation sequence

```text
Authoritative Tasks 1–5
        ↓
CUDA Foundation FB1–FB8
        ↓
Authoritative Tasks 6–14 using CUDA-primary backends
        ↓
Tasks 15–16 frontend, offline packaging, security, and deployment
        ↓
Task 17 end-to-end fault, performance, soak, and clean-machine closure
```

The controller enforces this sequence from the checked task entries in:

- `agent-execution/VWM_PROGRESS.md`;
- `agent-execution/CUDA_FOUNDATION_PROGRESS.md`.

The next dependency-ready work is Authoritative Task 1, tracked by GitHub issue #6.

## 6. Current blockers to production readiness

1. Rust project/revision state is not yet authoritative through every UI operation and export.
2. Candidate review, accept/reject, exact source selections, and operation publication are incomplete.
3. The accepted transactional SQLite metadata architecture has not been implemented.
4. The product CUDA runtime, broker, supervised worker, kernels, and telemetry are not implemented.
5. Mistral.rs is not yet supervised as a product process.
6. Perception lacks packaged production model assets and complete view-to-source integration.
7. The frontend remains monolithic and retains browser-owned production compute/export paths.
8. Offline release packaging, security hardening, upgrade/rollback, fault matrix, soak tests, and clean-machine validation are incomplete.

## 7. Completion rule

3DMk may be reported as `PROJECT_COMPLETE` only when every authoritative task and CUDA-foundation task is independently reviewed, verified, recorded, synchronized, and supported by the full release evidence package. Documentation, routes, prototypes, scaffolding, executor checks, or passing static checks alone do not satisfy that state.
