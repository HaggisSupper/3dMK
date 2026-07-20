# 3DMk Implicit Reconstruction Engine Audit and Port Plan

Date: 2026-07-18  
Status: source audit and correction complete; application port not yet implemented  
Related plan: [Perception engine audit and port plan](../../VWM-Repo-Perception/docs/3dmk-perception-engine-port-plan.md)

## 1. Executive verdict

`VWM-Repo-Implicit` contains the most important reconstruction advance found in the VWM repositories: a functioning, pure-Rust Screened Poisson path from oriented points to a triangle mesh. This can replace 3DMk's unavailable external PDAL executable as the first real native point-cloud-to-mesh engine.

It is not ready to be connected directly to an end-user button.

The engine currently assumes that points are already clean, density-balanced, and supplied with consistently oriented normals. The iPhone LiDAR fixture proves that the solver executes quickly in a release build, but the generated geometry is a set of smooth, closed, disconnected masses rather than a usable room model. Exposing that output as “Reconstruct mesh” would repeat the current product failure: a technically successful process with an unusable visible result.

The correct port is therefore a workflow, not a function call:

1. validate the source package and spatial units;
2. remove isolated scan artifacts and estimate local density;
3. estimate or repair normals and orient them from camera evidence;
4. reconstruct an immutable preview in an isolated Rust worker;
5. compare the preview with the measured source;
6. transfer colors or reproject source photographs;
7. accept the preview as a new revision only after user review.

The Implicit repository should become the canonical VWM source because it is a superset of the Perception repository. The existing `VWM-Repo-Perception` and `vwm-workspace/VWM-Repo` copies should not remain independently editable after compatibility gates pass.

## 2. What the repository actually supplies

| Crate | Useful capability | Product readiness |
|---|---|---|
| `vwm-core` | Canonical geometry, datum, units, origins, source IDs | Reuse after canonicalization |
| `vwm-io` | OBJ, PLY, STL, LAS/LAZ, glTF/GLB ingestion | Reuse with corrected loaders |
| `vwm-geometry` | Adjacency, patches, normals, planes, intersections, relations | Reuse as deterministic geometry layer |
| `vwm-perception` | Local ONNX segmentation/classification and mask-to-source slicing | Reuse behind evidence overlays |
| `vwm-implicit-core` | Oriented point sets, fields, bounds, dense grids, ray/curve zero crossings, field meshes | Reuse as internal contracts |
| `vwm-implicit-poisson` | Pure-Rust Screened Poisson reconstruction | Port behind preprocessing, a worker boundary, and quality gates |
| `vwm-implicit-surface-nets` | Dense sampled-field to mesh extraction | Defer until memory-bounded chunking is implemented |
| `vwm-implicit` | Facade over the implicit crates | Make this the application's reconstruction dependency |

### 2.1 Key advance

The `poisson_reconstruction 0.4.0` dependency produces a real surface from an `OrientedPointSet` without Python, PDAL, a cloud service, or a separate native SDK. It works in a Tauri-compatible Rust toolchain and preserves the project's all-Rust processing direction.

### 2.2 Capabilities that are foundations, not user features

The analytical sphere and plane fields, generic ray intersections, dense scalar grids, and direct Surface Nets extraction are sound lower-level building blocks. They should remain internal. They are not end-user operations and must not be presented as “Voxelize,” “Splat,” “Convex mesh,” or arbitrary reconstruction modes.

## 3. Corrected source defects

The audit first repaired failures that would make any application port unsafe or misleading.

### 3.1 Workspace and loaders

- Imported the required `las::Read` trait so LAS/LAZ ingestion compiles.
- Passed a mutable reader to the PLY parser.
- Rejected PLY vertices with missing X, Y, or Z properties instead of silently inventing zero coordinates.
- Rejected partial normal and color tuples.
- Normalized integer colors by their actual range while preserving already-normalized float colors.
- Rejected non-finite PLY values and empty PLY inputs.
- Supported signed and unsigned PLY face lists, `vertex_index` and `vertex_indices`, and polygon triangulation.
- Rejected negative and out-of-range face indices.
- Traversed glTF scene hierarchies, applied node/world transforms and instances, transformed normals with the inverse transpose, and supported valid non-indexed triangle primitives.
- Rejected invalid glTF indices and ignored non-triangle primitive modes.
- Kept OBJ normal and UV arrays only when they are complete.

