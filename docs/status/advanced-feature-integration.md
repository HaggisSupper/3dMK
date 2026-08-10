# Advanced Spatial and VWM Feature Integration

**State:** `MERGE_CANDIDATE`  
**Integration branch:** `agent/spatial-vwm-feature-integration`  
**Historical source:** `agent/spatial-engineering-platform` at `9544fa1c18f5dfdcd168137e9ad97e18c5a6d912`  
**Curated integration lineage:** current mainline as first parent, historical feature head as second parent, explicitly selected tree

## Integrated production surfaces

- detached-mesh review with exact staged selection, hide/show state, explicit save, undo/redo, and package persistence;
- calibrated-photo projection with bounded photo counts, exact camera/source-path pairing, occlusion handling, observation thresholds, original-color blending, topology preservation, and coverage reporting;
- VWM perception controls with deterministic fallback and optional local VLM adjudication;
- expanded image refinement, scene, package, point-cloud, reconstruction, API, and workstation behavior;
- multi-format direct model import through the canonical VWM I/O path;
- root frontend, security, resource-limit, and engine regression contracts;
- `docs/VWM_ADVANCED_WORKFLOWS.md` operator guidance.

## Integration corrections

The historical branch was not merged wholesale. The curated candidate corrects the following before acceptance:

- restores the current direct-import format set rather than regressing to GLB/PLY/OBJ only;
- makes VLM endpoint, model, and credentials server-owned;
- restricts Mistral.rs/VLM connections to loopback HTTP;
- removes credential and endpoint controls from the browser request surface;
- bounds perception request bytes and decoded pixels;
- preserves calibrated-projection request, image, vertex, face, and provenance budgets;
- removes clippy failures and an avoidable linear cluster lookup;
- keeps capability text explicit that transient exact review exists while immutable revision publication does not;
- retains current CUDA-first governance, current-state truth, dependency gates, and active executor.

## Deliberately excluded historical material

The integration does not import:

- the duplicate `spatial-engineering-platform/apps/desktop` shell;
- Python reference or verification code;
- bootstrap, extraction, publication, PR, and status markers;
- user model payloads;
- superseded dated design/implementation documents;
- duplicate contract crates that have no approved dependency boundary with `VWM-Repo-Implicit`.

Git lineage records the source branch without making rejected files authoritative.

## Merge evidence required

The final candidate must pass, on one immutable head:

- root formatting, compile, unit, contract, integration, regression, release-build, and strict lint gates;
- canonical VWM formatting, workspace compile, tests, and strict lint;
- Tauri debug and release compilation;
- current documentation validation;
- CUDA-only Mistral.rs harness validation;
- adversarial review of exact-selection behavior, package/export authority, calibrated-camera binding, resource bounds, local-only AI connections, capability truth, cancellation, and error cleanup;
- zero unresolved review threads;
- removal of temporary repair workflows and scripts.

This record describes the feature-integration merge. It is not a `PROJECT_COMPLETE` claim for the full 3DMk application.
