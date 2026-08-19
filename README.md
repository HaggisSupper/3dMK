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

The approved target architecture additionally makes 3DMk permanently a **child capability domain of Veritas**. Standalone deployment remains mandatory: the Tauri product will package the required pinned Veritas core components with the 3DMk geometry domain rather than requiring a separately installed Veritas application. This target architecture is approved but not yet implemented.

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
- Veritas child-domain/provider/escalation contracts required by the approved target architecture are not yet integrated;
- no product-runtime cloud intelligence provider exists today;
- release assets, security hardening, upgrade/rollback, and clean-machine offline acceptance remain incomplete.

The definitive implementation truth is in [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md).

## CUDA requirement

The accepted target system requires:

- Windows 11 x64;
- a supported NVIDIA CUDA-capable GPU;
- at least 6 GiB usable VRAM;
- at least 32 GiB system RAM, with 48 GiB as the reference configuration;
- a CUDA-enabled Mistral.rs runtime with no CPU LLM fallback for the active local development executor and CUDA-backed local inference paths that declare Mistral.rs as their provider.

CUDA is the primary backend for eligible heavy compute after each capability passes differential correctness, fault, memory, and end-to-end performance gates. Project I/O, transactions, routing, serialization, and audit remain CPU-owned control-plane work.

The CUDA requirement is an accepted architecture contract; the complete product CUDA plane is not yet implemented. ADR-004 separately permits an optional future product-runtime escalation to an approved OpenAI-compatible cloud model after deterministic and local tiers abstain, subject to Veritas policy, privacy, and authority gates. That optional tier does not satisfy or replace CUDA system compliance, and deterministic core workflows remain offline-capable.

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

## Start the local VLM

The currently implemented VWM object-detection path uses a loopback, OpenAI-compatible Mistral.rs vision server. The helper builds/verifies the CUDA-enabled Mistral.rs binary, sets the runtime endpoint/model variables, and starts a multimodal server:

```powershell
.\scripts\start-mistralrs-vlm.ps1
```

Keep that process running while using the current VWM perception path. The VWM perception panel has a **Check connection** action and will report the configured model, endpoint, and `/v1/models` readiness without exposing credentials. CPU or remote VLM endpoints are rejected by the currently implemented backend. Future ADR-004 cloud escalation is a separate Veritas-governed capability and SHALL NOT bypass this existing local-only endpoint contract.

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

1. [`docs/README.md`](docs/README.md) — authoritative document index and disposition policy.
2. [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md) — implementation truth.
3. [`AGENTS.md`](AGENTS.md) — agent execution, authority, and completion contract.
4. [`docs/architecture/decisions/ADR-004-capability-domain-convergence-architecture.md`](docs/architecture/decisions/ADR-004-capability-domain-convergence-architecture.md) — accepted Veritas-child capability/convergence decision.
5. [`docs/architecture/CAPABILITY_FRAMEWORK_MASTER_SPEC.md`](docs/architecture/CAPABILITY_FRAMEWORK_MASTER_SPEC.md) — approved target architecture.
6. [`docs/architecture/CAPABILITY_FRAMEWORK_AGENT_GOVERNANCE.md`](docs/architecture/CAPABILITY_FRAMEWORK_AGENT_GOVERNANCE.md) — epistemic challenge and normative decision-disposition rules.
7. [`docs/architecture/NON_NEGOTIABLE_ACCEPTANCE_GATES.md`](docs/architecture/NON_NEGOTIABLE_ACCEPTANCE_GATES.md) — mandatory capability-domain gates.
8. [`docs/superpowers/specs/2026-08-07-3dmk-world-class-quality-standard.md`](docs/superpowers/specs/2026-08-07-3dmk-world-class-quality-standard.md) — quality standard.
9. [`docs/superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md`](docs/superpowers/specs/2026-08-07-3dmk-cuda-first-system-design.md) — CUDA-first design.
10. [`docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`](docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md) — current product plan.

## Non-negotiable boundaries

- 3DMk remains a child capability domain of Veritas; missing generic framework contracts are resolved upstream rather than copied locally.
- No Docker, Podman, WSL, Electron, or Python production backend.
- No work directly on `main`.
- No capability claim based only on a route, button, crate, feature flag, document, or conversational approval.
- No failed, cancelled, timed-out, stale-generation, out-of-memory, or invalid operation may publish a partial revision.
- No AI or VLM output may directly mutate accepted engineering geometry.
- No project may be reported complete without fresh unit, integration, regression, fault, performance, offline, and end-to-end evidence.
