# Calibrated Photo Projection Design

## Goal

Use validated iPhone capture camera records and their paired photographs to improve mesh or point-cloud appearance with confidence-weighted vertex colors. Keep the existing uncalibrated texture contrast pass clearly separate.

## Eligibility

Projection is available only when the imported Rust package inventory contains an `iphone_capture` report with valid calibration and at least one complete camera/image pair. Photos are paired by preserved package path, with basename matching used only when unambiguous. Uncalibrated images remain available for texture quality assessment but cannot enter geometry projection.

## Projection

Rust loads the submitted ASCII PLY as a canonical VWM scene. Each valid camera projects vertices into its image using the documented `world_from_camera` convention. A bounded per-camera depth grid rejects occluded samples. Accepted samples are weighted by image quality, viewing angle, and depth, then blended across views. Existing vertex color can be retained with a user-selected weight.

The operation changes colors only. It preserves vertex positions, triangle indices, and normals. It does not claim geometry displacement, depth fusion, texture atlas baking, or general photogrammetry.

## Controls

- Maximum calibrated photos: 4, 8, 16, 32, or 64.
- Occlusion tolerance in metres.
- Minimum independent observations per vertex.
- Original vertex-color blend from 0 to 100 percent.
- Explicit Project calibrated photos button.
- Coverage, used/rejected photo counts, and camera-pair status.

## Output and history

The backend writes a colored PLY and a projection report. The UI loads the result as one undoable geometry edit while retaining the capture inventory and camera frame. Package export preserves source photos, calibration, and the projection report status.

## Verification

Synthetic camera tests prove pixel projection, front-surface occlusion, color output, and uncovered fallback. Frontend contracts prove every control and API path is exposed. Workspace tests, JavaScript syntax, and the Tauri release build must pass.
