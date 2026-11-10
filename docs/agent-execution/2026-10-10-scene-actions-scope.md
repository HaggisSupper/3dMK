# 10 October 2026 scene add / clear UX

Status: DELIVERED_WINDOWS_CANDIDATE. This is the operator-authorized bounded continuation on MaVis's cloud VM of the 9–10 October repair exception, preserving the existing dirty repaired baseline. No product-wide completion or Windows/CUDA qualification is asserted. No public publishing or GitHub Actions is authorized.

## Approved requirements

- The existing dense Tauri/Axum/Three.js workstation SHALL retain its visual identity and every supported source/package importer.
- Persistent scene-level Add files to scene, Open model/package and Clear scene actions SHALL be reachable independently of panel expansion.
- Add files SHALL append GLBs and supported PNG/JPEG references through Rust-owned scene commands, preserving existing object state, hierarchy and immutable source bytes. Image references require a finite positive width in metres. An empty workspace requires a GLB, which MAY be included with images in the same selection. Other existing formats SHALL retain their replacement source/project workflow and be clearly distinguished from additive formats.
- Clear SHALL require a native confirmation dialog with Cancel initially focused and an explicit unsaved-workspace-loss warning. Cancel and Escape SHALL leave state unchanged.
- Confirmed Clear SHALL detach the active project and discard only local render/source cache, measurements, selection, clipping, local undo/redo, pending package review and pending preview imports. It SHALL NOT delete source files, delete persisted projects/datasets, alter the application domain, or submit an empty persisted scene.
- A clear/reset generation SHALL prevent a late parser, asset fetch, project attachment, image load or additive upload from restoring the cleared workspace. Requests already accepted by the backend are not rolled back; the UI SHALL NOT imply otherwise.
- No geometry-authority migration, framework redesign, or unrelated backend expansion is in scope.

## Acceptance evidence

New `tests/scene_actions.mjs` initially failed because the admission/lifetime helper did not exist. It now covers file/width/resource admission, Cancel/Escape, confirmed clear, stale GLB parser callbacks, awaiting project attach, clear during delayed attach, and actual source-cache/history/measurement teardown. `tests/scene_workspace_dom.mjs` additionally covers additive preservation and clear during asset upload without a later add_objects command. Syntax and existing scene DOM regressions pass in the Linux cloud environment.

Independent review, full regression, native GUI QA, packaging and delivery are owned by the coordinating executor and remain pending at this implementation handoff. JavaScript fixture tests do not qualify WebGL rendering or the reference Windows/CUDA runtime.

## Review repair batch

Independent review reproduced cancellation state leaking from Clear into the next ZIP inspection, late secondary PLY publication after Clear, and primary parser publication after Stop. The repair replaces global request cancellation checks with request-local abort state, retains the captured workspace generation through backend-derived geometry/image operations and local crop/sampling yields, discards stale decoder outputs, and invalidates pending scene/parser work on Stop without clearing already rendered content. Regressions execute the production Clear → ZIP selection → inspection → commit path, secondary PLY callback after Clear, main parser after Stop, refined-image download after Clear, and crop/sampling after-yield continuation. All pass in fixture-based Linux tests; live GUI/release acceptance remains separately owned.

Format help explicitly distinguishes GLB/GLTF/OBJ/PLY/STL preview support from preserved backend-import-only formats (FBX/DAE/3DS/3MF/OFF/U3D/X3D). The empty-state Add action has the same primary treatment as the persistent Add action.

A follow-up review reproduced startup health reporting Offline when Add/Open/Clear changed the workspace generation before the one-time health response returned. Health and VLM capability probes now explicitly use non-workspace request scope: they keep their own timeout controller, never replace the cancellable scene controller, and remain valid across scene changes. A production-function fixture with delayed health and VLM responses passes.

## Final validation — 11 October 2026

Independent review passed. All199 Rust tests,9 JavaScript suites,strict Clippy and formatting pass. Final native Linux GUI verified additive multiselect and append (359 to562objects,all existing objects unchanged), Cancel/Escape,confirmed Clear with10 persisted project/domain files unchanged, and Clear→ZIP inspect→commit rendering all three GLB roots. Original4K screenshots retained. Windows candidate is unsigned and runtime untested; previously reported Defender warning remains unresolved.
