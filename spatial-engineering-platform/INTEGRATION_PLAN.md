# 3dMK Spatial Engineering Platform Integration Plan

## Objective

Integrate the Spatial Engineering Platform as a bounded `spatial-engineering-platform/` subsystem while preserving the existing root `agentic-cad-backend` commands and the existing `VWM-Repo-Implicit` implementation.

## Phase 0 — Materialize and verify

1. Run `bootstrap/install-spatial-platform.ps1`.
2. Verify repository structure and Python reference tests.
3. Run Cargo workspace tests on a Windows MSVC Rust environment.
4. Commit the extracted source files.
5. Remove the encoded transport archive.

## Phase 1 — Root workspace integration

1. Convert the root manifest into a Cargo workspace without changing the root package name.
2. Add platform crates and applications as members.
3. Keep root CLI behavior intact.
4. Establish workspace dependency, lint, formatting, and security policy.
5. Add Windows-native CI with no containers.

## Phase 2 — Existing-system adapters

1. Implement `vwm-adapter` against the existing VWM crates.
2. Implement `cad-truck-adapter` against the existing Truck dependencies.
3. Preserve current PDAL/implicit reconstruction fallback behind a processing-backend contract.
4. Add compatibility forwarding from the root CLI only after adapter tests pass.

## Phase 3 — Tauri workstation

1. Replace the placeholder shell with Tauri 2, React, TypeScript, and Vite.
2. Implement a fixed, non-dockable primary viewport.
3. Implement constrained Golden Layout docking through the canonical adapter.
4. Restore validated layouts before the first interactive frame.
5. Implement synchronized project tree, viewport selection, properties, jobs, output, and status surfaces.
6. Enforce compact density and explanation-suppression contracts.

## Phase 4 — Spatial processing

1. Add out-of-core point-cloud storage and level-of-detail rendering.
2. Complete native file codecs and golden datasets.
3. Implement cleanup, decimation, normals, picking, primitive fitting, and residual reporting.
4. Implement registration, pose graph, segmentation, and persistent spatial entities.
5. Retain complete provenance and uncertainty through every derived operation.

## Phase 5 — Engineering models

1. Add OpenCascade behind `cad-occ-adapter`.
2. Implement reverse-engineering feature graphs and STEP export.
3. Implement internal BIM graph and IFC4.3 mapping.
4. Add CAD-to-scan and scan-to-scan inspection.
5. Add calibrated metrology and indeterminate outcomes where uncertainty overlaps tolerance.

## Phase 6 — Photogrammetry and semantics

1. Implement camera calibration, SfM, MVS, and LiDAR-image fusion.
2. Supervise Mistral.rs headlessly for local semantic and VLM inference.
3. Use Llama.cpp only as a model-compatibility fallback.
4. Keep AI output advisory until deterministic validation and user acceptance.

## Definition of done

- Root behavior remains stable.
- Workspace tests and strict Clippy pass.
- Platform source is visible as normal repository files.
- Fixed viewport and docking persistence are machine-tested.
- UI contracts and evaluations run in CI.
- Geometry, semantic, BIM, and metrology outputs preserve source evidence, frames, units, residuals, confidence, uncertainty, and provenance.
- CUDA is preferred, Vulkan/WebGPU is the fallback, and multicore CPU remains available.
- Docker, Podman, Electron, and a Python application backend are absent.
