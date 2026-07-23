# Spatial Engineering Platform Design

## Scope

Deliver a testable first vertical slice that accepts point-cloud observations, preserves coordinate/provenance information, performs deterministic reduction and primitive fitting, exposes semantic/BIM/metrology contracts, and provides CLI plus Tauri integration boundaries.

## Architecture

The authoritative implementation is a Rust workspace of small crates. Data crosses crate boundaries only through public concrete types. A standard-library Python implementation mirrors the primary workflows so the repository remains executable where Rust is not installed. The desktop application is a Tauri 2 shell and does not own processing logic.

## Data flow

`file -> canonical point cloud -> summary/decimation -> geometric fit -> semantic hypothesis -> validated engineering entity -> BIM/metrology outputs`

Every derived artifact records method, source identifiers, residuals, confidence, and uncertainty where meaningful.

## Failure behavior

Parsers reject malformed or non-finite coordinates with line context. Algorithms reject underspecified data. Metrology decisions return `Indeterminate` when uncertainty overlaps a tolerance boundary. Semantic hypotheses require explicit acceptance.

## Verification

Rust unit tests are included for public behavior. Python unit tests execute the same primary workflows. `scripts/verify_repo.py` validates required files, workspace membership, forbidden placeholders, and basic source invariants.

## Deferred adapters

Photogrammetry, OpenCascade, CUDA, WebGPU, IFC serialization, E57/LAS binary readers, learned segmentation, OCR, and VLM inference remain versioned adapter boundaries. They are not represented as completed functionality.
