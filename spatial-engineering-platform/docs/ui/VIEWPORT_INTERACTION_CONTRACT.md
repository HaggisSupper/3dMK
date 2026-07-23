# Viewport Interaction Contract

**Contract ID:** SEP-UI-004

## 1. Navigation defaults

Default controls SHALL match common desktop 3D workstation expectations and be remappable:

- orbit: middle-button drag;
- pan: Shift + middle-button drag;
- zoom: wheel or Ctrl + middle-button drag;
- frame selection: `F`;
- frame all: `Home`;
- orthographic/perspective toggle: numpad `5` or mapped equivalent;
- standard views: numpad/view-cube commands;
- cancel active tool: `Esc`;
- accept active tool: `Enter`.

A preset SHALL be provided for Blender, Autodesk, and CAD-style navigation. The active preset is project-independent user preference.

## 2. Picking

Pick result SHALL expose entity, sub-element, coordinates, normal, color/scalar value, source index, and uncertainty when available. Selection priority SHALL be deterministic and configurable for points, vertices, edges, faces, segments, and engineering objects.

Dense clouds require depth-aware picking, nearest-point tolerance, and a magnified pick inset. The inset SHALL not obscure the target and SHALL show the exact selected sample.

## 3. Tool modes

Only one exclusive pointer tool is active per viewport. Tool state is visible in the viewport header and status bar. Navigation remains available through modifier keys or paused mode. Exiting a tool restores the previous selection tool.

## 4. Visibility

Every visible entity supports:

- show/hide;
- isolate;
- ghost/x-ray where meaningful;
- selectability lock;
- render mode;
- opacity;
- clipping participation.

Visibility state in the tree and viewport SHALL be identical.

## 5. Rendering

Point clouds SHALL support RGB, intensity, height, classification, scalar field, residual, confidence, and uniform color where data exists. Meshes SHALL support solid, textured, vertex color, wireframe, normals, and deviation modes.

Every data-driven color mode SHALL show legend, units, range, clamp state, invalid/missing treatment, and palette name.

## 6. Sections and clipping

Clipping plane, box, and section tools SHALL be non-destructive viewport objects. They may be saved, named, and reused for extraction or reporting. Numeric position/orientation controls SHALL be available as an alternative to dragging.

## 7. Measurement

Measurement modes include point, distance, angle, area, radius/diameter, point-to-plane, section, and clearance where supported. Every drag-based placement SHALL also permit point picking and numeric adjustment. Labels SHALL avoid occluding source geometry and may be pinned.

## 8. Performance feedback

Status bar SHALL show loaded points/triangles, displayed points/triangles, active LOD, GPU backend, and viewport frame time on demand. Performance degradation SHALL reduce rendering detail, not silently alter analysis data.
