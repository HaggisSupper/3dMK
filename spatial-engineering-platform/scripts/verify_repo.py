from pathlib import Path
root = Path(__file__).resolve().parents[1]
required = [
    "Cargo.toml",
    "crates/format-io/Cargo.toml",
    "crates/format-io/src/lib.rs",
    "crates/format-contracts/src/lib.rs",
    "crates/pointcloud-core/src/lib.rs",
    "crates/mesh-core/src/lib.rs",
    "crates/world-model-contracts/src/lib.rs",
    "crates/ui-layout-contracts/Cargo.toml",
    "crates/ui-layout-contracts/src/lib.rs",
    "docs/FORMAT_SUPPORT.md",
    "docs/ui/UI_CONTRACT.md",
    "docs/ui/NAVIGATION_CONTRACT.md",
    "docs/ui/UI_EVALUATION_MATRIX.md",
    "docs/ui/DOCK_LAYOUT_PERSISTENCE_CONTRACT.md",
    "docs/ui/LAYOUT_RECOVERY_AND_MIGRATION.md",
    "docs/ui/schemas/dock-layout.schema.json",
    "docs/ui/layouts/default-engineering-layout.json",
    "docs/ui/ui-contract.json",
]
missing = [p for p in required if not (root / p).exists()]
if missing:
    raise SystemExit(f"missing required paths: {missing}")
source = (root / "crates/format-io/src/lib.rs").read_text(encoding="utf-8")
required_symbols = [
    "pub fn read_e57",
    "pub fn read_las",
    "pub fn write_las",
    "pub fn read_ply_points",
    "pub fn write_binary_ply_points",
    "pub fn read_stl",
    "pub fn write_binary_stl",
    "pub fn read_gltf",
    "pub fn write_gltf",
    "pub fn write_glb",
]
missing_symbols = [s for s in required_symbols if s not in source]
if missing_symbols:
    raise SystemExit(f"missing format adapter symbols: {missing_symbols}")
registry = (root / "crates/format-contracts/src/lib.rs").read_text(encoding="utf-8")
for ext in ["e57", "las", "laz", "ply", "gltf", "glb", "stl"]:
    if f'"{ext}"' not in registry:
        raise SystemExit(f"format registry missing {ext}")
import subprocess, sys
subprocess.run([sys.executable, str(root / "scripts/verify_ui_contract.py")], check=True)
print("repository structure, adapters, and immutable UI contract verification passed")
