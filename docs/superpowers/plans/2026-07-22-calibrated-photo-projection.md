# Calibrated Photo Projection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Project validated package photos into confidence-weighted mesh or point-cloud vertex colors with end-user advanced controls and honest capability reporting.

**Architecture:** Extend `CanonicalCamera` with depth-aware projection, implement image weighting and occlusion in `image_refinement.rs`, expose a bounded multipart API, and wire package-preserved photos/cameras into the Three.js UI. Emit colored PLY without changing topology.

**Tech Stack:** Rust, Axum multipart, VWM canonical scenes, `image`, Three.js, Tauri 2.

## Global Constraints

- Reject uncalibrated or unpaired photos.
- Preserve positions, faces, and normals.
- Bound images to 64 and occlusion grids to 512 pixels on their longest side.
- Report coverage and provenance.
- Keep texture contrast enhancement labeled separately.

---

### Task 1: Define failing engine and frontend contracts

**Files:**
- Modify: `src/image_refinement.rs`
- Modify: `tests/frontend_shell.rs`

- [ ] Add synthetic calibrated-camera tests for visible color projection and occlusion rejection.
- [ ] Require `calibratedPhotoProjectionBtn`, all advanced controls, `projectCalibratedPhotos`, and `/api/calibrated-photo-project` in the frontend contract.
- [ ] Run the focused tests and confirm failures are caused by missing projection code and controls.

### Task 2: Implement calibrated projection

**Files:**
- Modify: `src/packages.rs`
- Modify: `src/image_refinement.rs`

- [ ] Add `CanonicalCamera::project_world_with_depth`.
- [ ] Decode and score camera images.
- [ ] Build a bounded depth grid per camera.
- [ ] Accumulate quality, angle, depth, and visibility-weighted RGB observations.
- [ ] Blend original colors and write colored PLY with preserved faces and normals.
- [ ] Return a serializable coverage report.

### Task 3: Expose the backend API and capability

**Files:**
- Modify: `src/api.rs`

- [ ] Add `POST /api/calibrated-photo-project`.
- [ ] Parse cloud, camera report, photo manifest, options, and at most 64 images.
- [ ] Pair images to valid cameras by preserved package path.
- [ ] Run projection off the async runtime and return the output URL plus report.
- [ ] Mark calibrated image-assisted refinement experimental and usable.

### Task 4: Expose advanced controls

**Files:**
- Modify: `public/index.html`
- Modify: `tests/frontend_shell.rs`

- [ ] Add max-image, occlusion, observation, and blend controls.
- [ ] Preserve source package paths on reference-image blobs.
- [ ] Enable projection only for valid calibrated captures.
- [ ] Submit geometry, camera report, photo mapping, and options.
- [ ] Load returned colored PLY as one undoable edit and display coverage.

### Task 5: Validate and release

**Files:**
- Modify: `docs/3dmk-vwm-feature-merge-plan.md`

- [ ] Run Rust formatting and focused tests.
- [ ] Run the full workspace tests.
- [ ] Run JavaScript syntax and in-app self-test gates.
- [ ] Run bounded Graphify state mapping when available.
- [ ] Build `src-tauri/target/release/three-dmk-desktop.exe`.
