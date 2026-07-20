# vwm-implicit

Facade crate for VWM implicit geometry.

## Operational pipeline

1. Build an `OrientedPointSet` from measured points and normals.
2. Reconstruct a `PoissonField` with `PoissonReconstructor`.
3. Evaluate the field directly, find ray/curve zero crossings, or sample it with `sample_field_to_grid`.
4. Extract a mesh directly from Poisson or use `SurfaceNetsExtractor` on the sampled grid.
5. Convert `FieldMesh` or `FieldPointCloud` into `vwm_core::CanonicalScene`.

The original measured scene is not modified. All extracted outputs are marked as generated geometry.
