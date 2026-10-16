# Implicit Reconstruction Integration

## Purpose

The implicit workspace provides reusable field and surface-reconstruction contracts. The current implementation is a Rust/CPU reference and migration engine. Production 3DMk reconstruction additionally requires authoritative revisions, source mappings, brokered CUDA execution, quality gates, fault containment, and operator review.

## Current reusable pipeline

```text
OrientedPointSet
  points + normals
        ↓
PoissonReconstructor
        ↓
PoissonField: ImplicitField
        ├── ray/curve zero crossings
        ├── sampled DenseScalarGrid
        ├── direct Poisson mesh
        └── Surface Nets mesh from a sampled grid
        ↓
FieldMesh / FieldPointCloud
        ↓
CanonicalScene marked as generated geometry
```

The source measured scene is not modified.

## Crate responsibilities

| Crate | Current responsibility |
|---|---|
| `vwm-implicit-core` | field contracts, analytical fields, grids, intersections, generated geometry conversion |
| `vwm-implicit-poisson` | current pure-Rust/CPU Screened Poisson reconstruction |
| `vwm-implicit-surface-nets` | sampled-field isosurface extraction |
| `vwm-implicit` | application-facing facade and re-exports |

The contracts permit future CUDA sparse fields, CUDA Poisson solving, chunked extraction, alternate implicit methods, and verified WebGPU fallbacks without exposing backend-specific types to product callers.

## Product input contract

Production reconstruction consumes an accepted project revision and a prepared point set containing:

- positions in canonical units;
- consistently oriented normals;
- stable source point IDs;
- optional colors and confidence;
- declared datum/origin and local precision policy;
- preparation parameters, deterministic seed, and input hashes.

Preparation itself is an authoritative, provenance-recorded operation. Ad-hoc browser PLY uploads are not the target production boundary.

## Product output contract

Reconstruction returns staged generated geometry plus:

- source mapping and coverage;
- attribute-transfer report;
- residual and normal metrics;
- component, degeneracy, boundary/manifold, and triangle statistics;
- backend/device/kernel/precision/memory/transfer/timing evidence;
- warnings and quality disposition.

The root application may publish this as a candidate revision only after deterministic validation. Generating triangles is not success, and the result is not active until operator acceptance.

## CUDA target

The accepted target is chunked, brokered CUDA-primary preparation, field solving, and extraction through the supervised product worker. The current CPU implementation remains:

- the deterministic reference oracle;
- the implementation used for small fixtures and differential testing;
- a migration path while CUDA parity is incomplete.

It is not the normal production fallback after CUDA acceptance.

Datasets larger than a granted lease are chunked or processed out of core through the tiered NVMe/RAM/pinned/VRAM cache. Whole-scene VRAM residency is not required.

## Quality and fault gates

Required before production availability:

- representative real fixtures;
- CPU/CUDA differential tolerances;
- deterministic seeded ordering/hashes;
- source coverage and mapping verification;
- residual, normal, component, degeneracy, and boundary/manifold gates;
- attribute-transfer validation;
- cancellation, OOM, device loss, worker death, stale generation, and publication-failure tests;
- no partial revision on any failure;
- end-to-end performance improvement after transfers and validation;
- compare/accept/reject UI evidence;
- package/reopen/export round-trip.

## Current operational guidance

For CPU/reference fixtures:

- start at `65^3` or `129^3` sampled grids;
- begin with Poisson `max_depth` 7 or 8;
- validate normal orientation before reconstruction;
- preserve measured geometry beside the generated field/mesh for residual checks;
- record parameters and seed.

These values are starting points, not production quality guarantees.