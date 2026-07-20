# VWM Implicit Fields Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add concrete crates that reconstruct implicit fields from oriented points, evaluate those fields along rays, sample them into dense grids, and extract point clouds or meshes.

**Architecture:** `vwm-implicit-core` owns stable contracts and deterministic analytical/grid/ray logic. `vwm-implicit-poisson` wraps the pure-Rust Screened Poisson implementation. `vwm-implicit-surface-nets` wraps chunk-friendly Surface Nets extraction. `vwm-implicit` is the consumer-facing facade.

**Tech Stack:** Rust 2021, nalgebra 0.33, poisson_reconstruction 0.4, fast-surface-nets 0.2.1, serde, thiserror.

## Global Constraints

- Measured source geometry remains immutable.
- Generated meshes and point clouds are marked `GeometryOrigin::Generated`.
- Every backend consumes and produces concrete serializable contracts.
- No Python runtime, Docker, Electron, or Apple-specific dependencies.
- The CPU implementation remains the correctness reference.

---

### Task 1: Core contracts and validation

**Files:**
- Create: `crates/vwm-implicit-core/src/contracts.rs`
- Create: `crates/vwm-implicit-core/src/error.rs`
- Test: `crates/vwm-implicit-core/tests/contracts.rs`

**Interfaces:**
- Consumes: `vwm_core::CanonicalScene`, `vwm_core::GeometryOrigin`
- Produces: `Aabb3`, `OrientedPointSet`, `FieldMesh`, `FieldPointCloud`, `ImplicitField`

- [ ] Write tests for valid and invalid bounds and oriented point sets.
- [ ] Implement the minimum concrete contracts needed by all backends.
- [ ] Validate finite values, matching point/normal counts, and nonzero normals.

### Task 2: Dense sampled field

**Files:**
- Create: `crates/vwm-implicit-core/src/grid.rs`
- Test: `crates/vwm-implicit-core/tests/grid.rs`

**Interfaces:**
- Consumes: `ImplicitField`, `GridSamplingConfig`
- Produces: `DenseScalarGrid`

- [ ] Test indexing, trilinear interpolation, and field sampling.
- [ ] Implement overflow-safe grid construction and interpolation.
- [ ] Implement conversion of arbitrary fields to sampled grids.

### Task 3: Analytical fields and ray intersections

**Files:**
- Create: `crates/vwm-implicit-core/src/analytical.rs`
- Create: `crates/vwm-implicit-core/src/ray.rs`
- Test: `crates/vwm-implicit-core/tests/ray.rs`

**Interfaces:**
- Consumes: `ImplicitField`, `Ray3`, `RayIntersectionConfig`
- Produces: ordered `RayHit` values with positions and normals

- [ ] Test two sphere intersections and no-hit rays.
- [ ] Implement normalized rays, bracketing, bisection, duplicate suppression, and gradient normals.

### Task 4: Surface Nets extraction

**Files:**
- Create: `crates/vwm-implicit-surface-nets/src/lib.rs`
- Test: `crates/vwm-implicit-surface-nets/tests/sphere.rs`

**Interfaces:**
- Consumes: `DenseScalarGrid`
- Produces: `FieldMesh`

- [ ] Test mesh extraction from a sampled sphere.
- [ ] Wrap `fast_surface_nets` with runtime grid dimensions.
- [ ] Transform lattice positions and normals into world coordinates.

### Task 5: Screened Poisson reconstruction

**Files:**
- Create: `crates/vwm-implicit-poisson/src/lib.rs`
- Test: `crates/vwm-implicit-poisson/tests/reconstruction.rs`

**Interfaces:**
- Consumes: `OrientedPointSet`, `PoissonConfig`
- Produces: `PoissonField` and `FieldMesh`

- [ ] Test config validation and a low-depth sphere reconstruction.
- [ ] Wrap `PoissonReconstruction::from_points_and_normals`.
- [ ] Expose field evaluation, gradients, bounds, and marching-cubes mesh extraction.

### Task 6: Facade and integration documentation

**Files:**
- Create: `crates/vwm-implicit/src/lib.rs`
- Create: `crates/vwm-implicit/README.md`
- Create: `docs/implicit-field-integration.md`
- Modify: `Cargo.toml`
- Modify: `README.md`

**Interfaces:**
- Produces: one import surface for the complete implicit reconstruction stack.

- [ ] Re-export the stable contracts and operational backends.
- [ ] Document point-cloud-to-field, field-to-grid, ray-intersection, and field-to-mesh workflows.
- [ ] Update workspace membership and validation scripts.
