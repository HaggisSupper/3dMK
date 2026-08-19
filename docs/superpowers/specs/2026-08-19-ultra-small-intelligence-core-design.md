# Ultra-Small Local Intelligence Core Design

## Goal

Fold the Ultra Small App LLM work into 3DMK as a bounded, deterministic-first local intelligence core. It must decide whether a request is handled deterministically, requires a local Mistral.rs capability, or is blocked. It must never start a model, silently fall back, or publish authoritative geometry.

## Scope

This implementation adds a typed Rust routing core and a CLI inspection surface. It does not add a second scheduler, download a model, start Mistral.rs, start llama.cpp, add cloud inference, or mutate projects/revisions.

## Why this is the correct first slice

The existing product has local VLM calls but no product GPU broker, supervised model lifecycle, or complete authoritative revision transaction. A pure routing core gives the app one auditable decision point now without competing for GPU memory or creating a new publication path.

## Immutable contract

`contracts/local-intelligence-routing.v1.json` is the machine-readable authority. It defines:

- contract version `1`;
- deterministic, Mistral.rs, llama.cpp, and blocked routes;
- the allowed operation vocabulary;
- input limits;
- no-silent-fallback and advisory-only invariants;
- explicit fallback evidence requirements.

Rust owns the corresponding concrete enums. Dynamic request text is bounded at ingress and cannot select a runtime, endpoint, model, or publication state.

## Routing rules

| Operation | Decision |
|---|---|
| `metadata_inspection`, `geometry_validation`, `evidence_summarization` | deterministic |
| `vision_adjudication`, `floorplan_interpretation` | local Mistral.rs |
| `general_assistance` | blocked until a product-approved capability is defined |

All local-model decisions are advisory candidates. A local Mistral.rs decision is not execution permission. llama.cpp is represented as an explicit future capability only: it requires a separately recorded fallback reason, compatible CUDA evidence, and the same contract version. It is never selected automatically in this slice.

## Safety and ownership

- The core accepts only typed operations, byte counts, and an explicit authoritative-state flag.
- Inputs over the contract limit block before any model path.
- Any request capable of affecting authoritative state is marked `advisory_only`.
- Unknown operations are impossible after typed CLI/API ingress; malformed serialized input must be rejected at deserialization.
- No route exposes secrets, model endpoints, or process controls.
- The core is pure and deterministic: no filesystem, network, process, GPU, clock, or mutable global state.

## Integration

`src/local_intelligence.rs` is exported by `src/lib.rs`. The root CLI gains `intelligence-route`, producing a versioned JSON decision. Existing VLM functions remain unchanged; a later supervised runtime consumes these decisions only after the GPU broker and authoritative transaction gates exist.

## Verification

- Unit tests prove each declared route, oversized-input blocking, advisory-only marking, unknown operation rejection, and no automatic llama.cpp fallback.
- A contract validator rejects version drift, undeclared operations, missing no-silent-fallback invariant, and source/contract divergence.
- The CLI result is serializable JSON with the contract version.

## Non-goals

- General chat.
- A cloud endpoint.
- CPU inference.
- Runtime process management.
- Automatic fallback.
- Direct geometry, project, asset, revision, job, export, or UI mutation.
