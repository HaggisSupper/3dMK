# 3DMK Authority Map

This is a navigation aid, not a replacement for the authoritative documents.

| Concern | Authoritative source | Required interpretation |
|---|---|---|
| Current implementation truth | `docs/CURRENT_STATE.md` | Do not infer production readiness from feature presence. |
| Agent/repository rules | `AGENTS.md` | Read before editing; conflicts block work. |
| Immutable contracts | `constitution/immutable-contract.md` or the governing contract named by `AGENTS.md` | Cross-component handshakes must be explicit, versioned, typed, and testable. |
| Progress/dependency sequence | `agent-execution/VWM_PROGRESS.md`, `agent-execution/CUDA_FOUNDATION_PROGRESS.md` | Do not bypass predecessor tasks or mark them complete without evidence. |
| Durable state | Rust project/revision/job/package stores and their contracts | Browser caches and viewport state are not documents of record. |
| Heavy processing | Rust processing routes, workers, admission, cancellation, and publication boundaries | Every operation must follow admit → stage → execute → validate → publish → finalize. |
| GPU/inference truth | CUDA runtime contracts, Mistral.rs startup/telemetry, and known-answer evidence | CUDA-primary is a requirement, not evidence that CUDA is already implemented. |
| Release truth | packaging, installer, offline, rollback, clean-machine, fault, and soak evidence | A passing unit test does not prove deployability. |

## Forbidden authority substitutions

Do not substitute:

- browser state for Rust state;
- a rendered scene for an accepted revision;
- a successful request for a published transaction;
- a CPU fallback for CUDA compliance;
- a model response for deterministic validation;
- a draft PR for verified integration;
- a static document for runtime evidence;
- a local filesystem timestamp for a committed task change.

## Required task record

Every implementation or repair record must identify:

1. base commit and active branch;
2. governing documents read;
3. exact acceptance conditions;
4. failure-reproducing or contract test;
5. focused and regression commands;
6. independent review and verification evidence;
7. rollback and residual-risk notes.