### 3.2 Geometry and perception

- Rejected invalid mesh indices instead of panicking.
- Added finite-coordinate and finite-normal validation.
- Corrected plane coplanarity for equivalent planes whose normals have opposite signs.
- Corrected the synthetic room fixture to use the application's Y-up convention.
- Corrected segmentation-mask unletterboxing for low-resolution masks.
- Checked image buffer multiplication and ONNX output/manifest cardinality.
- Rejected non-finite model scores and inconsistent class, label, and mask counts.

### 3.3 Implicit contracts and algorithms

- Rejected non-finite configuration values, overflowed bounds, invalid surface normals, degenerate point sets, and empty reconstructions.
- Normalized extremely large and extremely small finite vectors without overflow/underflow.
- Bounded ray sample counts and checked their arithmetic.
- Rejected non-finite field values during ray and bisection evaluation.
- Suppressed spatially duplicate hits rather than comparing only curve parameters.
- Rejected zero field gradients instead of inventing a Y-axis normal.
- Oriented structured-surface triangles to agree with field normals.
- Bounded Poisson depth to 12 and solver relaxation iterations to 10,000.
- Rejected a degenerate Poisson leaf-cell extent before entering the third-party solver.
- Rejected Surface Nets ISO values that cannot be represented as finite `f32` values.

Runnable regression tests accompany the non-trivial changes.

## 4. Real iPhone LiDAR fixture evidence

Source fixture:

`Model Sample files/Untitled_Scan_2_11_17_57.ply`

Observed source:

- 255,330 points;
- per-point normal, RGB color, curvature, and camera metadata declarations;
- a recognizable room scan;
- detached floating clusters and scanning debris;
- no triangle faces.

| Run | Build | Input sampling | Poisson depth | Load | Reconstruction | Output | Visible result |
|---|---|---:|---:|---:|---:|---:|---|
| A | Debug | stride 256, about 998 points | 5 | completed | exceeded 5 minutes | none | Timed out; unsuitable execution mode |
| B | Release | stride 512, 499 points | 4 | 1.336 s | 1.945 s | 7,492 vertices, 14,980 triangles | One smooth closed occupancy blob; not a usable room |
| C | Release | stride 64, 3,990 points | 5 | 1.019 s | 6.083 s | 3,134 vertices, 6,256 triangles | Multiple disconnected rounded masses; not a usable room |

Generated audit artifacts:

- `output/implicit-audit-stride512-depth4.obj`
- `output/implicit-audit-stride64-depth5.obj`

The correct conclusion is not that Poisson “failed.” It reconstructed the field implied by sparse, noisy, inconsistently sampled oriented points. The failure is in the missing product pipeline around it: artifact removal, density-aware sampling, normal repair, parameter selection, residual analysis, and visible acceptance review.

The stride-based example is intentionally only a diagnostic harness. Production code must never use input order as a spatial sampling strategy.

## 5. Scope boundaries

### 5.1 Port now

- canonical scene and corrected file loaders;
- oriented-point and field contracts;
- Screened Poisson behind presets and hard resource limits;
- deterministic cleanup, normal preparation, and component analysis;
- source-versus-preview quality metrics;
- vertex-color transfer with provenance;
- immutable preview and revision acceptance;
- task-first reconstruction UI;
- job progress, hard cancellation, failure recovery, and local logs.

### 5.2 Port after the base workflow is proven

- camera-pose-guided normal orientation from the scanner ZIP;
- multi-view photograph projection and texture baking;
- perception overlays used to protect or select objects during cleanup;
- ray/field intersections where they solve a specific measurement or sectioning task;
- sparse or chunked field extraction.

### 5.3 Do not expose

- analytical sphere or plane generation;
- raw Poisson depth, screening, or relaxation controls in the normal UI;
- direct dense-grid voxelization;
- raw Surface Nets;
- “Splat,” “Convex mesh,” or “Voxelize point cloud” as alternative user tasks;
- a claim that Poisson is smoothing;
- a claim that reconstruction preserves existing UVs or draped photographs automatically;
- successful completion based only on an HTTP 200, non-empty OBJ, or triangle count.

