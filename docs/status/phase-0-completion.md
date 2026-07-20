# Phase 0 completion — baseline truth and canonical source

**Completed:** 2026-07-18 19:48 America/Cancun  
**Stories:** M0-01 through M0-07  
**Exit gate:** passed

## Delivered

- Created and verified the pre-merge source archive and SHA-256 restore manifest.
- Captured the toolchain, route, dependency, offline, test, and capability baseline.
- Added a versioned eight-fixture catalog and native PowerShell digest verifier.
- Repointed root `vwm-core`, `vwm-io`, and `vwm-geometry` dependencies to `VWM-Repo-Implicit`.
- Restored `PartialEq` on the canonical structural-analysis value types, preserving deterministic 3DMk equality tests.
- Added `GET /api/v1/capabilities` with maturity, reason, engine, dependency, scene-kind, and prerequisite fields.
- Corrected misleading legacy health flags for image refinement, model room recognition, and non-destructive cleanup.
- Added legacy API deprecation headers and an in-process request counter.
- Added a Cargo-metadata integration test that rejects runtime VWM crates outside the canonical tree.
- Cleared five pre-existing Rust 1.94 clippy findings required by the warnings-denied gate.

## Evidence

| Gate | Result |
|---|---|
| Archive test | 929 files, 204 folders, 1,241,342,576 source bytes; 7-Zip `Everything is Ok` |
| Archive SHA-256 | `1089BB8B7D366939AB59A48B905D25728D4BBAF4FE65B64DA9D3A15BF131631B` |
| Excluded cache-path scan | 0 matches |
| Fixture verification | 8 identities verified |
| Canonical VWM format | Pass |
| Canonical VWM clippy, warnings denied | Pass |
| Canonical VWM tests | 40 passed, 0 failed |
| Root format | Pass |
| Root clippy, warnings denied | Pass |
| Root tests | 37 passed, 0 failed |
| Tauri `cargo check` | Pass |
| Live v1 capability count | 17 |
| Live PDAL maturity | `unavailable` |
| Live room-solver maturity | `unavailable` |
| Live object-recognition maturity | `unavailable` |
| Legacy response headers | `Deprecation: true` and successor link present |

## Exit-gate conclusion

One canonical VWM tree resolves; all current source and test gates pass; capability output no longer claims the rejected/unimplemented features work; fixture identities are stable; old source copies remain untouched as planned. Phase 1 may begin.
