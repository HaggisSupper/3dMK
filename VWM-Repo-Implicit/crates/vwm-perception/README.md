# vwm-perception

Self-contained Rust perception crate for 3DMk/VWM.

It provides:

- RGBA image validation and model preprocessing;
- deterministic connected-region fallback segmentation;
- ONNX instance segmentation through `tract-onnx`;
- direct-mask and YOLO prototype output decoders;
- mask-based extraction of independent mesh faces or point-cloud points;
- transparent object crops;
- image and 3D shape features;
- deterministic shape classification;
- ONNX image classification;
- optional OpenAI-compatible VLM adjudication;
- immutable source geometry and explicit derived-object provenance.

See `../../docs/perception-integration.md` and `../../models/examples/`.

## Deterministic smoke example

```powershell
cargo run -p vwm-perception --example deterministic_pipeline --no-default-features
```

This exercises image validation, connected-region segmentation, transparent cropping,
feature extraction, and deterministic classification without external model weights.

## Model assets

ONNX weights are deliberately external because their licenses, label sets, and output tensors
are model-specific. `models/examples/` contains explicit supported ABI manifests; see
`models/README.md` before attaching a production model.
