# vwm-perception

`vwm-perception` is the reusable Rust perception and source-evidence crate for 3DMk/VWM.

## Implemented now

- RGBA image validation and model preprocessing;
- deterministic connected-region fallback segmentation;
- current tract-based ONNX instance-segmentation and image-classification paths;
- direct-mask and YOLO-prototype output decoders;
- mask-based extraction of source mesh faces or point-cloud points;
- transparent object crops;
- image and 3D shape features;
- deterministic shape classification;
- optional OpenAI-compatible VLM adjudication hook;
- immutable source geometry and explicit derived-object provenance contracts.

A deterministic smoke example is available:

```powershell
cargo run -p vwm-perception --example deterministic_pipeline --no-default-features
```

## Product integration status

The root application currently exposes an experimental VWM perception route and deterministic fallback behavior. Production object recognition is not complete because the repository does not yet include all of the following:

- an approved, licensed, hash-pinned production model package;
- complete renderer RGBA/depth/source-ID evidence capture tied to exact project revisions and cameras;
- deterministic multiview fusion;
- persisted object records, exact source selections, review decisions, and candidate extraction revisions;
- CUDA-backed production inference and measured memory/performance evidence.

Therefore, source presence and route presence do not mean the object-recognition capability is production available.

## Target production backend

The accepted target uses a CUDA-backed production inference backend behind the same explicit model ABI and provenance contracts. The current tract implementation remains useful for deterministic fixtures, CPU reference testing, and migration until the CUDA backend passes parity, fault, memory, and performance gates.

CPU inference is not the final production fallback for the CUDA-compliant system.

## Model assets

ONNX weights remain external until their licenses, labels, preprocessing, tensor names/shapes, output decoders, thresholds, and hashes are explicitly approved. `models/examples/` contains ABI examples; it is not a bundled production model set.

## Authority boundary

The crate may propose masks, classifications, exact source slices, alternatives, confidence, and rationale. It never edits measured geometry or accepts a product decision. The root 3DMk application persists evidence and exposes operator compare/accept/reject workflows.

See:

- `../../docs/perception-integration.md`
- `../../models/README.md`
- root `../../../docs/CURRENT_STATE.md`