## 6. Canonical source decision

3DMk currently has at least three live copies of common VWM code:

- `VWM-Repo-Perception`;
- `VWM-Repo-Implicit`;
- `vwm-workspace/VWM-Repo`, which the root `Cargo.toml` currently references.

This guarantees drift. The corrected shared crates in Perception and Implicit have been reconciled. The Implicit workspace is the superset and should become the canonical source.

### Required migration

1. Freeze feature work in the two older copies.
2. Rename `VWM-Repo-Implicit` to a neutral canonical path such as `vwm-workspace/vwm` only after root path dependencies can be changed atomically.
3. Point the app at `vwm-core`, `vwm-io`, `vwm-geometry`, `vwm-perception`, and `vwm-implicit` from that one workspace.
4. Run API compatibility and fixture gates.
5. Archive the old copies outside the build graph.
6. Add a repository check that fails if a second live `vwm-core` package is introduced.

Do not overwrite one directory with another. The migration must be a dependency-path change followed by validation so no app-specific edits are lost.

## 7. Target Rust architecture

~~~text
ZIP / PLY / LAS / GLB
        |
        v
package_ingest.rs
  manifest, images, poses, units, provenance
        |
        v
vwm-io -> CanonicalScene (immutable measured revision)
        |
        v
reconstruction_prepare.rs
  finite checks -> crop -> components -> outliers
  -> density balancing -> normals -> confidence
        |
        v
reconstruction_jobs.rs
        |
        +----> isolated reconstruction worker process
        |       vwm-implicit -> Poisson preview
        |
        v
reconstruction_quality.rs
  residuals, coverage, holes, topology, components
        |
        +----> color_transfer.rs
        +----> projection/bake service (later)
        |
        v
immutable preview revision
        |
        v
user reject / adjust / accept as new derived revision
~~~

### 7.1 Suggested application modules

Keep the first port narrow and reuse current application patterns:

- `src/reconstruction_api.rs` — typed request/response adapters;
- `src/reconstruction_jobs.rs` — lifecycle, status, cancellation, and artifact ownership;
- `src/reconstruction_worker.rs` — child-process protocol and resource limits;
- `src/reconstruction_quality.rs` — comparison metrics and acceptance policy;
- `src/package_ingest.rs` — Rust ZIP parsing and scanner-package validation;
- `src/package_export.rs` — atomic package export with revisions and assets.

Existing `src/api.rs` should route requests but not contain reconstruction logic. Existing `src/point_cloud.rs` should stop shelling out to PDAL once the new path passes quality gates.

## 8. Input and package contract

### 8.1 Accepted inputs

- point cloud with valid positions;
- optional per-point color;
- optional normals;
- optional confidence or curvature;
- declared or user-confirmed units;
- optional camera intrinsics, extrinsics, depth, and source photographs;
- optional crop or selection region.

### 8.2 Scanner ZIP ingestion

Move ZIP inspection out of `public/index.html` and JSZip into Rust. The backend must:

- reject absolute paths, parent traversal, symlinks, duplicate normalized paths, zip bombs, and excessive entry counts;
- stream extraction into a job-owned temporary directory;
- identify geometry, manifest, textures, and reference frames by explicit manifest first and heuristics only as a reviewable fallback;
- preserve every original asset unchanged;
- normalize paths without losing the source name;
- report paired/unpaired RGB/depth/pose records;
- validate image dimensions and camera matrices;
- calculate an immutable package digest;
- never choose a “texture” silently when several candidates tie.

The observed iPhone archive contains 74 paired frames in the ZIP. The expanded sample folder contains a different count, so the UI and logs must report the actual opened source rather than a hard-coded expected value.

### 8.3 Required manifest additions

~~~json
{
  "schema_version": 2,
  "geometry": {
    "file": "geometry/scan.ply",
    "kind": "point_cloud",
    "units": "meters",
    "up_axis": "y"
  },
  "capture": {
    "frames": [
      {
        "id": "frame-0001",
        "rgb": "capture/rgb/0001.jpg",
        "depth": "capture/depth/0001.png",
        "intrinsics": "capture/calibration/0001.json",
        "camera_to_scene": "capture/poses/0001.json"
      }
    ]
  },
  "revisions": [],
  "provenance": {}
}
~~~

