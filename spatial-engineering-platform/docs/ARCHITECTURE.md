# Architecture

## Trust boundary

The platform distinguishes four classes of artifact:

1. **Observation** — immutable sensor data.
2. **Derived geometry** — deterministic processing with parameters and residuals.
3. **Hypothesis** — semantic or learned interpretation awaiting validation.
4. **Accepted engineering entity** — explicitly validated model state.

A hypothesis cannot be serialized as an accepted BIM/CAD/metrology result without a validation record.

## Adapter boundaries

| Adapter | Contract responsibility | Intended backend |
|---|---|---|
| `CaptureImporter` | Convert source formats to canonical observations | E57, LAS, PLY, Modelar packages |
| `PhotogrammetryBackend` | Return refined camera poses and dense geometry | COLMAP/OpenMVG/OpenMVS-derived worker |
| `CadKernel` | B-rep, booleans, healing, STEP | OpenCascade through isolated FFI |
| `ComputeBackend` | Point/mesh kernels | CUDA first, WebGPU/Vulkan fallback |
| `SemanticBackend` | Structured hypotheses only | mistral.rs/VLM/classical models |
| `BimSerializer` | Validated IFC/BCF/IDS output | IFC4.3 implementation |

## Coordinate policy

All geometry is stored in an identified coordinate frame. Transform composition is explicit. Export recentering and axis conversion create new frames rather than mutating source coordinates.

## Scaling policy

Point data is designed for chunking and future out-of-core storage. The current slice uses in-memory vectors intentionally; large-scale backends must preserve the same public contracts.


## Advanced 3D continuity

The v0.2 workspace explicitly restores the advanced 3D thread's missing boundaries:

- `mesh-core` owns triangle topology and native OBJ/OFF/ASCII-STL handling.
- `format-contracts` owns the full interchange capability registry.
- `world-model-contracts` owns persistent world entities, relations, object extraction, classifiers, VLM proposals, and compute-backend priority.
- Compute priority is CUDA, then Vulkan, then WebGPU, then CPU.
- Classifiers and VLMs propose hypotheses; deterministic validation remains authoritative.
- Point clouds, meshes, scenes, CAD entities, and semantic world entities remain distinct artifacts connected by provenance.

This is aligned with the advanced 3D architecture, but the binary codecs, GPU kernels, photogrammetry engine, and vector-world-model inference are still adapter implementations rather than completed engines.
