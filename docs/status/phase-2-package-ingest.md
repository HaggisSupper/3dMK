# Phase 2 package ingest and review status

**Status:** Package inspection, iPhone metadata parsing, atomic commit, canonical export, and UI review are complete; raw-block decoding remains open.

## Delivered

- Safe ZIP inventory in Rust with normalized-path, symlink, encryption, duplicate, size, expansion, and compression-ratio checks.
- Deterministic primary-model ranking with explicit user choice for ties.
- Verified extraction with per-entry SHA-256 and byte-count checks.
- Atomic project publication preserving the source ZIP, selected model, textures, photos, calibration descriptors, dependencies, and derived inventory/camera reports.
- iPhone `Session.json`/`RawImages_blocks.json` parsing with the documented camera convention, valid-pair counts, missing records, and calibration warnings.
- `POST /api/v1/packages/inspect`, `POST /api/v1/projects/import-package`, and `GET /api/v1/projects/:project_id/assets`.
- `GET /api/v1/projects/:project_id/export` creates a canonical Rust-owned ZIP containing the manifest, revisions, and content-addressed assets.
- Browser/Tauri package flow: inspect, choose, commit, reopen model dependencies from project assets, and render preserved texture data.
- Truthful package capability reporting; no subscription or event-stream lifecycle.

## Evidence

| Gate | Result |
|---|---|
| Root tests, all targets | 57 passed, 0 failed |
| Package extraction tests | Digest verification, duplicate grouping, tie-choice, cleanup, and rollback pass |
| API package tests | Inspect and commit/reopen model+texture+source-ZIP pass |
| Real archive inventory | `Untitled_Scan-2026-Jun-23-archive.zip`: 161 entries, 1 GLB, 74 JPEG, 79 JSON, 192,020,309 expanded bytes |
| Tauri release | `src-tauri/target/release/three-dmk-desktop.exe` built and launched with a responsive `3DMk` window |

## Remaining Phase 2 work

- Decode and document raw binary blocks before making raw-point claims.
- Add raw-block decoding and digest-matching re-import coverage for full iPhone capture fidelity.
