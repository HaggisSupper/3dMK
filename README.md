# 3DMk

3DMk is a Windows-first, local-first 3D engineering workstation for importing, inspecting, processing, reviewing, preserving, and exporting meshes, point clouds, scan packages, images, measurements, and derived engineering evidence.

The repository is under active implementation. It contains substantial working functionality, but it is **not yet deployment-ready** and has not reached the repository-defined `PROJECT_COMPLETE` state.

## Current architecture

```text
Tauri 2 desktop shell
        ↓
Axum loopback service
        ├── project / asset / revision / job / package APIs
        ├── VWM geometry, perception, and implicit engines
        └── legacy processing routes during migration
        ↓
Three.js workstation UI
```

The Tauri application and browser-development mode use the same Rust/Axum backend. Rust is the authoritative production language. JavaScript owns rendering, input, and review presentation; it is being removed as a second geometry and export authority.

## Current implementation

Implemented today:

- Tauri 2 desktop startup with an embedded Axum service on an ephemeral `127.0.0.1` port;
- Rust CLI commands for floorplan-to-STEP, point-cloud reconstruction, and the development server;
- model and bounded ZIP-package import;
- SHA-256 content-addressed assets;
- projects, root revisions, child-revision foundations, operations, analyses, jobs, and active-revision state;
- package inspection, digest verification, committed package import, and Rust-owned package export;
- a capable mesh/point-cloud viewer with scene tree, cameras, render modes, clipping, measurements, textures, reference images, overlays, cleanup controls, and multiple export surfaces;
- reusable VWM crates for canonical I/O, geometry analysis, perception contracts, and implicit reconstruction;
- a CUDA-only Mistral.rs local development-agent harness.

Important current limitations:

- browser geometry state and Rust project state can still diverge after processing;
- many legacy processing routes are not revision-backed;
- candidate compare/accept/reject and exact persisted source selections are incomplete;
- authoritative project metadata still uses recoverable JSON rather than the accepted SQLite transaction design;
- the product CUDA runtime, GPU broker, supervised compute worker, and CUDA kernels are planned but not implemented;
- Mistral.rs is not yet supervised as part of the product runtime;
- release assets, security hardening, upgrade/rollback, and clean-machine offline acceptance remain incomplete.

The definitive implementation truth is in [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md).

## CUDA requirement

The accepted target system requires:

- Windows 11 x64;
- a supported NVIDIA CUDA-capable GPU;
- at least 6 GiB usable VRAM;
- at least 32 GiB system RAM, with 48 GiB as the reference configuration;
- a CUDA-enabled Mistral.rs runtime with no CPU LLM fallback.

CUDA is the primary backend for eligible heavy compute after each capability passes differential correctness, fault, memory, and end-to-end performance gates. Project I/O, transactions, routing, serialization, and audit remain CPU-owned control-plane work.

The CUDA requirement is an accepted architecture contract; the complete product CUDA plane is not yet implemented.

## Repository structure

```text
src/                         Rust backend, API, project, package, and processing code
src-tauri/                   Tauri 2 desktop shell
public/                      Current single-page workstation UI
VWM-Repo-Implicit/           Canonical reusable VWM Rust workspace
docs/                        Authoritative architecture, plans, ledgers, and operations docs
scripts/                     Windows-first setup, validation, and local-agent scripts
tests/                       Root integration and contract tests
```

## Run the browser-development server

```powershell
cargo run -- serve --port 8181
```

Then open:

```text
http://127.0.0.1:8181
```

## Run the desktop application

With the Tauri CLI and native Windows prerequisites installed:

```powershell
cargo tauri dev
```

This command is not a substitute for the clean-machine release gate; current release packaging remains work in progress.

## Run verification

Root crate:

```powershell
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Canonical VWM workspace:

```powershell
Push-Location .\VWM-Repo-Implicit
.\scripts\check.ps1
Pop-Location
```

CUDA and complete-system acceptance require the reference NVIDIA Windows machine and are tracked separately from static CI.

## Documentation

Start with:

1. [`docs/README.md`](docs/README.md) — authoritative document index.
2. [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md) — implementation truth.
3. [`AGENTS.md`](AGENTS.md) — agent execution and completion contract.
4. [`docs/superpowers/specs/2026-08-07-3dmk-world-class-quality-standard.md`](docs/superpowers/specs/2026-08-07-3dmk-world-class-quality-standard.md) — quality standard.
5. [`docs/superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md`](docs/superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md) — CUDA-first design.
6. [`docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`](docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md) — current product plan.

## Non-negotiable boundaries

- No Docker, Podman, WSL, Electron, or Python production backend.
- No work directly on `main`.
- No capability claim based only on a route, button, crate, feature flag, or document.
- No failed, cancelled, timed-out, stale-generation, out-of-memory, or invalid operation may publish a partial revision.
- No AI or VLM output may directly mutate accepted engineering geometry.
- No project may be reported complete without fresh unit, integration, regression, fault, performance, offline, and end-to-end evidence.