The exact iPhone scanner layout must be mapped into this canonical form by a format adapter, not by embedding one scanning application's folder assumptions throughout the engine.

## 9. Preprocessing engine

Poisson should receive a prepared oriented point set, not raw imported points.

### 9.1 Preflight

- validate finite positions, normals, and colors;
- resolve units and Y-up datum;
- calculate bounds, density distribution, point count, and estimated memory;
- flag implausible room dimensions;
- report missing normals and capture poses;
- preserve source IDs through every filter.

### 9.2 Artifact and component cleanup

Implement deterministic, reviewable filters:

1. radius outlier score from neighbor count;
2. statistical outlier score from local distance distribution;
3. voxel-connected or radius-connected components;
4. component size, bounds, distance from the dominant scan, and support evidence;
5. confidence categories: keep, removable, uncertain;
6. visible colored overlay before deletion.

The default action must be non-destructive. “Remove floating pieces” creates a new prepared revision and retains the original measured cloud.

### 9.3 Density-aware sampling

Replace stride sampling with spatial voxel-centroid sampling:

- choose voxel size from source density and target outcome;
- retain representative source IDs;
- average or robustly combine colors;
- select normals only after consistency checks;
- preserve thin regions with low occupancy;
- cap overrepresented near-camera regions;
- provide a preview of retained versus omitted points.

### 9.4 Normal estimation and orientation

For clouds without trustworthy normals:

- use k-nearest-neighbor PCA;
- reject neighborhoods without a stable smallest eigenvector;
- calculate planarity, curvature, and normal confidence;
- orient normals toward or away from the known camera origin for each capture when camera evidence exists;
- otherwise orient a component with a minimum-spanning-tree propagation and a documented seed rule;
- detect normal discontinuities rather than smoothing across wall corners;
- expose a normal-consistency overlay for diagnostics.

For imported normals, run the same consistency checks. “Normals present” is not proof that their sign or magnitude is usable.

## 10. Reconstruction execution

### 10.1 Outcome presets

The normal UI should offer outcome-level presets:

| Preset | Intent | Starting policy |
|---|---|---|
| Fast preview | Validate cleanup and normal orientation | aggressive density balancing, lower depth |
| Room surfaces | Preserve broad walls/floor/ceiling | planar-edge-aware cleanup, moderate depth |
| Object detail | Preserve smaller features in a selected region | tighter crop, higher local density and depth |

Preset values are starting policies, not promises. Calibrate them against fixtures and hardware budgets before finalizing numbers.

An Advanced disclosure may show estimated point count, voxel spacing, and memory. Raw solver fields remain in diagnostic logs unless a developer mode is deliberately added.

### 10.2 Hard cancellation

The third-party Poisson call has no progress or cancellation callback. Dropping a Tokio task does not stop its CPU work. Therefore:

- execute reconstruction in a dedicated child worker process;
- use a versioned JSON-lines or length-prefixed local protocol;
- stream coarse stage events before and after the opaque solve;
- assign a job directory and output paths owned by the parent;
- apply time and memory ceilings;
- terminate the worker for hard cancellation;
- validate and atomically publish its output only after a clean exit;
- delete partial output on failure without touching the source revision.

A thread-pool wrapper alone is insufficient.

### 10.3 Progress semantics

Never invent a fake percentage during the opaque solver call. Report stages:

- Validating;
- Cleaning;
- Balancing density;
- Preparing normals;
- Starting reconstruction;
- Reconstructing surface, elapsed time shown;
- Measuring result;
- Transferring color;
- Ready for review.

## 11. Quality and acceptance engine

A generated mesh is a candidate until measured.

### 11.1 Geometry comparison

Calculate both directions:

- source point to preview mesh distance;
- preview sample to source cloud distance.

Report median, P90, P95, P99, maximum, and histograms in project units. One direction alone can hide unsupported filled surfaces or missing areas.

### 11.2 Coverage and unsupported fill

- percentage of source points within tolerance;
- preview area supported within tolerance;
- unsupported filled area;
- large residual clusters;
- residuals by source component and spatial region.

