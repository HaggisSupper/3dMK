# VWM Perception Integration

## Purpose

`vwm-perception` converts rendered visual evidence into isolated, classified VWM object candidates without modifying measured source geometry.

## Concrete input boundary

A `PerceptionInput` contains:

- one RGBA image frame;
- optional camera intrinsics and camera-to-scene transform;
- optional per-pixel face IDs, point IDs, and depth from the 3D renderer;
- the immutable `CanonicalScene` used to render that frame;
- an optional closed list of domain labels.

The renderer should produce an ID buffer with the same dimensions as the image. Each visible pixel contains either a source face ID, source point ID, or `u32::MAX` for background. This is more reliable than trying to infer 3D membership from color alone.

## Processing flow

```text
RGBA frame
|
instance segmenter
|
2D masks and bounding boxes
|
renderer face/point ID projection
|
independent submesh or point subset
|
image and geometry feature extraction
|
local classifier
|
optional VLM adjudication when confidence is low
|
PerceptionBatch
```

## Concrete output boundary

Each `PerceivedObject` contains:

- the original proposal and binary mask;
- a transparent RGBA crop;
- a derived `CanonicalScene` containing only selected faces or points;
- source face or point IDs for complete traceability;
- image and geometric features;
- classification, confidence, model ID, alternatives, source, and optional rationale.

The object slice has `GeometryOrigin::Derived`. The source scene is not edited.

## ML model ABI

The crate uses pure-Rust `tract-onnx` inference. Models are configured with JSON manifests rather than hard-coded model names.

### Segmentation ABI A: `direct_masks_v1`

- detections output: `[N,6]` or `[1,N,6]`;
- each row: `[x1,y1,x2,y2,confidence,class_id]` in model-input pixels;
- masks output: `[N,H,W]` or `[1,N,H,W]`;
- mask values: probabilities.

### Segmentation ABI B: `yolo_proto_v1`

Supports the common YOLO segmentation structure:

- predictions in channel-first or channel-last layout;
- `cx,cy,w,h`, optional objectness, class scores, and mask coefficients;
- prototype tensor `[M,H,W]` or `[1,M,H,W]`;
- class-aware non-maximum suppression;
- mask reconstruction and letterbox reversal.

The manifest must match the exact export. Model output formats can change between YOLO generations, so the ABI is explicit rather than guessed at runtime.

### Classification ABI

- one NCHW float RGB input;
- output tensor with at least one score per manifest label;
- softmax logits or already-normalized probabilities.

## VLM hook

`OpenAiCompatibleVlm` sends the isolated PNG crop, geometry summary, local classification, and candidate labels to an OpenAI-compatible multimodal chat endpoint. This supports local LM Studio-compatible endpoints and remote compatible services.

The VLM is called only below the configured local-confidence threshold. Its output must be strict JSON and must meet both the VLM minimum confidence and the current local confidence before replacing the classification. It cannot modify geometry.

## Integration into 3DMk

1. Render an RGBA view plus face-ID or point-ID buffer.
2. Construct `ImageFrame` and attach `ProjectionMap`.
3. Pass the original `CanonicalScene` in `PerceptionInput`.
4. Configure a segmenter and classifier.
5. Add a VLM hook only when required.
6. Store accepted object slices as derived scene objects with their source IDs and model provenance.

## Required validation on the development laptop

```powershell
cargo fmt --all
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
