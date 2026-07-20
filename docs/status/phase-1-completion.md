# Phase 1 completion — durable projects, revisions, and jobs

**Completed:** 2026-07-18 20:22 America/Cancun  
**Stories:** M1-01 through M1-09  
**Exit gate:** passed

## Delivered

- Added versioned Rust records for projects, assets, revisions, operations, analyses, and jobs.
- Added immutable SHA-256 source and derived assets with streamed writes, byte limits, deduplication, and safe display names.
- Added recoverable project metadata writes and atomic project, root-revision, asset, and child-revision publication.
- Added v1 create, import, read, list, asset-streaming, revision-list, and active-revision endpoints with typed safe errors.
- Added bounded persisted jobs with validated transitions, progress, cancellation, terminal records, restart failure recovery, and job-specific temporary-output cleanup.
- Kept the job API deliberately simple: `GET /api/v1/jobs/:id` polls status and `DELETE` requests cancellation. There is no SSE, subscription, reconnect, or listener lifecycle.
- Kept both browser development and Tauri on the same Axum/ProjectStore implementation.

## Evidence

| Gate | Result |
|---|---|
| Root format | Pass |
| Root Clippy, all targets, warnings denied | Pass |
| Root tests, all targets | 50 passed, 0 failed |
| Phase 1 library tests | 45 passed, 0 failed |
| Fixture identities | 8 verified |
| Tauri `cargo check` | Pass |
| Real GLB source | `Model Sample files/Untitled_Scan-2026-Jun-23.glb` |
| Real GLB import | 453,090 vertices; 148,262 faces; mesh root revision |
| Real source/stored asset bytes | 10,398,860 / 10,398,860 |
| Real source/stored SHA-256 | `7DCEA9BAEB84A4424224601BDCBD4A3592C2ECEFDA2BDBB3705DC192DBC59473` / exact match |
| Reopen proof | Project `09421dd1-ac11-4eb9-9bc9-3dc0d85b2fc4` reloaded with root/active revision `3b54801f-cfa5-4a03-9fdb-d2b069fbc661` |
| Subscription/SSE search | No lifecycle references or `/events` route remain; regression test expects 404 |

## Exit-gate conclusion

A real model imports into immutable project storage, survives server restart, and reopens by project ID with identical source bytes. Failed writes cannot publish partial projects or assets. Job records survive UI disconnection and restart without requiring a subscription mechanism. Phase 2 may begin.