### 11.3 Topology and integrity

- connected component count and area distribution;
- boundary-loop count and lengths;
- watertightness;
- non-manifold edges and vertices;
- degenerate triangles;
- self-intersection indicator;
- normal consistency;
- bounds and volume change;
- smallest feature size relative to source density.

### 11.4 Acceptance policy

Initial thresholds must be fixture-specific and unit-aware. The app may recommend:

- Ready to review;
- Review warnings;
- Do not accept.

It must not auto-accept merely because the process completed. Acceptance always creates a new `GeometryOrigin::Derived` revision linked to source revision, job configuration, engine version, and quality report.

## 12. Color, photographs, and texture preservation

### 12.1 Point color transfer

For colored point clouds:

- build a spatial index over source points;
- project or nearest-neighbor blend source RGB onto preview vertices;
- weight by distance, normal agreement, and source confidence;
- store transferred vertex colors and source-ID provenance;
- flag preview regions with no reliable color support.

This is suitable for the first reconstruction release and avoids inventing UVs.

### 12.2 Camera-image projection

For ZIP packages with calibrated photographs:

- parse and validate camera intrinsics and camera-to-scene transforms;
- render visibility/depth from the candidate mesh;
- select source frames by view angle, distance, sharpness, and occlusion;
- project images into a texture atlas;
- blend seams with confidence masks;
- keep the original images and projection report in the package.

This depends on the `vwm-capture` and `vwm-projection` work described in the Perception plan. Perception labels can help mask movable objects, but they cannot substitute for camera calibration and visibility.

### 12.3 Existing textured mesh

Remeshing changes topology and invalidates UV correspondence. The app must:

- preserve the original textured revision;
- create a separate remeshed candidate;
- transfer texture only through an explicit bake/projection step;
- show untextured or unsupported regions;
- never claim that retaining a JavaScript texture object preserved the asset.

Export must keep geometry, textures, source photographs, calibration, revisions, and analysis in one atomic ZIP package.

## 13. Application API

Use typed Rust contracts shared by HTTP and Tauri invoke adapters.

### 13.1 Prepare request

~~~json
{
  "scene_revision_id": "measured-001",
  "selection_id": null,
  "cleanup_preset": "room",
  "normal_policy": "prefer_capture_then_mst",
  "target_spacing": null
}
~~~

Response:

~~~json
{
  "job_id": "prepare-...",
  "status": "queued"
}
~~~

### 13.2 Reconstruction request

~~~json
{
  "prepared_revision_id": "prepared-001",
  "preset": "room_surfaces",
  "color_policy": "source_vertex_color",
  "quality_profile": "indoor_lidar"
}
~~~

### 13.3 Job and review resources

- `POST /api/reconstruction/prepare`
- `POST /api/reconstruction/jobs`
- `GET /api/reconstruction/jobs/{id}`
- `DELETE /api/reconstruction/jobs/{id}`
- `GET /api/reconstruction/jobs/{id}/events`
- `GET /api/reconstruction/previews/{id}`
- `GET /api/reconstruction/previews/{id}/quality`
- `POST /api/reconstruction/previews/{id}/accept`
- `DELETE /api/reconstruction/previews/{id}`

The Tauri adapter should call the same Rust service methods directly. Do not reimplement processing in JavaScript.

Every error response includes a stable code, plain-language summary, failing stage, recoverability, and job log reference. Internal paths and stack traces remain out of the UI.

## 14. UI and interaction plan

Open Design project: `3dmk-implicit-reconstruction-workflow`  
Open Design run: `2b61556b-7ba4-4d9d-a9fa-fcd1b2f3457c`  
Artifact: `surface-reconstruction-workflow.html`  
Rendered preview: http://127.0.0.1:63970/api/projects/3dmk-implicit-reconstruction-workflow/raw/surface-reconstruction-workflow.html

The artifact is an interaction/design reference, not production code. Its animated reconstruction bar is illustrative; implementation must use the truthful stage semantics in section 10.3 and show the opaque solver stage as indeterminate with elapsed time rather than a fabricated completion percentage.

### 14.1 Task-first entry

When the loaded active object is a point cloud, show one contextual task:

**Create surface**

