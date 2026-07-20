# Third-party components used by the implicit geometry crates

| Component | Version | Role | License |
|---|---:|---|---|
| `poisson_reconstruction` | 0.4.0 | Pure-Rust Screened Poisson implicit reconstruction and direct mesh extraction | MIT OR Apache-2.0 |
| `fast-surface-nets` | 0.2.1 | Runtime-grid isosurface extraction from sampled signed-distance values | MIT OR Apache-2.0 |
| `ndshape` | transitive through `fast-surface-nets` | Runtime 3D grid linearization | MIT OR Apache-2.0 |
| `nalgebra` | 0.33 | Point/vector conversion and existing VWM numerical types | Apache-2.0 |

The workspace does not vendor these projects. Cargo resolves them from crates.io when the project is built.
