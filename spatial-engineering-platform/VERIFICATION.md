# Verification Record

## Completed in the artifact environment

- Python reference tests.
- Repository structural verifier.
- Immutable UX contract verifier (`scripts/verify_ui_contract.py`).
- Contract corpus includes 11 normative/research documents plus a machine-readable policy baseline.
- Static checks that all native capability declarations have concrete adapter entry points.
- Binary-format implementation tests are included in `format-io`:
  - binary STL write/read round trip;
  - binary PLY mesh write/read round trip;
  - GLB signature and container generation.
- Dependency APIs were checked against current primary Rust crate documentation for `e57` 0.11.13, `las` 0.9.11 and `gltf` 1.4.1.

## Original artifact-environment limitation

The original artifact environment had no Rust toolchain, so it could not execute the workspace. That limitation applied only to the generated archive before materialization.

## Local Windows verification

On 2026-07-23, `cargo test --workspace` completed successfully in the materialized repository on Windows. All 19 unit tests and all doc-test targets passed after correcting generated Rust syntax, type-inference, PLY borrow-checking, and LAS API issues.


## v0.5 dock-layout persistence verification

- UI verifier checks the immutable fixed-viewport invariant.
- UI verifier checks the canonical persistence schema and shipped default layout.
- UI verifier enforces atomic-write, recovery-copy, restore-order, debounce, monitor, and DPI contract concepts.
- Rust contract crate provides typed persisted state, panel policies, and invalid-layout rejection tests.
- Dock-layout contract tests are included in the locally observed passing workspace run above.
