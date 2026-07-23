# Detached Mesh Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give end users a color-highlighted, multi-select detached-mesh review workflow with reversible hiding, staged deletion, a right-click canvas menu, and an explicit Save edits commit.

**Architecture:** Keep review state in three cluster-ID sets: selected, hidden, and pending deletion. Render hidden and pending geometry out of the base scene without mutating `loadedGeometryStore`; Save edits performs one filtered geometry commit and one undo snapshot. Persist review state in the 3DMk ZIP manifest.

**Tech Stack:** Tauri 2, static HTML/CSS/JavaScript, Three.js raycasting and BufferGeometry, Rust frontend contract tests.

## Global Constraints

- Hide never deletes geometry.
- Delete only stages a deletion.
- Save edits applies all staged deletions as one undoable model edit.
- Save edits retains hidden state for components not deleted.
- Right-click actions operate only on VWM-highlighted detached geometry.
- Existing model import, analysis, undo, export, and responsive UI behavior must remain intact.

---

### Task 1: Add failing frontend contracts

**Files:**
- Modify: `tests/frontend_shell.rs`

**Interfaces:**
- Consumes: `public/index.html` through `include_str!`.
- Produces: static contracts for the review list, context menu, staged state, and commit functions.

- [ ] **Step 1: Add contract assertions**

```rust
assert!(html.contains("id=\"floatingMeshReviewList\""));
assert!(html.contains("id=\"saveFloatingEditsBtn\""));
assert!(html.contains("id=\"discardFloatingEditsBtn\""));
assert!(html.contains("id=\"floatingMeshContextMenu\""));
assert!(html.contains("function renderFloatingMeshReviewList"));
assert!(html.contains("function stageFloatingClusterDeletion"));
assert!(html.contains("function saveFloatingMeshEdits"));
assert!(html.contains("function handleFloatingMeshContextMenu"));
```

- [ ] **Step 2: Run the focused test and confirm it fails**

Run: `cargo test --test frontend_shell`

Expected: failure on the first missing detached-mesh review contract.

- [ ] **Step 3: Commit the failing contract**

```powershell
git add tests/frontend_shell.rs
git commit -m "test: define detached mesh review contract"
```

### Task 2: Add review controls and accessible canvas menu

**Files:**
- Modify: `public/index.html`

**Interfaces:**
- Consumes: existing Scene analysis workflow card and `action-btn` styles.
- Produces: `floatingMeshReview`, `floatingMeshReviewList`, selection/action buttons, `floatingMeshContextMenu`, and menu action buttons.

- [ ] **Step 1: Add review markup under Scene analysis**

```html
<section id="floatingMeshReview" class="floating-review" hidden aria-labelledby="floatingMeshReviewHeading">
  <div class="analysis-object-head">
    <h4 id="floatingMeshReviewHeading">Detached meshes</h4>
    <span id="floatingMeshReviewCount" class="analysis-object-score">0 found</span>
  </div>
  <div class="floating-review-toolbar">
    <button id="selectAllFloatingBtn" class="action-btn" type="button">Select all</button>
    <button id="clearFloatingSelectionBtn" class="action-btn" type="button">Clear</button>
  </div>
  <div id="floatingMeshReviewList" class="floating-review-list" role="list"></div>
  <div class="floating-review-toolbar">
    <button id="hideSelectedFloatingBtn" class="action-btn" type="button">Hide selected</button>
    <button id="showSelectedFloatingBtn" class="action-btn" type="button">Show selected</button>
    <button id="deleteSelectedFloatingBtn" class="action-btn danger" type="button">Stage deletion</button>
  </div>
  <button id="saveFloatingEditsBtn" class="action-btn primary" type="button" disabled>Save edits</button>
  <button id="discardFloatingEditsBtn" class="action-btn" type="button" disabled>Discard staged</button>
  <div id="floatingMeshEditStatus" class="status-line" aria-live="polite">No staged edits.</div>
</section>
```

- [ ] **Step 2: Add the fixed canvas context menu**

```html
<div id="floatingMeshContextMenu" class="canvas-context-menu" role="menu" hidden>
  <div id="floatingContextTitle" class="canvas-context-title">Detached mesh</div>
  <button id="contextHideFloatingBtn" type="button" role="menuitem">Hide</button>
  <button id="contextShowFloatingBtn" type="button" role="menuitem">Show</button>
  <button id="contextDeleteFloatingBtn" type="button" role="menuitem" class="danger">Stage deletion</button>
</div>
```

- [ ] **Step 3: Add focused styling**

Define selected, hidden, pending, removable, and uncertain row states; compact two-column toolbars; and a high-z-index fixed context menu constrained to the viewport.

- [ ] **Step 4: Run the focused contract**

Run: `cargo test --test frontend_shell`

Expected: markup assertions pass; behavior-function assertions remain failing.

### Task 3: Implement staged review state and rendering

**Files:**
- Modify: `public/index.html`

**Interfaces:**
- Consumes: `floatingObjectEntries`, `clusterBounds`, `subsetGeometryByBounds`, `renderSceneAnalysisOverlay`, `rebuildMeshes`, and `currentModelPackage.analysisContext`.
- Produces: `selectedFloatingClusterIds`, `hiddenFloatingClusterIds`, `pendingFloatingDeletionIds`, `renderFloatingMeshReviewList(analysis)`, `setFloatingClustersHidden(ids, hidden)`, `stageFloatingClusterDeletion(ids)`, and `discardFloatingMeshEdits()`.