Do not show mesh-to-point-cloud, Poisson implementation names, Surface Nets, or unrelated structural tools in the same decision panel.

### 14.2 Four-step flow

1. **Prepare**
   - source summary, units, bounds, normal status;
   - cleanup suggestions with visible artifact overlay;
   - retained and removed point counts;
   - manual crop/selection entry.
2. **Reconstruct preview**
   - Fast preview, Room surfaces, Object detail;
   - estimated runtime/memory;
   - truthful stage progress and Cancel.
3. **Compare**
   - Source, Preview, and Difference view modes;
   - residual heatmap;
   - holes, unsupported fill, detached components, and normal warnings;
   - source and preview metrics beside each other.
4. **Decide**
   - Reject;
   - Adjust settings;
   - Accept as new revision;
   - Accept with warnings only after explicit acknowledgement.

### 14.3 Canvas and side panels

- Keep the 3D canvas visually dominant.
- Use the left workflow panel for the current task, not a catalogue of engines.
- Use one right dock with mutually exclusive View, Selection, and Help tabs.
- Pinning Help may widen the dock but must not place a second panel over the canvas.
- Settings and Help must not overlap or use competing floating buttons.
- Preserve standard 3D controls:
  - left-drag rotates;
  - wheel or two-finger pinch zooms;
  - right-drag or two-finger pan moves the view;
  - touchscreen one-finger rotates and two-finger pinch/pan remains native to OrbitControls.
- Display these controls at the top of Help without intercepting pointer events on the canvas.

### 14.4 Evidence overlays

Every automated result must be visible:

- removal candidates in red;
- uncertain components in amber;
- kept source in neutral point color;
- preview in teal;
- positive/negative residuals in a diverging heatmap;
- boundary loops and holes as line overlays;
- unsupported reconstructed regions in magenta;
- selected source points and corresponding preview faces linked in the inspector.

“Cleanup complete” without an overlay and counts is not sufficient.

### 14.5 State and modularization

`public/index.html` is currently a multi-thousand-line monolith with loading, model state, ZIP parsing, rendering, analysis, and self-tests interleaved. Do not attempt a full visual rewrite before one vertical slice works.

For the first reconstruction slice:

- extract the viewport adapter, job client, reconstruction workflow, revision store, and right dock into modules;
- use a small Zustand vanilla store for shared browser state once the frontend build step is introduced;
- keep high-frequency Three.js frame state outside the reactive store;
- render server-owned job state from typed responses;
- vendor frontend libraries for offline Tauri packaging;
- remove CDN dependencies, including JSZip and remote Draco assets, from the packaged executable.

## 15. Phased implementation backlog

Tasks are intentionally bounded to eight effort points or less. Each phase has an exit gate and should be completed in order.

### Phase 0 — Canonicalize the VWM workspace

| Task | Effort | Deliverable |
|---|---:|---|
| Inventory app-specific changes in all three VWM copies | 3 | Reconciliation report |
| Promote the Implicit superset to the canonical workspace path | 3 | One build-graph source |
| Repoint root path dependencies | 2 | Updated root `Cargo.toml` |
| Add perception and implicit facade dependencies | 2 | App compiles against canonical crates |
| Add duplicate-workspace detection | 2 | Failing check for a second live core |
| Run common fixture and API compatibility gates | 3 | Migration evidence |

Exit gate: the root app and both source-workspace checks pass using one shared source tree.

### Phase 1 — Rust package ingestion and preflight

| Task | Effort | Deliverable |
|---|---:|---|
| Define versioned scanner-package manifest | 3 | Serde contract and examples |
| Implement safe streaming ZIP reader | 5 | Rust package ingest service |
| Add iPhone archive adapter | 5 | Canonical frame records |
| Validate units, axes, bounds, and finite data | 3 | Preflight report |
| Add package digest and immutable source revision | 3 | Provenance record |
| Replace JSZip import path for one vertical slice | 5 | Backend-owned ZIP open |

Exit gate: the sample ZIP opens offline, reports actual frame pairing, and preserves all source assets without browser-side parsing.

### Phase 2 — Point preparation

