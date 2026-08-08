# vwm-implicit

`vwm-implicit` is the consumer-facing facade for the current Rust implicit-field and generated-geometry crates.

## Implemented facade

1. Build an `OrientedPointSet` from measured points and normals.
2. Reconstruct a `PoissonField` through `PoissonReconstructor`.
3. Evaluate the field, find ray/curve zero crossings, or sample it to a regular grid.
4. Extract a mesh directly from Poisson or through `SurfaceNetsExtractor`.
5. Convert `FieldMesh` or `FieldPointCloud` into a `vwm_core::CanonicalScene` marked as generated geometry.

The measured source scene is never modified.

## Current status

The facade currently exposes the Rust/CPU implicit implementation. It is useful for real reconstruction work, deterministic fixtures, and CPU-reference validation, but it is not by itself the complete production 3DMk reconstruction workflow.

Production integration still requires:

- an accepted input project revision;
- stable source IDs and prepared-point provenance;
- brokered supervised CUDA-primary execution;
- source mapping, attribute transfer, quality report, and accelerator evidence;
- candidate revision publication;
- compare/accept/reject;
- failure, cancellation, OOM, device-loss, deterministic, performance, reopen, and export gates.

See `../../docs/implicit-field-integration.md` and the root authoritative plans. A successful facade call or generated mesh must not be represented as an accepted or quality-approved product revision.