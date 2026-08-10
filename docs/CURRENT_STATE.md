# 3DMk Current Implementation State

**Audit date:** 2026-08-10  
**Audited integration baseline:** advanced spatial/VWM integration candidate on `agent/spatial-vwm-feature-integration`; this document describes the source state carried by that candidate and remains valid when it is merged  
**Branch admission policy:** before model startup, the controller fetches current `origin/main`, fast-forwards the branch when it is strictly behind, blocks on divergence, and admits work only after proving `origin/main` is an ancestor of the active implementation branch  
**Product state:** active implementation; not deployment-ready; `PROJECT_COMPLETE` has not been reached

## 1. System boundary

3DMk is a Windows-first, local-first 3D engineering workstation composed of:

- a Tauri 2 desktop shell;
- one embedded Axum service bound to loopback;
- a Three.js workstation UI;
- a Rust CLI and reusable Rust backend library;
- a filesystem-backed project, asset, revision, job, and package subsystem;
- the canonical `VWM-Repo-Implicit` Rust workspace for I/O, geometry, perception, and implicit reconstruction;
- a mandatory CUDA-first target architecture and a CUDA-only local Mistral.rs development executor.

Cloud accounts and hosted processing are outside the current product. OpenCode is not part of the active workflow.

## 2. Implemented runtime surfaces

### Tauri and Axum

Implemented:

- Tauri 2 starts the same Axum router used by browser development;
- the backend binds to `127.0.0.1` on an available port;
- the workstation HTML is embedded and materialized beneath application data;
- startup failures are recorded to a local log;
- the CLI exposes `pdf-to3d`, `poisson-reconstruct`, and `serve`.

Not implemented:

- release-grade offline vendoring of every frontend dependency;
- per-launch application/session authentication;
- final installer, upgrade, rollback, uninstall, and clean-machine acceptance.

### Projects, assets, revisions, and packages

Implemented:

- project create/list/read;
- direct model import for GLB, GLTF, PLY, OBJ, STL, FBX, DAE, 3DS, 3MF, OFF, U3D, and X3D through the canonical VWM I/O path;
- bounded upload streaming and explicit source-unit handling;
- safe ZIP inspection and package import;
- immutable source assets keyed by SHA-256;
- projects, root and child revisions, operations, analyses, jobs, and active-revision state;
- asset listing and streaming;
- Rust-owned package export;
- package asset classification, digest verification, image/camera inventory, and provenance records.

Current limitations:

- authoritative metadata remains recoverable JSON plus content-addressed assets rather than the accepted SQLite/CAS transaction architecture;
- the complete admit → stage → execute → validate → publish → finalize transaction is not implemented for every operation;
- package rehydration and every browser action are not yet proven to preserve authoritative revision identity end to end.

### Viewer and workstation UI

Implemented:

- mesh and point-cloud loading for browser-supported formats;
- committed Rust package loading and package inspection;
- scene tree, perspective/orthographic cameras, orbit controls, clipping, lighting, render modes, point-size and stride controls;
- textures, reference images, measurements, cropping, room-plan overlays, structure/fragment overlays, contextual help, and explicit export controls;
- transient undo/redo and detached-mesh review with exact staged component selection, hide/show, save, and package persistence;
- calibrated-photo projection controls and reports;
- advanced VWM geometry, perception, reconstruction, floorplan, and rendering controls;
- browser serializers for GLB, OBJ, PLY, and STL.

Current limitations:

- detached-mesh review is exact and reversible in transient/browser state but is not yet published as an immutable reviewed Rust revision;
- `loadedGeometryStore`, transient undo state, overlays, measurements, and browser-built ZIP output can still diverge from the Rust project record;
- post-import geometry operations are not consistently revision-backed;
- legacy export paths can still traverse viewport state;
- the frontend remains monolithic and uses network-hosted development assets that must be vendored before offline release.

## 3. Advanced spatial and VWM integration

The integrated feature set now includes:

- exact detached-fragment identification and review state;
- calibrated photo-to-geometry binding by upload filename, camera ID, and exact package source path;
- bounded calibrated PLY and image ingestion;
- occlusion-aware vertex-color projection with topology preservation and coverage evidence;
- image assessment and refinement contracts;
- VWM perception with deterministic fallback and optional local VLM adjudication;
- expanded point-cloud analysis, leveling, smoothing, reconstruction, scene metadata, and floorplan extraction;
- production-facing capability truth and regression contracts for the advanced UI.

Security and resource controls added during integration:

