# Perception model assets

The Rust crate is complete without model files: its deterministic segmentation, slicing,
feature extraction, shape classification, and VLM-interface tests do not require external
weights.

ONNX weights are not redistributed in this repository because model licenses and output ABIs
vary. Place an approved model beside its JSON manifest and set `model_file` to that exact
filename. The manifests under `examples/` define the supported tensor contracts; they are not
claims that a model file is bundled.

For production deployment, record the model's:

- source and license;
- SHA-256 digest;
- input normalization;
- output tensor indexes and shapes;
- label order;
- export tool and version.

The runtime rejects configuration or tensor-shape mismatches rather than guessing the ABI.
