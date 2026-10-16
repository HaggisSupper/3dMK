# Perception Model Assets

No production perception weights are bundled in this repository.

The `vwm-perception` source can compile and exercise deterministic segmentation, source slicing, feature extraction, shape classification, decoder fixtures, and VLM-interface tests without external weights. That does **not** make production object recognition complete.

## Model-package requirements

An approved model package contains:

- the ONNX or accepted engine file;
- source, license, redistribution terms, and attribution;
- SHA-256 digest;
- model/version identity;
- input tensor names, dtypes, shape bounds, color space, normalization, resize/letterbox policy;
- output names, dtypes, shapes, decoder version, label order, confidence/NMS/mask thresholds;
- supported execution providers and precision modes;
- export tool and version;
- representative decoder and compatibility fixtures.

The manifests under `examples/` demonstrate ABI shapes only. They are not proof that a corresponding model is bundled, licensed, compatible, accurate, or approved for production.

## Runtime rules

- Tensor layouts and label maps are never guessed silently.
- Missing weights, license metadata, hash mismatch, unsupported execution provider, or tensor-shape mismatch fails closed with a typed model-ABI error.
- Production inference is CUDA-backed under the accepted system architecture; the current tract path remains a reference/migration implementation until the production CUDA backend passes its gates.
- Model output is advisory evidence. It cannot directly mutate accepted project geometry.
- Product availability also requires revision-bound RGBA/depth/source-ID capture, multiview fusion, exact source selections, persisted object records, operator review, fault tests, performance evidence, and package round-trip.

See `../docs/perception-integration.md` and the root `docs/CURRENT_STATE.md`.