| Task | Effort | Deliverable |
|---|---:|---|
| Implement radius and statistical outlier scores | 5 | Per-point confidence |
| Implement spatial connected components | 5 | Reviewable component candidates |
| Implement voxel-centroid density balancing | 5 | Prepared points with source IDs |
| Implement kNN PCA normals | 8 | Normals and confidence |
| Implement camera-ray normal orientation | 5 | Capture-guided signs |
| Implement MST fallback orientation | 5 | No-camera fallback |
| Add cleanup and normal overlays | 5 | Visible Prepare review |

Exit gate: the iPhone fixture's floating components are visibly distinguished and the prepared cloud passes normal-consistency thresholds.

### Phase 3 — Safe Poisson worker

| Task | Effort | Deliverable |
|---|---:|---|
| Define versioned worker protocol | 3 | Typed parent/child messages |
| Add standalone reconstruction worker binary | 5 | Isolated solver |
| Add resource estimates and configuration caps | 3 | Refusal before unsafe work |
| Add hard cancellation and partial-output cleanup | 5 | Termination test |
| Map outcome presets to internal configuration | 3 | Calibratable policies |
| Publish immutable preview artifacts atomically | 3 | Revision-safe output |

Exit gate: a cancellation leaves no accepted output or runaway process, and release reconstruction completes from the prepared real fixture.

### Phase 4 — Quality comparison

| Task | Effort | Deliverable |
|---|---:|---|
| Implement source-to-mesh distances | 5 | Coverage metrics |
| Implement mesh-to-source distances | 5 | Unsupported-fill metrics |
| Add components, boundary loops, and manifold checks | 5 | Topology report |
| Add normal and degenerate-triangle checks | 3 | Integrity report |
| Define unit-aware quality profiles | 3 | Review recommendations |
| Add real-fixture baseline artifacts | 3 | Regressions and thresholds |

Exit gate: the two audit meshes are automatically classified as unsuitable without relying on a human noticing the blobs.

### Phase 5 — Color and provenance

| Task | Effort | Deliverable |
|---|---:|---|
| Build source-point spatial index | 3 | Reusable lookup |
| Transfer vertex colors with confidence | 5 | Colored preview |
| Track source IDs for transferred attributes | 3 | Provenance |
| Mark unsupported color regions | 3 | Visible warning overlay |
| Export source, preview, report, and settings atomically | 5 | Revisioned ZIP |

Exit gate: the accepted preview reopens with the same colors and complete source provenance.

### Phase 6 — Reconstruction UI vertical slice

| Task | Effort | Deliverable |
|---|---:|---|
| Extract reconstruction state and job client | 5 | Testable modules |
| Implement Prepare step | 5 | Cleanup review |
| Implement preset and progress step | 5 | Cancellable job UI |
| Implement Source/Preview/Difference modes | 8 | Comparison canvas |
| Implement quality and warning inspector | 5 | Evidence review |
| Implement reject/adjust/accept revision actions | 5 | Non-destructive decision flow |
| Consolidate View/Selection/Help right dock | 5 | No panel collision |

Exit gate: a user can complete the workflow without encountering engine terminology, hidden automation, or destructive replacement.

### Phase 7 — Optional field and Surface Nets work

| Task | Effort | Deliverable |
|---|---:|---|
| Measure dense-grid memory across target sizes | 3 | Capacity model |
| Design overlapped chunk extraction and stitching | 5 | Approved technical design |
| Implement boundary-consistent chunk sampling | 8 | Chunked field |
| Weld and validate chunk seams | 5 | Seam quality report |
| Expose only through a proven product task | 3 | No raw engine button |

Exit gate: memory remains bounded and chunk seams pass geometry checks. If no product task requires this, stop after measurement.

### Phase 8 — Perception and image-assisted refinement

| Task | Effort | Deliverable |
|---|---:|---|
| Add capture and projection crates | 8 | Camera/frame contracts |
| Generate renderer ID/depth buffers | 8 | 2D-to-3D mapping |
| Run local ONNX proposals across selected views | 5 | Candidate overlays |
| Fuse candidates across views | 8 | Stable object groups |
| Protect/remove selected classes during cleanup | 5 | Reviewable policy |
| Implement calibrated texture bake | 8 | Photo-derived atlas |
| Add bake residual and source-view report | 5 | Texture evidence |

