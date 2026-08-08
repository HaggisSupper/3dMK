# 3DMk World-Class Acceptance Matrix

| Domain | Test | Passing evidence | Failure disposition |
|---|---|---|---|
| Source integrity | Import and hash source bytes | Reopen/export/import hashes identical | Release blocker |
| Transactional metadata | Crash during each database transition | Integrity check passes; no partial cross-record state | Release blocker |
| Immutable asset publication | Interrupt write, flush, durable rename, and metadata commit | No authoritative reference to an absent or corrupt payload | Release blocker |
| Atomic operation publication | Kill at every operation phase | No partial revision, operation, analysis, or dangling asset | Release blocker |
| Concurrency | Stale generation and parallel jobs | HTTP 409 or deterministic queue; no overwrite | Release blocker |
| CUDA compliance | Inventory plus known-answer kernel | Device UUID, versions, kernel hash, and correct output | Operational blocker |
| Mistral.rs CUDA | Exact PID in NVIDIA compute-process list | Persisted runtime evidence tied to executable hash | Operational blocker |
| Product CUDA worker | Start, heartbeat, execute, cancel, kill, restart | Supervised recovery; no project mutation by worker | Release blocker |
| VRAM broker | Randomized lease sequences and process reconciliation | Grants plus reserve never exceed observed budget | Release blocker |
| OOM recovery | Force allocation failure | No publish; resources return; exact diagnosis | Release blocker |
| Cancellation | Cancel every transfer, kernel, validation, and publication phase | Immediately non-publishable; bounded cleanup | Release blocker |
| Device/worker loss | Kill worker or invalidate CUDA context | Staged result discarded; controlled restart/circuit breaker | Release blocker |
| Determinism | Repeated seeded runs | Stable order, IDs, hashes within declared tolerance | Capability remains experimental |
| Geometry parity | CPU/CUDA differential corpus | Operation-specific error limits | Capability remains experimental |
| Performance | Representative end-to-end fixtures | Meets complete-workflow gates, not kernel-only gates | Capability remains experimental |
| Reopen | Close/reopen accepted project | Same active revision, review records, measurements, and provenance | Release blocker |
| Package round-trip | Save/re-import complete project | Revision, operation, analysis, and asset identity preserved | Release blocker |
| Export truth | Explicit revision/object exports | No viewport helpers; correct attributes and mappings | Release blocker |
| Fault recovery | Process/GPU/power/disk interruption corpus | Recovery report and valid authoritative state | Release blocker |
| Security | Archive, loopback, origin, session, model, and supply-chain tests | No traversal/link/bomb/origin/session/hash bypass | Release blocker |
| Offline | Clean-machine network-denied launch | Complete primary workflow available | Release blocker |
| Upgrade | Upgrade and forced rollback | Projects remain readable; prior release restored | Release blocker |
| Soak | Twelve-hour mixed workload | Bounded RAM, VRAM, handles, queues, and temporary files | Release blocker |
| Supportability | Generate support bundle after injected fault | Complete, redacted, correlated evidence | Release blocker |
| Accessibility | Keyboard, focus, scaling, contrast, and assistive review | Primary workflows fully operable | Major release blocker |
| Capability truth | Compare API/UI claims with runtime evidence | Every status and fallback is real and reasoned | Release blocker |

## Completion rule

A passing subset does not establish product completion. `PROJECT_COMPLETE` requires every applicable release blocker to pass with fresh evidence on the supported Windows/NVIDIA configuration, plus the full authoritative and CUDA-foundation ledgers.