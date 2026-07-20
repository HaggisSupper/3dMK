# VWM Workspace

Self-contained Rust workspace for the 3DMk Vector World Model pipeline.

## Crates

- `vwm-core` — canonical scene, datum, geometry provenance, and shared contracts.
- `vwm-io` — mesh and point-cloud ingestion into `CanonicalScene`.
- `vwm-geometry` — mesh adjacency, patch segmentation, plane fitting, relations, and intersections.
- `vwm-perception` — image preprocessing, ONNX instance segmentation and classification, object mask-to-3D slicing, feature extraction, and optional VLM adjudication.

## Perception boundary

The renderer supplies an RGBA frame and a same-size face-ID or point-ID buffer. The perception crate returns independent derived submeshes or point subsets with source IDs and classification provenance. It never edits measured geometry.

See `docs/perception-integration.md` and `models/examples/`.

## Laptop verification

This package was assembled in an environment without a Rust toolchain, so compilation could not be executed here. Run:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The scripts under `scripts/` run the standard checks and packaging flow. See `BUILD_STATUS.md` for the precise verification boundary of this archive.


## Implicit geometry crates

The repository now includes a concrete implicit reconstruction stack:

- `vwm-implicit-core`: field contracts, analytical fields, sampled grids, ray zero-crossings, canonical outputs.
- `vwm-implicit-poisson`: pure-Rust Screened Poisson reconstruction.
- `vwm-implicit-surface-nets`: field-to-mesh extraction using `fast_surface_nets`.
- `vwm-implicit`: facade crate for application consumers.

See `docs/implicit-field-integration.md` for the data flow and recommended starting parameters.
