# VWM Workspace

`VWM-Repo-Implicit` is the canonical reusable Rust engine workspace consumed by 3DMk. It contains geometry, I/O, perception, and implicit-field components; it does not own Tauri, Axum routing, project persistence, revision review, product capability status, or operator decisions.

## Current crates

- `vwm-core` — canonical scenes, units, transforms, datum, geometry origin, structural contracts, and reusable provenance types.
- `vwm-io` — validated model conversion to and from canonical scenes.
- `vwm-geometry` — adjacency, components, patch segmentation, plane fitting, structural relations, and reusable geometric analysis.
- `vwm-perception` — image validation, deterministic region/shape fallback algorithms, current tract-based ONNX paths, projection/source-slicing contracts, and optional advisory VLM hooks.
- `vwm-implicit-core` — implicit-field, grid, intersection, and generated-geometry contracts.
- `vwm-implicit-poisson` — current pure-Rust/CPU Screened Poisson implementation.
- `vwm-implicit-surface-nets` — sampled-field surface extraction.
- `vwm-implicit` — consumer-facing implicit reconstruction facade.

## Product boundary

The workspace returns deterministic geometry, analyses, source selections, mappings, quality evidence, and engine errors. The root 3DMk application owns:

- projects, assets, revisions, operations, analyses, jobs, measurements, and packages;
- expected-generation concurrency;
- candidate compare/accept/reject;
- accelerator admission, supervision, telemetry, and fault handling;
- capability truth and UI presentation;
- explicit export products and release policy.

A VWM result is not an accepted project mutation until the root application validates and publishes it through the authoritative transaction boundary.

## CUDA direction

The current reusable engines are predominantly Rust/CPU implementations and reference oracles. The accepted target adds focused crates for accelerator contracts, the GPU resource broker, CUDA runtime, CUDA geometry, CUDA implicit reconstruction, CUDA vision, verified WebGPU fallbacks, and CPU references.

Until a CUDA backend passes differential correctness, deterministic ordering, cancellation, OOM, device-loss, worker-failure, memory, and end-to-end performance gates:

- it is not a production-available capability;
- the existing CPU implementation remains the reference and migration path;
- a CUDA feature flag or crate alone is not acceptance evidence.

## Source-evidence rule

Perception, cleanup, resampling, reconstruction, and extracted objects must retain stable source point/vertex/face/primitive/instance evidence. Measured source geometry is immutable. Derived geometry is explicitly marked, mapped, quality-gated, and published only as a candidate until accepted by the product workflow.

## Verification

Run from this directory on Windows:

```powershell
.\scripts\check.ps1
```

Equivalent commands:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Live CUDA acceptance requires the supported NVIDIA Windows host and the separate product acceptance workflow. Documentation does not assert a current successful build unless the corresponding progress ledger or CI run contains fresh command evidence.

## Further documentation

- `docs/perception-integration.md`
- `docs/implicit-field-integration.md`
- crate-level READMEs
- root `../docs/CURRENT_STATE.md`
- root CUDA and quality standards under `../docs/superpowers/specs/`
