# Implicit Field Integration

## Concrete boundaries

### Point cloud to continuous field

```text
OrientedPointSet
    points: Vec<[f64; 3]>
    normals: Vec<[f64; 3]>
        |
        v
PoissonReconstructor
        |
        v
PoissonField: ImplicitField
```

### Field to ray/curve intersections

`find_ray_intersections` evaluates `F(r(t)) - iso_value`, brackets sign changes, and applies bisection. Each hit includes position, field value, parameter `t`, and a normal from the field gradient.

### Field to sampled grid

`sample_field_to_grid` evaluates any `ImplicitField` at a regular 3D lattice and returns `DenseScalarGrid` with world origin, nonuniform axis spacing, dimensions, values, and iso-value.

### Grid to mesh

`SurfaceNetsExtractor` passes the sampled signed-distance values to `fast_surface_nets`, then transforms lattice vertices and normals into VWM world coordinates.

### Poisson direct mesh

`PoissonField::reconstruct_mesh` uses the mesh buffers exposed by `poisson_reconstruction` and returns the same `FieldMesh` contract as Surface Nets.

## Why there are four crates

| Crate | Responsibility |
|---|---|
| `vwm-implicit-core` | Stable contracts, analytical fields, grids, ray intersections, mesh conversion |
| `vwm-implicit-poisson` | Pure-Rust Screened Poisson reconstruction from oriented points |
| `vwm-implicit-surface-nets` | Chunk-compatible isosurface extraction from sampled fields |
| `vwm-implicit` | Consumer-facing facade and re-exports |

The split permits later backends such as MLS, RBF, OpenVDB, CUDA sparse grids, Dual Contouring, or a learned residual field without changing the consumer contracts.

## Example

```rust
use vwm_implicit::*;

let input = OrientedPointSet::new(points, normals)?;
let field = PoissonReconstructor.reconstruct(&input, PoissonConfig::default())?;

let hits = find_ray_intersections(
    &field,
    Ray3::new(camera_origin, camera_direction)?,
    RayIntersectionConfig::default(),
)?;

let grid = sample_field_to_grid(
    &field,
    GridSamplingConfig {
        bounds: field.bounds(),
        dimensions: [129, 129, 129],
    },
)?;

let mesh = SurfaceNetsExtractor.extract(&grid, SurfaceNetsConfig::default())?;
let scene = mesh.into_canonical_scene()?;
```

## Runtime guidance

- Start at `65^3` or `129^3` for interactive tests.
- Use Poisson `max_depth` 7 or 8 for ordinary scans before increasing it.
- Preserve consistently oriented normals; inverted normals corrupt inside/outside sign.
- Use the ray-intersection route when structured rays or curves already exist.
- Use Surface Nets when a sampled field is required for reuse, chunking, or later GPU storage.
- Keep measured geometry beside the implicit field for provenance and residual validation.
