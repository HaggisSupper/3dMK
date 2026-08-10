# Advanced Spatial and VWM Feature Integration

**State:** `TASK_CANDIDATE`  
**Integration branch:** `agent/spatial-vwm-feature-integration`  
**Historical source:** `agent/spatial-engineering-platform` at `9544fa1c18f5dfdcd168137e9ad97e18c5a6d912`

## Integrated production surfaces

- detached-mesh review, exact staged deletion, hide/show state, explicit save, undo/redo, and package persistence;
- calibrated photo projection with bounded photo counts, camera pairing, occlusion handling, observation thresholds, original-color blending, topology preservation, and coverage reporting;
- VWM perception controls and provenance-aware capability status;
- image refinement, scene, package, point-cloud, reconstruction, API, and viewport/workstation extensions;
- root frontend and engine regression contracts;
- `docs/VWM_ADVANCED_WORKFLOWS.md` operator guidance.

## Deliberately excluded historical material

The integration does not import:

- the duplicate `spatial-engineering-platform/apps/desktop` application shell;
- Python reference or verification code;
- archived bootstrap, extraction, publication, PR, and status markers;
- user model payloads;
- superseded dated design and implementation-plan documents;
- independent contract crates that duplicate the canonical `VWM-Repo-Implicit` workspace without an approved dependency boundary.

The historical branch is retained as the second parent of the curated merge commit so provenance is explicit without making rejected artifacts authoritative.

## Required evidence before merge

- root formatting, compile, unit, contract, integration, regression, and strict lint gates;
- canonical VWM workspace compile, tests, and strict lint;
- Tauri desktop compilation;
- current documentation and Mistral.rs harness validation;
- adversarial review of exact-selection behavior, package/export authority, calibrated-camera pairing, resource bounds, capability truth, and cancellation/error paths;
- correction and rerun of every failing or disputed gate.

This record is not a product-completion claim.
