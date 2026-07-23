# Format Support Matrix

Support is reported per operation. **Native** means executable code is present in the repository. **Partial** means the format is usable but some format semantics are intentionally not retained. **Adapter contract** means the boundary is defined but no codec is claimed.

| Format | Read | Write | Implemented behavior |
|---|---:|---:|---|
| XYZ / XYZRGB / CSV / TXT / ASC | Native | Native | XYZ and optional 8-bit RGB |
| PTS | Native | Native | Optional point-count header |
| PTX | Native | Partial | Structured header import; flat export only |
| PLY | Native | Native | ASCII plus binary little/big-endian import; binary little-endian export; vertices, RGB and polygon faces |
| OBJ | Native | Native | Geometry and polygon triangulation; materials are not retained |
| STL | Native | Native | ASCII and binary import; binary export |
| OFF | Native | Partial | Polygon import and triangulation; export pending |
| E57 | Native | Adapter contract | Reads all point-cloud records and normalized RGB using the pure-Rust `e57` codec. The canonical `PointCloud` merges scans; transforms, images, intensities and CRS metadata require the richer spatial-project adapter before lossless write is enabled. |
| LAS | Native | Native | LAS point geometry and RGB using `las` |
| LAZ | Native | Native | Parallel compression/decompression through the `las` LAZ feature |
| glTF | Native | Native | Mesh primitives, indexed/unindexed triangles, strips and fans; geometry-only export with external BIN |
| GLB | Native | Native | Same primitive support; standards-compliant embedded binary export |
| PCD | Adapter contract | Adapter contract | Planned PCL-compatible codec |
| 3MF | Adapter contract | Adapter contract | Planned archive/model adapter |
| U3D | Adapter contract | Adapter contract | ECMA-363/U3D codec boundary; no native codec is claimed |
| STEP / IGES | Adapter contract | Adapter contract | OpenCascade boundary |
| USD / USDZ | Adapter contract | Adapter contract | OpenUSD boundary |
| FBX | Adapter contract | Adapter contract | External SDK boundary |

## CLI conversion

```powershell
cargo run -p spatial-cli -- convert-cloud scan.e57 scan.laz
cargo run -p spatial-cli -- convert-cloud scan.las scan.ply
cargo run -p spatial-cli -- convert-mesh model.glb model.stl
cargo run -p spatial-cli -- convert-mesh model.ply model.gltf
```

## Deliberate E57 constraint

E57 is a project container, not merely an XYZ stream. The current native reader provides a useful merged cloud, but lossless E57 round-tripping is disabled until `spatial-project` can preserve:

- individual scan descriptors and transforms;
- coordinate reference metadata;
- intensity and invalid-state records;
- row/column structure;
- images and camera projections;
- acquisition and sensor metadata.

This prevents a misleading writer that silently destroys professional scan context.
