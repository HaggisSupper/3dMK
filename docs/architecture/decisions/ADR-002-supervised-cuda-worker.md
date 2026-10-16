# ADR-002: Supervised Crash-Only CUDA Compute Worker

**Status:** Accepted

## Decision

Heavy production CUDA work will run in a long-lived supervised process:

```text
3dmk.exe / Axum control plane
        |
        | authenticated named-pipe control messages
        | asset IDs, paths, parameters, leases
        v
3dmk-compute-worker.exe
        |
        +-- one product CUDA primary context
        +-- streams, events, memory pools, and kernels
        +-- staged outputs only
```

Mistral.rs remains a separate supervised CUDA process.

## Invariants

- The worker never opens or mutates the authoritative project database.
- Bulk geometry is exchanged through immutable asset paths, memory-mapped chunks, or staging files, not JSON or Tauri IPC payloads.
- The worker writes only inside the assigned staging directory.
- Every command is idempotently identified by `operation_id`.
- Heartbeats include process ID, device UUID, lease ID, phase, and memory high-water mark.
- The control plane validates and publishes results.
- Worker death invalidates the current staged result and releases the broker lease.
- Restart uses bounded exponential backoff and a circuit breaker.
- CUDA context corruption is contained to the worker process.
- Cancellation immediately makes the operation non-publishable; subsequent worker output is discarded.

## IPC envelope

```json
{
  "schema_version": 1,
  "message_id": "uuid",
  "operation_id": "uuid",
  "kind": "execute|cancel|heartbeat|result|fault",
  "deadline_utc": "RFC3339",
  "payload": {}
}
```

Messages are length-prefixed and authenticated with a per-launch random secret inherited through a restricted process handle or protected environment block.

## Why not in-process only

An in-process CUDA fatal error, allocator corruption, or native-library crash can terminate the control plane and obscure whether durable publication occurred. Process isolation allows project authority, recovery supervision, diagnostics, and operator state to remain alive.

## Current implementation boundary

The worker does not exist yet. Current heavy processing executes in the root process or through external tools. This ADR governs the CUDA-foundation implementation and must not be represented as an implemented capability until its acceptance tests pass.
