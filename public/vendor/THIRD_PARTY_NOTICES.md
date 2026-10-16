# 3DMK pinned offline frontend dependencies

The shipped runtime files are unmodified copies of these already-pinned browser distributions:

| Component | Version | Source | Upstream license |
| --- | --- | --- | --- |
| Three.js and required addons | 0.160.0 | Official npm `three@0.160.0` archive | MIT |
| three-mesh-bvh | 0.7.0 | Official npm `three-mesh-bvh@0.7.0` archive | MIT |
| jsPDF UMD browser bundle | 2.5.1 | Official npm `jspdf@2.5.1` archive | MIT, with original bundled notices |
| JSZip browser bundle | 3.10.1 | Official npm `jszip@3.10.1` archive | MIT option selected; the unchanged upstream license file also contains its alternative GPL license |
| Draco browser decoders | 1.5.6 | `three@0.160.0/examples/jsm/libs/draco/` | Apache-2.0; original tagged LICENSE and AUTHORS included |

Draco's JavaScript decoder, WASM wrapper, and WASM binary were each verified byte-for-byte against the original `https://www.gstatic.com/draco/versioned/decoders/1.5.6/` files. The wrapper and WASM binary also match the official `draco3d@1.5.6` package; its wrapper is named `draco_decoder_nodejs.js` there. The separate Three.js glTF-optimized decoder variant is not used.

All original distributed runtime bytes and embedded notices are retained. The `licenses/` directory contains package license files, original jsPDF/JSZip bundled notice text, and supplemental license texts for bundled components identified in upstream release lockfiles (Babel helpers, fflate, immediate, lie, pako, and setimmediate). Those license-only downloads add no runtime dependencies. Exact bundled component builds have not been independently reconstructed; the existing published bundles remain unchanged.

The jsPDF source distribution includes an existing MD5 notice stating that its author does not specify a particular license. That original notice is retained in `licenses/jspdf-bundled-NOTICES.txt`. This bounded repackaging does not newly resolve that upstream licensing ambiguity or constitute a fresh legal or security audit.

The `runtime-assets.json` manifest lists relative deployment paths, SHA-256 hashes, byte sizes, content types, and asset kinds for the runtime and notice files. It excludes itself and `SHA256SUMS` to avoid self-referential hashes. `SHA256SUMS` additionally authenticates `runtime-assets.json` and can be checked from this directory with `sha256sum -c SHA256SUMS`.
