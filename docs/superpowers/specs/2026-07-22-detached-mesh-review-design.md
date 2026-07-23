# Detached Mesh Review and Save Edits

## Scope

Add an end-user review workflow for VWM detached-geometry findings. This change covers highlighted detached components, multi-selection, canvas context actions, staged deletion, reversible hiding, and an explicit save boundary. Calibrated photo-to-geometry refinement is a separate subsystem.

## Interaction model

- VWM analysis continues to classify detached geometry as removable or uncertain.
- Every finding appears as color-highlighted geometry and as a row in a Detached meshes list.
- Removable findings use red, uncertain findings use amber, and selected findings use cyan.
- Users can select one or many rows, select all, clear the selection, hide, show, or stage deletion.
- Right-clicking highlighted geometry opens a canvas menu for Hide, Show, or Delete.
- Hide changes visibility immediately without removing geometry.
- Delete marks a component as Pending deletion; geometry is not changed yet.
- Save edits applies every pending deletion to the active model in one undoable operation.
- Saving retains the hidden state of components that were not deleted.
- Discard staged clears pending deletions without changing hidden state or model geometry.

## Data and rendering

The browser keeps three review sets keyed by the VWM cluster identifier: selected, hidden, and pending deletion. Floating review geometry carries its cluster identifier for raycasting. Base scene rendering excludes hidden and pending-delete bounds without changing the loaded geometry store. Save edits filters all pending-delete regions from the loaded geometry store once, updates the VWM analysis summary, clears committed review identifiers, and rebuilds the scene.

The existing undo snapshot is taken immediately before Save edits. Individual Hide and Delete staging actions do not create geometry snapshots because they do not mutate the model.

## Persistence

The active package manifest records hidden and pending review state. Save edits removes committed deletions from the analysis context while retaining hidden identifiers. ZIP project export includes this review state so hidden findings can remain hidden when the package is reopened.

## Error handling

- Controls remain disabled until VWM analysis produces detached findings.
- Save edits is disabled when there are no pending deletions.
- Missing or stale cluster identifiers are ignored and removed from review state during rerender.
- If filtering would remove all geometry, Save edits is rejected and the active model remains unchanged.

## Verification

- Frontend contract tests assert the list, context menu, staged actions, and Save edits controls exist.
- Browser self-tests cover multi-selection, hidden-state retention, staging without mutation, and one-shot commit behavior.
- The Rust/frontend test suite and release build must pass before handoff.
