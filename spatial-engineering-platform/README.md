# Spatial Engineering Platform Integration Payload

This branch introduces the Spatial Engineering Platform as a bounded subsystem under `spatial-engineering-platform/`.

## Why the bootstrap payload exists

The connected GitHub publishing environment cannot perform an authenticated native `git push`. The complete verified source tree is therefore committed as a compressed, base64-encoded archive together with a deterministic PowerShell extraction script.

This is an integration transport mechanism, not the intended final repository layout. The archive contains the full v0.5 source tree, including Rust crates, Tauri shell, format adapters, immutable UX contracts, layout-persistence contracts, schemas, tests, samples, and verification scripts.

## Materialize the source tree

From the repository root in PowerShell 7:

```powershell
.\spatial-engineering-platform\bootstrap\install-spatial-platform.ps1
```

The script:

1. Decodes the committed archive to a temporary file.
2. Extracts it into `spatial-engineering-platform/`.
3. Refuses to overwrite an already-materialized source tree.
4. Removes the temporary archive.

Then verify:

```powershell
python .\spatial-engineering-platform\scripts\verify_repo.py
python -m unittest discover -s .\spatial-engineering-platform\reference\tests -v
```

Where Rust is installed:

```powershell
cargo test --manifest-path .\spatial-engineering-platform\Cargo.toml --workspace
cargo clippy --manifest-path .\spatial-engineering-platform\Cargo.toml --workspace --all-targets -- -D warnings
```

## Required follow-up commit

After extraction, commit the materialized source files and remove:

```text
spatial-engineering-platform/bootstrap/spatial-engineering-platform.tar.gz.b64
```

The extraction script may remain as a migration utility or be removed in the same cleanup commit.

## Integration boundary

The subsystem must remain isolated from the current root backend except through explicit adapters:

- `vwm-adapter` for `VWM-Repo-Implicit`
- `cad-truck-adapter` for the existing Truck kernel
- `cad-occ-adapter` for later OpenCascade integration
- typed processing and artifact contracts for existing root CLI forwarding

The fixed primary viewport, dock-layout persistence, compact UI density, explanation suppression, format capability reporting, provenance, uncertainty, and AI validation rules are normative contracts—not optional implementation guidance.