- [ ] **Step 1: Add review state**

```javascript
let selectedFloatingClusterIds = new Set();
let hiddenFloatingClusterIds = new Set();
let pendingFloatingDeletionIds = new Set();
let floatingContextClusterId = null;
```

- [ ] **Step 2: Make highlighted objects raycastable by cluster**

For each floating mesh or point object, set `object.userData.floatingClusterId = cluster.id`, retain its base color, and update color/opacity/visibility from selected, hidden, and pending state.

- [ ] **Step 3: Render the multi-select list**

Create one checkbox row per `Remove` or `Uncertain` cluster. Show its point count, confidence, classification, and state badge. Checkbox changes update `selectedFloatingClusterIds` and refresh visual highlighting without rerunning VWM analysis.

- [ ] **Step 4: Render without mutating the model**

Add `geometryForCurrentReview(source)` that sequentially excludes bounds for IDs in `hiddenFloatingClusterIds` and `pendingFloatingDeletionIds`. Use it only inside `rebuildMeshes`; keep `loadedGeometryStore` unchanged until Save edits.

- [ ] **Step 5: Implement list actions**

`Hide selected` adds selected IDs to hidden state. `Show selected` removes them. `Stage deletion` adds selected IDs to pending state. `Discard staged` clears pending state. Every action rebuilds the visible scene, redraws overlays, updates the list, and updates button availability.

### Task 4: Implement canvas context actions and Save edits

**Files:**
- Modify: `public/index.html`

**Interfaces:**
- Consumes: Three.js `raycaster`, `activeCamera`, `renderer.domElement`, `saveState`, `processGeometries`, and Task 3 review state.
- Produces: `handleFloatingMeshContextMenu(event)`, `hideFloatingMeshContextMenu()`, `saveFloatingMeshEdits()`, and batch analysis filtering.

- [ ] **Step 1: Raycast highlighted geometry on contextmenu**

Convert the pointer to normalized device coordinates relative to `renderer.domElement`, raycast `floatingObjectLayerGroup`, resolve `floatingClusterId` from the hit or its ancestors, and open the menu at a viewport-clamped position. Close it on outside pointerdown, Escape, resize, or an action.

- [ ] **Step 2: Wire menu actions**

Hide/Show modifies only `hiddenFloatingClusterIds`. Stage deletion adds the target to `pendingFloatingDeletionIds`. Opening the menu selects the target if it is not already part of the multi-selection.

- [ ] **Step 3: Commit staged deletion once**

`saveFloatingMeshEdits()` validates the pending IDs, filters their bounds from every geometry, rejects an empty result, takes one `saveState()` snapshot, updates the analysis context and artifact summary, clears deleted IDs from all review sets, calls `processGeometries`, and preserves remaining hidden IDs.

- [ ] **Step 4: Bind controls and lifecycle cleanup**

Bind every new button and the canvas `contextmenu` event during initialization. Clear review state when a new model is loaded or analysis is invalidated. Reconcile stale IDs whenever analysis changes.

### Task 5: Persist review state in packages

**Files:**
- Modify: `public/index.html`

**Interfaces:**
- Consumes: `buildModelPackage()` and `applyLoadedPackageManifest()`.
- Produces: `manifest.review.detached_meshes.hidden_cluster_ids` and `pending_deletion_cluster_ids`.

- [ ] **Step 1: Export review state**

```javascript
detached_meshes: {
  hidden_cluster_ids: [...hiddenFloatingClusterIds],
  pending_deletion_cluster_ids: [...pendingFloatingDeletionIds]
}
```

- [ ] **Step 2: Restore review state**

Read numeric IDs from `manifest.review.detached_meshes`, ignore invalid values, and apply them after the package analysis context is restored.

- [ ] **Step 3: Add browser self-test contracts**

Assert staging does not alter `loadedGeometryStore`, Save edits reduces geometry exactly once, Discard preserves geometry, and hidden IDs survive Save edits.

### Task 6: Validate and build

**Files:**
- Modify: `tests/frontend_shell.rs`
- Modify: `docs/3dmk-vwm-feature-merge-plan.md`

**Interfaces:**
- Consumes: completed detached-mesh workflow.
- Produces: verified frontend contracts, updated feature status, and a Tauri 2 release executable.

- [ ] **Step 1: Run format checks**

Run: `cargo fmt --all -- --check`

Expected: exit code 0.

- [ ] **Step 2: Run focused and workspace tests**

Run: `cargo test --test frontend_shell`

Run: `cargo test --workspace`

Expected: all tests pass.

- [ ] **Step 3: Build the Tauri release executable**

Run from `src-tauri`: `cargo tauri build --no-bundle`

Expected: `src-tauri/target/release/three-dmk-desktop.exe` is produced.

- [ ] **Step 4: Update architecture state when available**

Run: `graphify update C:\Development\3dMK`

Expected: bounded repository mapping succeeds. If `graphify` is unavailable, record that fact in the handoff.

- [ ] **Step 5: Commit the feature**

```powershell
git add public/index.html tests/frontend_shell.rs docs/3dmk-vwm-feature-merge-plan.md docs/superpowers/specs/2026-07-22-detached-mesh-review-design.md docs/superpowers/plans/2026-07-22-detached-mesh-review.md
git commit -m "feat: add staged detached mesh review"
```
