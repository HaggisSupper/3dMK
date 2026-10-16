# 3DMK Verification Matrix

Use only rows relevant to the bounded task. A row is evidence, not a promise.

| Surface | Minimum verification |
|---|---|
| Rust library or CLI | `cargo fmt --all -- --check`; focused unit tests; affected integration tests; `cargo clippy --workspace --all-targets --all-features -- -D warnings` where supported |
| Axum/Tauri route | request-contract tests; invalid-input and size-limit tests; cancellation/timeout tests; publication and rollback assertions |
| Geometry or point-cloud processing | deterministic fixture; CPU reference parity; point/face limits; memory/peak working-set telemetry; OOM and cancellation behavior |
| CUDA path | capability detection; known-answer comparison; deterministic ordering; device-loss and worker-failure handling; CPU reference comparison |
| Mistral.rs integration | Windows CUDA startup evidence; process ownership; model/profile and VRAM budget evidence; failure and restart behavior |
| Browser/UI seam | browser tests for presentation and bounded evidence only; verify no authoritative mutation bypasses Rust |
| Persistence/package/export | round-trip identity; digest/provenance checks; stale-generation rejection; partial-output cleanup; explicit export truth |
| Release/deployment | Windows clean-machine install/launch; offline dependency proof; upgrade/rollback; support bundle; uninstall |
| Documentation/governance | link/reference integrity; task sequence consistency; no unsupported completion claims |

## Gate interpretation

- `PASS`: the command ran against the relevant current commit and its decisive output is recorded.
- `FAIL`: the command or required assertion failed.
- `NOT_RUN`: not applicable or unavailable; explain why.
- `BLOCKED`: an external requirement prevents the gate; name the exact operator action.

Never upgrade `NOT_RUN` or `BLOCKED` to `PASS`.
