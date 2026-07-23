# Spatial Engineering Platform

A Rust-first, Tauri-oriented foundation for turning mobile LiDAR/photogrammetry captures into traceable engineering geometry.

This repository is the first production slice of a larger reverse-engineering, semantic-understanding, BIM, and metrology platform. It deliberately separates **implemented deterministic functionality** from **extension contracts** that require mature external kernels, trained models, calibrated sensors, or GPU backends.

## What works now

- Canonical coordinate frames, transforms, provenance, confidence, and uncertainty types.
- Native point-cloud ingestion for XYZ, XYZRGB, CSV, TXT, ASC, PTS, PTX, ASCII PLY, OBJ vertices, ASCII STL vertices, and OFF vertices.
- Native mesh parsing for OBJ, OFF, and ASCII STL, with OBJ export.
- Deterministic voxel-grid decimation and statistical summaries.
- Plane and cylinder fitting primitives with residual reporting.
- Semantic hypothesis contracts that cannot silently mutate engineering state.
- BIM element and metrology result schemas.
- Rust CLI commands for ingest, decimate, fit-plane, and measure.
- Runnable Python reference CLI and tests requiring only Python 3.11+.
- Tauri 2 desktop-shell source wired around a command boundary.

## Explicitly not claimed complete

- Full SfM/MVS photogrammetry.
- Globally optimized multi-session SLAM.
- OpenCascade B-rep reconstruction and STEP authoring.
- Neural point-cloud segmentation or VLM inference.
- Complete IFC4.3 serialization.
- Survey/metrology certification.
- CUDA/WebGPU kernels.

Those are represented as stable interfaces and roadmap modules rather than fabricated implementations.

## Run immediately

```powershell
py -3 -m unittest discover -s reference_py/tests -v
py -3 reference_py/spatial_engineering/cli.py summarize samples/plane.xyz
py -3 reference_py/spatial_engineering/cli.py fit-plane samples/plane.xyz
```

## Rust workflow

Install Rust 1.78+ and run:

```powershell
cargo test --workspace
cargo run -p spatial-cli -- summarize samples/plane.xyz
cargo run -p spatial-cli -- fit-plane samples/plane.xyz
```

## Repository map

- `crates/spatial-types`: canonical data contracts.
- `crates/format-contracts`: authoritative native/adapter format capability registry.
- `crates/pointcloud-core`: multi-format import, summaries, decimation.
- `crates/mesh-core`: triangle meshes and native OBJ/OFF/STL parsing.
- `crates/world-model-contracts`: object extraction, classifier, VLM, relationship, and compute-backend contracts.
- `crates/geometry-fit`: primitive fitting and residuals.
- `crates/semantic-contracts`: proposal/validation boundary for ML/VLM.
- `crates/bim-contracts`: typed BIM entities and validation states.
- `crates/metrology`: traceable measurements and uncertainty-aware decisions.
- `crates/spatial-cli`: deterministic command-line workflows.
- `reference_py`: executable reference implementation and tests.
- `apps/desktop`: Tauri 2 shell source.
- `docs`: architecture, format matrix, design, plan, and capability roadmap.

## Design rules

1. Raw observations are immutable.
2. Every transformation is explicit and provenance-bearing.
3. AI produces hypotheses, never authoritative geometry mutations.
4. Quality and uncertainty travel with derived artifacts.
5. Sensor tier limits are never hidden by polished visualization.
6. Modules communicate through concrete versioned contracts.
7. Rust is authoritative; Python is a testable reference and interoperability layer.
8. No Docker.


## Advanced 3D conversation alignment

Version 0.2 corrects the original repository's partial alignment. It now includes explicit mesh, interchange, and world-model modules; object-extractor, classifier, and VLM interfaces; and CUDA → Vulkan → WebGPU → CPU compute priority. See `docs/ARCHITECTURE.md` and `docs/FORMAT_SUPPORT.md`.

## v0.3 binary and structured format adapters

The `format-io` crate adds executable adapters for E57 read, LAS/LAZ read-write, ASCII/binary PLY read-write, ASCII/binary STL read with binary write, and glTF/GLB mesh read-write. Use `spatial-cli convert-cloud` and `spatial-cli convert-mesh` for conversion. See `docs/FORMAT_SUPPORT.md` for retained and deliberately unsupported semantics.

## v0.4 immutable UX and workflow specification

The interface is now governed by normative contracts under `docs/ui/`. These specify the dense engineering workstation frame, persistent project tree, synchronized selection, seven operational workspaces, viewport interactions, compact design tokens, explanation suppression, keyboard/command behavior, accessibility, errors/jobs, and a release-blocking UI evaluation matrix. `docs/ui/ui-contract.json` is the machine-readable policy baseline and `scripts/verify_ui_contract.py` enforces structural invariants.

The existing desktop shell remains a placeholder and is explicitly non-conforming; implementation must be replaced against these contracts rather than incrementally styled.


## v0.5 dock-layout persistence guardrails

The primary viewport is now contractually fixed as the non-dockable spatial authority. Complete dock state must persist across sessions and restore before the first interactive frame. The canonical schema, default engineering layout, recovery/migration rules, atomic-save protocol, monitor/DPI reconciliation, and Rust validation contracts are defined under `docs/ui/` and `crates/ui-layout-contracts`. Raw docking-library state may not be persisted directly.
