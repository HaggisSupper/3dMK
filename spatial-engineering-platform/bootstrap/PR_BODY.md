# Add Spatial Engineering Platform integration payload

## What changed

- Added the complete Spatial Engineering Platform v0.5 source payload under `spatial-engineering-platform/bootstrap/`.
- Added a deterministic PowerShell extraction script.
- Added the subsystem integration plan and boundary contract.
- Preserved the existing root application without behavior changes.

## Why

The platform is being introduced as an isolated subsystem so its Rust/Tauri, geometry, VWM, CAD, UI, persistence, and processing contracts do not contaminate the existing root backend.

## Validation

The source payload was previously verified with the included repository verifier and Python reference tests. Cargo verification still requires a Windows Rust toolchain after extraction.

## Required follow-up

Run the extraction script, verify the materialized source, commit the normal source tree, and remove the encoded archive transport file.
