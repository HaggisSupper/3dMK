# 3DMk World-Class Risk Register

| Risk | Consequence | Required control | Required evidence |
|---|---|---|---|
| RTX 4050 VRAM contention between Mistral.rs, rendering, and product compute | nondeterministic OOM and unusable workflows | measured broker, safety reserve, explicit model profile, deterministic admission, queueing | coexistence, OOM, and soak tests |
| CUDA driver/toolkit/native-library drift | startup, inference, or kernel failure | version inventory, pinned compatibility matrix, executable/kernel hashes, known-answer kernel | clean-machine and upgrade validation |
| Browser and Rust state divergence | incorrect save, reopen, or export | Rust revision authority; browser is render cache; no viewport export authority | frontend authority and round-trip tests |
| Multi-file JSON metadata during migration | cross-record crash inconsistency | accepted SQLite transaction architecture plus immutable CAS assets | migration, integrity, backup, and crash tests |
| Async pinned-host buffer reuse | corrupted GPU input or nondeterministic results | owned transfer guards, bounded pools, CUDA events, lifetime tests | race and fault-injection tests |
| Device buffer freed while work remains in flight | native fault or data corruption | stream/event ownership, supervised worker, allocator invariants | worker-kill and concurrent-stream tests |
| Cancellation after kernel launch | late partial publication | irrevocable non-publishable state and control-plane transaction gate | phase-by-phase cancellation matrix |
| Stale UI or delayed job publication | newer project state overwritten | expected project generation checked again immediately before publish | concurrency tests returning HTTP 409 |
| Nondeterministic compaction or ordering | unstable hashes, revisions, and comparisons | canonical ordering, persisted seeds, tolerance contracts | repeated-run tests |
| Large scans exceed VRAM | operation failure or destructive downsampling | chunked canonical data, NVMe/RAM/pinned/VRAM tiers, out-of-core scheduling | maximum-fixture and pressure tests |
| Overambitious simultaneous kernel port | regressions and unreviewable change | one kernel family at a time, CPU oracle, independent review, end-to-end performance gate | per-family PR evidence |
| Mistral.rs output changes authoritative geometry | loss of engineering trust | advisory-only contract, typed tools, deterministic validation, user review | mutation-negative tests |
| Perception model ABI or license mismatch | invalid classifications or undistributable release | explicit manifests, pinned hashes/licenses, packaged approved models, strict tensor validation | model compatibility and license evidence |
| Reconstruction creates triangles without quality | false success and unusable geometry | residual, coverage, component, normal, attribute, and visual quality gates | reconstruction quality report |
| Recovery code deletes valuable data | source or project loss | immutable source assets, quarantine before deletion, grace-period reachability GC | recovery corpus and destructive-action tests |
| Update breaks project compatibility | loss of operability | versioned migrations, backups, staged signed update, rollback | upgrade and forced-rollback matrix |
| Dependency or model compromise | supply-chain exposure | pinning, SHA-256, SBOM, license inventory, signed release/update | release provenance evidence |
| Loopback service exposed or unauthenticated | local privilege or cross-origin abuse | `127.0.0.1`, per-launch secret, origin/session validation, restrictive Tauri capabilities/CSP | security tests |
| Capability marketing exceeds implementation | operator distrust and unsafe decisions | typed status with runtime evidence, dependencies, reasons, and verified fallbacks | capability audit |
| Documentation diverges from source again | incorrect implementation and operator action | `docs/CURRENT_STATE.md`, authority order, stale-doc deletion policy, documentation contract tests | documentation audit in every release |

## Review cadence

Review this register after every completed task, architecture decision, serious defect, dependency change, model change, CUDA/toolchain change, and release candidate. New risks are added with an owner, control, and evidence requirement before the associated capability can be marked available.