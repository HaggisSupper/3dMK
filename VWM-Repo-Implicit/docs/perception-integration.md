# VWM Perception Integration

## Purpose

`vwm-perception` converts rendered visual evidence into isolated, classified object candidates with exact source-geometry evidence. It never modifies accepted measured geometry.

## Input boundary

A production `PerceptionInput` is tied to one project revision and contains:

- one RGBA frame;
- depth or visibility evidence when required;
- camera intrinsics and camera-to-scene transform;
- a same-size face-ID, point-ID, primitive-ID, or instance-ID buffer;
- the immutable canonical scene/revision used to render the evidence;
- an approved closed label set and model manifest;
- capture/view identity, timestamp, viewport parameters, and evidence hashes.

Background pixels use an explicit sentinel. Source-ID buffers are required for authoritative 3D membership; color-only inference cannot establish exact source geometry.

## Processing flow

```text
revision-bound RGBA/depth/source-ID views
        ↓
approved instance segmenter
        ↓
2D masks, boxes, classes, scores
        ↓
source-ID projection and exact selections
        ↓
multiview association and deterministic fusion
        ↓
image + geometry features and local classification
        ↓
optional advisory VLM adjudication under policy
        ↓
persisted object candidates and evidence
        ↓
operator review
        ↓
optional candidate derived-object revision
```

## Output boundary

Each object candidate records:

- proposal, masks, boxes, transparent crops, and contributing view IDs;
- exact source-selection assets and counts;
- optional derived preview scene;
- image and geometric features;
- classification, alternatives, confidence, model ID/hash, ABI version, thresholds, and backend evidence;
- source revision, cameras, projection evidence, parameters, and warnings;
- deterministic fusion identity;
- operator decision and optional advisory rationale.

The source scene is not edited. An extracted object becomes geometry only through a root-application candidate revision and normal compare/accept/reject workflow.

## Current versus target backend

Implemented today:

- deterministic region/shape fallbacks;
- tract-based ONNX paths with explicit model manifests;
- mask/source-slicing and optional compatible VLM hooks.

Required for production availability:

- packaged approved model assets and licenses;
- complete revision-bound RGBA/depth/source-ID capture;
- multiview fusion and persisted object records;
- CUDA-backed production inference;
- brokered memory, supervised execution, cancellation/fault containment, accelerator provenance, and measured acceptance evidence.

The current tract path remains a CPU reference/migration implementation. It is not the final production fallback for a CUDA-compliant installation.

## Model ABI

Every model manifest defines:

- license and redistribution status;
- file SHA-256;
- input names, dtypes, color space, normalization, resize/letterbox policy, and shape bounds;
- output names, shapes, decoder version, label map, confidence/NMS/mask thresholds;
- approved execution providers and precision;
- model/version identity written into operation and object records.

Output tensor layouts are never guessed silently. A mismatch fails closed with a precise model-ABI error.

## Advisory VLM boundary

The VLM receives only bounded, redacted evidence required by policy. It may rank or explain candidates but cannot:

- change exact source membership;
- mutate geometry;
- accept/reject a revision;
- override deterministic validation;
- publish a product record without the normal Rust transaction path.

## Verification

Required tests include:

- image/source-ID dimension and sentinel validation;
- malformed and incompatible model manifests;
- decoder fixtures for every supported ABI;
- exact source-slice membership;
- multiview ordering and seeded repeatability;
- CPU/CUDA differential tolerances;
- cancellation, OOM, worker death, stale generation, and late-result suppression;
- model/backend provenance and package round-trip;
- negative tests proving VLM output cannot mutate accepted state.

Run the workspace gates through `../scripts/check.ps1`. Production CUDA acceptance requires the supported NVIDIA Windows host and root product workflow evidence.