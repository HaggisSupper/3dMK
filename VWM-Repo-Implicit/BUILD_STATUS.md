# VWM Verification Boundary

This file defines what constitutes current verification for `VWM-Repo-Implicit`. It is not a completion report.

## Required local verification

From the workspace root:

```powershell
.\scripts\check.ps1
```

The script must execute, with zero failures:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Evidence authority

Fresh command output belongs in:

- the relevant GitHub Actions run;
- `../docs/agent-execution/VWM_PROGRESS.md` for authoritative product tasks;
- `../docs/agent-execution/CUDA_FOUNDATION_PROGRESS.md` for accelerator-foundation work;
- the final release evidence package.

This document does not claim that the current commit passes those commands. Historical archive assembly, delimiter scans, manifest generation, or ZIP CRC checks are not substitutes for Rust compilation and tests.

## CUDA verification

Normal CI may validate contracts, fake inventories, CPU references, and dynamic-loading behavior without an NVIDIA device. Production CUDA status additionally requires the supported Windows/NVIDIA host and fresh evidence for:

- driver/device inventory;
- known-answer CUDA kernel;
- product worker lifecycle;
- GPU broker invariants and VRAM reserve;
- Mistral.rs/product-worker coexistence;
- differential correctness, deterministic ordering, cancellation, OOM, device-loss, memory-return, and end-to-end performance gates.

## Status interpretation

- A crate existing in the workspace means source is present.
- A passing workspace check means that commit passes the listed Rust gates in that environment.
- A passing CUDA acceptance suite means that backend passed its declared CUDA gates on the recorded device.
- Product availability requires the root 3DMk workflow, revision, quality, UI, fault, and release gates in addition to crate verification.
