# Scene and panel candidate

This is a bounded continuation of the 3DMK repair on the cloud VM, authorized on 9 October 2026. It preserves the Tauri/Axum/Three.js architecture. It is not a qualification of the repository's broader CUDA roadmap.

## Controls

Workspace, View and Help use one shared panel controller. Toolbar buttons toggle visibility. Pin prevents automatic dismissal; the close button explicitly hides a panel while retaining its pin preference. Escape closes the most recently opened unpinned panel and restores trigger focus. Two visible right-hand panels split the available height. Below 900 CSS pixels only the active panel is shown, without destroying the other panels' preferences. Restart restores visibility and pin preferences. Labels and panel headers are shortened.

## Scene composition

The Rust-owned scene document contains UUID object/asset references, groups, parent relationships, local metre transforms, roles, visibility, lock state, opacity, named camera/clipping/section views and durable Undo/Redo. Commands have expected revisions and immutable command receipts. A retried command cannot apply twice. A command ID reused with different contents, stale revision, missing asset, invalid number/quaternion, zero scale, hierarchy cycle or locked mutation fails before persistence.

Immutable source assets and editable instances are distinct. Scene instances share source geometry while cloning materials. Project save/reopen retains the full scene state, history and command receipts. Existing divergent state causes a conflict rather than an overwrite. Named-view activation captures the previous unsaved camera so Undo can restore it after reopening. Scene assets and state are included in the existing Rust ZIP package, not a browser-reconstructed replacement.

The UI provides a scene tree, numeric transform editing, groups, duplicate instances, parent/role/name/lock/opacity controls, GLB/image assets and named camera/visibility/clipping views. Images require a width in metres. Raw geometry export remains source-only; project save retains composition and viewport PDF is a presentation snapshot.

Bounds: 4,096 live objects, 64 named views, 32 Undo/Redo entries each, 10,000 retained command receipts, 32 MiB state; frontend preview 64 unique assets / 512 MiB source bytes / local coordinates within 100 km. Receipt exhaustion fails explicitly. This is a single-app store-lock design, not a multi-process collaborative editor.

## Technical boundaries

- Composition accepts embedded GLB and PNG/JPEG reference assets. Large-origin/georeferenced assets remain usable in Source geometry but combining them requires a future shared registration workflow. Origins are never silently discarded. The current scene frame is project-local metres; unresolved CRS/datum is disclosed.
- Section-view data is validated and persisted by the backend. The UI currently saves camera and clipping views; it does not author a full named geological-section definition or compute field survey intersections.
- The original synthetic fixture has separate planned/actual well, top and section objects. Piecewise-linear XYZ intersections are checked numerically. Ribbons are schematic and do not claim physical borehole diameter. This is a deterministic test dataset, not field data or a new production converter.
- Browser source-geometry edits still require the earlier explicit derived-export workflow. Scene Undo is separate from source-preview Undo.
- Cloud Chromium cannot initialize WebGL. Live panel-shell interactions were tested, but 3D rendering, picking and scene interaction were not visually qualified. Browser evidence is 3656×1893 pixels. The native screenshot service returned only a 1364×1024 region despite 3840×2160 window metadata, so no true 4K screenshot pass is claimed.
- Windows x86-64 GNU cross-compilation succeeded. PE GUI architecture and all nine icon frames were inspected. Windows launch, WebView2 runtime behavior and clean-machine installation remain untested. The previous release remains unchanged.

## Independence and test dependencies

Implementation was written from the approved 3DMK proposal, existing 3DMK contracts and original tests. No ArtCraft implementation, assets or interface text was read or copied. The scene-contract worker began without exposure to that source. Existing Three.js 0.160.0 is MIT; Linkedom 0.18.12 is ISC and used only for DOM unit tests; Python pefile 2024.8.26 is MIT and used only for binary inspection. No new production rendering engine was added.

## Verification

- Root Rust suite: 143 passing tests, including 24 scene-contract tests and 7 exchange/cancellation/fixture tests.
- VWM IO: 10 passing loader regressions, including original JSON double precision and large-coordinate centimetres.
- Node tests: panel interaction matrix; crop/material/attribute preservation; immutable scene instances; original well/top/section fixture; DOM request races, invalid preview recovery, bootstrap/reparent range checks, hierarchy cycles and detach.
- Live supported cloud-browser panel tests: paired opening, pin/unpin, close/reopen, Escape and reload. Narrow-panel behavior is unit-tested, not live-window-qualified.

Run JS tests using local test dependencies:

```
npm install --prefix /path/to/test-deps --save-exact three@0.160.0 linkedom@0.18.12
export THREE_MODULE_PATH=/path/to/test-deps/node_modules/three/build/three.module.js
export LINKEDOM_MODULE_PATH=/path/to/test-deps/node_modules/linkedom/esm/index.js
node tests/geometry_integrity.mjs
node tests/workspace_panels.mjs
node tests/scene_workspace.mjs
node tests/scene_workspace_dom.mjs
node tests/scene_fixture.mjs
```

See REPAIR-2026-10-09.md for the Veritas pinned-source Cargo override and prior exchange/seismic boundaries.
