# VWM Advanced Workflows

## Detached mesh review

VWM classifies disconnected geometry as **Remove**, **Uncertain**, or **Keep** candidates. The viewer highlights removable components in red, uncertain components in amber, and the current selection in cyan.

- Select one or more components in the detached-mesh list or right-click a highlighted component in the 3D canvas.
- **Hide** is reversible and does not delete geometry. Hidden component state is retained when edits are saved.
- **Stage deletion** marks exact connected triangles or points for removal without immediately mutating the loaded model.
- **Save edits** applies every staged deletion in one undoable model edit. It squashes deleted geometry from the working model while retaining hidden-state metadata.
- **Discard staged edits** clears pending deletion marks without changing the model.
- **Undo** restores the previous geometry and review state. **Redo** reapplies the saved edit.

Deletion must operate on exact connected-component membership. Axis-aligned cluster bounds may be used for visualization or initial candidate discovery, but must not remove unrelated geometry that happens to overlap a candidate's bounds.

## Calibrated photo projection

Calibrated projection uses the camera intrinsics and `world_from_camera` transforms supplied by an imported iPhone capture package. It projects accepted photographs onto mesh or point-cloud vertices and writes confidence-weighted vertex colors.

Controls:

- **Maximum photos** limits work and memory usage.
- **Occlusion tolerance** controls how far a projected sample may sit behind the nearest observed surface depth.
- **Minimum observations** requires multiple agreeing views before replacing a vertex color.
- **Original color blend** retains a configurable contribution from existing vertex color.

The result preserves vertex positions and mesh topology. The report identifies used, rejected, and unmatched photographs plus vertex coverage. Projection is disabled when calibrated camera/image pairs are unavailable or when the model no longer uses the capture coordinate frame.

This feature does not claim texture-atlas baking, photogrammetric geometry displacement, depth fusion, or mesh reconstruction from photographs.

## VWM perception

The perception panel sends a selected reference image or viewport capture to the VWM perception endpoint and lists the returned regions, labels, confidence values, and inference provenance.

The application reports whether inference used a packaged ONNX model or the deterministic region/shape fallback. Fallback output is exploratory assistance, not a claim of model-backed semantic recognition.

## Save and package persistence

Explicit **Save edits** is the commit boundary for destructive detached-geometry changes. Package manifests persist hidden detached-component identifiers and record the calibrated projection report as provenance. Pending deletions are review state and are not represented as completed model edits until Save is invoked.
