# ADR-001: Transactional Project Metadata with Immutable Content-Addressed Assets

**Status:** Accepted  
**Decision:** Replace multi-file JSON as the long-term authoritative metadata mechanism with one SQLite database per project. Retain large payloads in an immutable SHA-256 content-addressed asset store.

## Context

3DMk must atomically coordinate projects, revisions, assets, operations, analyses, measurements, jobs, accelerator evidence, and review decisions. Independent JSON files plus rename-based recovery do not provide one cross-record transaction and make crash recovery, schema migration, queries, and referential-integrity verification unnecessarily complex.

The current JSON store remains the implemented migration source until this ADR is delivered.

## Decision

Each project will use:

```text
<ProjectId>/
├── project.sqlite3
├── assets/
│   └── sha256/<digest>/payload
├── staging/
├── cache/
└── recovery/
```

SQLite owns authoritative metadata and relationships. Asset payloads are immutable files keyed by SHA-256. Device caches, regenerable previews, and temporary worker state are non-authoritative.

Required database configuration:

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;
PRAGMA busy_timeout = 5000;
PRAGMA trusted_schema = OFF;
```

## Atomic publication protocol

1. The compute worker writes only to a job-specific staging directory.
2. The control plane validates bytes, format, attributes, mappings, and quality reports.
3. Each payload is hashed and durably published to its final content-addressed path:
   - flush file contents;
   - use a platform durable rename (`MoveFileExW` with `MOVEFILE_WRITE_THROUGH` on Windows);
   - verify final length and SHA-256.
4. The control plane begins one SQLite transaction.
5. The transaction inserts asset metadata, revision, operation/analysis records, review state, event records, and project-generation update.
6. The transaction commits with `synchronous=FULL`.
7. Staging state is removed after commit.

A crash before the database commit may leave an unreferenced immutable asset. It cannot leave a metadata reference to an unpublished payload. Reachability-based garbage collection removes old unreferenced assets only after a safety grace period.

## Backups and packages

- Use SQLite's online backup API to create a consistent project-database snapshot.
- Stream package payloads from the snapshot's reachable asset list.
- Validate the completed archive before durable publication.
- Never copy a live WAL database with ordinary filesystem copy semantics.

## Migration

The existing JSON store remains readable during migration. A one-way importer creates the SQLite project, verifies every migrated digest, and emits a migration report. The original project is retained unchanged until reopen, round-trip, integrity, and export validation pass.

## Consequences

Positive:

- real cross-record transactions;
- foreign-key enforcement;
- deterministic migrations and integrity checks;
- robust queries and indexes;
- simpler crash recovery;
- consistent online snapshots.

Costs:

- schema and migration discipline;
- SQLite native-library packaging;
- explicit coordination between immutable payload publication and the metadata transaction.