Exit gate: identified objects and image-derived texture are visibly traceable to source views and survive package reopen.

### Phase 9 — Release and hardware validation

| Task | Effort | Deliverable |
|---|---:|---|
| Vendor all frontend runtime dependencies | 5 | Offline assets |
| Run browser interaction matrix | 5 | Mouse, trackpad, touch evidence |
| Run Tauri release build from a clean state | 3 | Launchable executable |
| Test spaces/non-ASCII paths and large ZIPs | 5 | Windows path evidence |
| Run memory/runtime soak on target laptop | 5 | Hardware budget |
| Reopen exported packages and compare digests | 3 | Round-trip evidence |

Exit gate: the packaged executable launches without a development server, completes a real reconstruction workflow, and reopens its output with colors and provenance intact.

## 16. Validation matrix

| Layer | Required checks |
|---|---|
| Rust source | format, workspace check, all-feature tests, Clippy warnings denied |
| Loaders | malformed/partial/non-finite fixtures; glTF transform and instance fixtures; real PLY/LAS files |
| Preparation | deterministic component IDs, source-ID preservation, density histograms, normal orientation |
| Worker | crash, timeout, cancel, disk full, partial file, version mismatch |
| Reconstruction | real clouds at several densities; release runtime and peak memory |
| Quality | synthetic known-distance shapes plus visually reviewed real scans |
| Color | color-error metric and unsupported-region overlay |
| ZIP | traversal, duplicate path, bomb limits, corrupt entry, missing manifest, ambiguous images |
| UI | state transitions, keyboard/accessibility, evidence overlays, no canvas obstruction |
| Input devices | mouse wheel/drag, Precision Touchpad pinch/two-finger pan, touchscreen pinch/pan |
| Tauri | offline launch, no CDN requests, package open/reopen, clean exit, second launch |

Mocked endpoint success and synthetic sphere reconstruction are necessary unit checks, but neither is a product-quality gate.

## 17. Risks and mitigations

| Risk | Consequence | Mitigation |
|---|---|---|
| Inconsistent normals | Inside/outside inversion and blobs | Camera-guided orientation, confidence, preview overlay |
| Uneven LiDAR density | Overfit near camera and missing distant surfaces | Spatial density balancing |
| Poisson fills gaps | Invented walls or closed blobs | Bidirectional residuals and unsupported-fill overlay |
| Opaque solver cannot cancel | Runaway CPU in Tauri | Isolated worker process |
| Dense grids exhaust memory | Desktop crash | Defer Surface Nets; capacity model and chunks |
| Remesh breaks UVs | Draped texture disappears or lies | Preserve original revision and explicitly rebake |
| Multiple VWM copies drift | Reintroduced loader/geometry bugs | One canonical workspace and duplicate check |
| Browser ZIP processing | Memory spikes and inconsistent Tauri behavior | Streaming Rust ingest/export |
| CDN frontend assets | Release executable fails offline | Vendor and bundle dependencies |
| UI exposes engine vocabulary | Confusing choices with no clear outcome | Contextual task and outcome presets |
| Quality thresholds overfit one scan | False approval or rejection | Curated multi-fixture corpus and hardware calibration |

## 18. Definition of done

The Implicit port is complete only when all of the following are true:

- one canonical VWM workspace supplies all app dependencies;
- scanner ZIPs are parsed and validated in Rust;
- raw clouds are cleaned and normals prepared with visible evidence;
- reconstruction runs in an isolated cancellable Rust worker;
- the original measured revision is never overwritten;
- every preview has bidirectional residual and topology reports;
- poor audit outputs are rejected by automated quality gates;
- point colors transfer with provenance;
- photograph-based textures, when enabled, are calibrated and explicitly baked;
- the UI supports Prepare, Preview, Compare, and Accept without engine jargon;
- mouse, trackpad, and touchscreen controls work with standard behavior;
- ZIP export/reopen retains geometry, images, calibration, revisions, and reports;
- the Tauri executable launches and completes the real-fixture workflow offline;
- runtime, memory, and visible geometry quality are documented on the target Windows hardware.

Until these gates pass, the Implicit repository should be described as a proven reconstruction foundation, not a completed 3DMk feature.