- VLM connection, model, and credentials are server-owned and cannot be overridden by request payloads;
- Mistral.rs and VLM endpoints are restricted to loopback HTTP origins;
- perception uploads have compressed-byte and decoded-pixel budgets;
- calibrated projection has route, multipart, cloud, report, manifest, provenance, option, image-count, compressed-image, vertex, and face limits;
- unsafe filenames, duplicate photo uploads, ambiguous camera bindings, invalid provenance, and unsupported formats fail closed;
- failed processing removes pending output rather than publishing a partial result.

These features are implemented and tested as an integrated candidate, but they remain `experimental` wherever immutable revision publication, packaged model assets, CUDA parity, or deployment gates are absent.

## 4. Canonical VWM engine state

Implemented reusable crates include:

- `vwm-core` — canonical scene and shared contracts;
- `vwm-io` — validated multi-format conversion to canonical scenes;
- `vwm-geometry` — structural and geometric analysis primitives;
- `vwm-implicit-core`, `vwm-implicit-poisson`, `vwm-implicit-surface-nets`, and `vwm-implicit` — CPU/Rust implicit reconstruction;
- `vwm-perception` — image validation, deterministic regions/shapes, current ONNX paths, projection/source-slicing contracts, and optional VLM adjudication hooks.

Not implemented or not production-integrated:

- persisted immutable source selections for every cleanup and perception result;
- revision-backed structure analysis, cleanup, smoothing, leveling, conversion, reconstruction, projection, and object extraction;
- packaged licensed production ONNX assets and complete multiview evidence;
- CUDA-primary VWM kernels and ONNX Runtime CUDA execution;
- quality-gated reconstruction acceptance through authoritative revision review.

## 5. CUDA and Mistral.rs truth

Accepted system requirement:

- the complete supported system requires a compatible NVIDIA CUDA GPU;
- Mistral.rs must run through CUDA; CPU or cloud inference fallback is prohibited;
- eligible heavy product compute is CUDA-primary after parity, determinism, fault, memory, and performance gates pass.

Implemented now:

- CUDA-only local Mistral.rs development-agent setup and controller;
- source-build provenance and executable-hash checks;
- live Mistral.rs CUDA-process observation through NVIDIA telemetry;
- authoritative Task 1–17 and CUDA Foundation FB1–FB8 selectors;
- fail-closed mainline and predecessor admission;
- separate implementer, reviewer, verifier, and repair sessions;
- static Windows validation of executor and documentation contracts.

Product CUDA runtime not implemented:

- Tauri/Axum ownership of the Mistral.rs lifecycle;
- product CUDA inventory and known-answer kernel;
- shared GPU resource broker;
- supervised product CUDA worker;
- pinned staging and device caches;
- CUDA geometry, reconstruction, image, and perception kernels;
- product-visible accelerator APIs and UI.

Therefore, the repository has a CUDA-first architecture and CUDA-only development executor, but not a complete CUDA-first product runtime.

## 6. Active dependency sequence

```text
Authoritative Tasks 1–5
        ↓
CUDA Foundation FB1–FB8
        ↓
Authoritative Tasks 6–14 with CUDA-primary backends
        ↓
Tasks 15–16 frontend authority, offline packaging, security, and deployment
        ↓
Task 17 end-to-end fault, performance, soak, and clean-machine closure
```

The controller enforces the sequence from:

- `agent-execution/VWM_PROGRESS.md`;
- `agent-execution/CUDA_FOUNDATION_PROGRESS.md`.

The next dependency-ready product work remains Authoritative Task 1, tracked by GitHub issue #6. Integrating advanced feature code does not bypass or mark any authoritative/foundation task complete.

## 7. Current blockers to product completion

1. Rust project/revision state is not authoritative through every UI operation and export.
2. Candidate publication, compare, accept/reject, exact persisted source selections, and recovery are incomplete.
3. The accepted transactional SQLite/CAS architecture is not implemented.
4. Product CUDA runtime, broker, worker, kernels, and telemetry are not implemented.
5. Mistral.rs is not supervised as a product process.
6. Perception lacks packaged production model assets and complete source-linked multiview integration.
7. Browser-owned production compute/export paths and external frontend dependencies remain.
8. Installer, offline packaging, session security, upgrade/rollback, fault matrix, long soak, and clean-machine deployment validation remain incomplete.

## 8. Completion rule

3DMk may be reported as `PROJECT_COMPLETE` only when every authoritative task, CUDA-foundation task, reliability gate, package/deployment test, adversarial review, and release evidence requirement has passed freshly. Documentation, routes, prototypes, scaffolding, integrated features, executor checks, or passing static CI alone do not satisfy that state.
