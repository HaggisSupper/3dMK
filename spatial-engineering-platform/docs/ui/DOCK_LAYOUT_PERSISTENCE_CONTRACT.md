# Dock Layout Persistence Contract

**Contract ID:** SEP-UI-DOCK-PERSISTENCE  
**Status:** Immutable normative  
**Applies to:** Desktop workstation shell, docking manager, floating tool windows, workspace switching, application startup and shutdown

## 1. Non-negotiable invariant

The application **must persist the complete valid dock layout across sessions and restore it before the first interactive frame**.

The primary viewport is not a dockable panel. It is the fixed spatial authority. All tool views are children of the workstation shell and derive screen alignment, selection context, and spatial projection from the primary viewport's published state.

A persisted layout must never:

- close, float, tab, auto-hide, or relocate the primary viewport;
- create a second authoritative selection or camera state;
- place a required panel outside every available monitor;
- restore a panel into a dock region prohibited by its panel contract;
- silently discard a user's last known valid layout;
- block startup because a layout file is missing, corrupt, stale, or incompatible.

## 2. Persisted state

The persistence system must retain all user-controlled layout state that affects task continuity.

### 2.1 Shell and workspace

- schema version;
- application version;
- active workspace;
- active layout profile;
- main-window bounds and maximized/fullscreen state;
- monitor topology and device-pixel ratio at save time;
- last successful restoration timestamp.

### 2.2 Dock panels

For every registered panel:

- stable panel identifier;
- dock region;
- tab group;
- tab order;
- active-tab state;
- visibility;
- pinned or auto-hidden state;
- floating state;
- floating-window logical bounds;
- floating-window monitor identifier;
- minimum and preferred dimensions;
- panel-specific persisted state version.

### 2.3 Splitters and regions

- split orientation;
- normalized split ratios;
- left and right dock widths;
- bottom dock height;
- nested group relationships;
- collapsed state where permitted.

### 2.4 Context continuity

- expanded project-tree nodes;
- selected entity identifiers;
- active result tab;
- table column order and widths;
- viewport camera state;
- active coordinate frame;
- visibility and clipping state;
- active non-destructive tool, where safe to restore.

Transient preview artifacts, pointer capture, drag state, modal state, running destructive commands, and uncommitted edits must not be restored as authoritative state.

## 3. Docking constraints

Panel placement is constrained by registry policy.

| Surface | Allowed placement |
|---|---|
| Primary viewport | Fixed center only; never represented as a dock panel |
| Project tree | Left dock; optional floating only when explicitly enabled by policy |
| Properties and validation | Right dock |
| Jobs, processing history, output and measurements | Bottom dock |
| Images and evidence | Right or bottom dock; floating allowed |
| Secondary comparison viewport | Explicit secondary-view contract only; never replaces primary authority |
| Global command strip | Fixed top |
| Status bar | Fixed bottom edge |

A docking library's permissive defaults do not override this contract.

## 4. Save protocol

### 4.1 Save triggers

A save must be scheduled after:

- dock or undock completion;
- tab reorder completion;
- splitter release;
- panel show, hide, pin, auto-hide, float, close, move, or resize completion;
- workspace or layout-profile change;
- tree expansion change;
- table-layout change;
- main-window move, resize, maximize, restore, or monitor transition;
- application close;
- periodic recovery checkpoint while dirty.

Continuous pointer-move events must not cause disk writes.

### 4.2 Debounce

- Default settle debounce: **500 ms**.
- Maximum unsaved interval while layout state is dirty: **10 seconds**.
- Application-close save bypasses debounce and must attempt synchronous completion within the shutdown budget.

### 4.3 Atomic write

The implementation must:

1. serialize to a sibling temporary file;
2. flush the file contents;
3. validate the serialized payload;
4. atomically replace `current.json`;
5. retain the previous validated file as `last-known-good.json`;
6. never overwrite both current and last-known-good with the same unvalidated payload.

## 5. Restore protocol

Restore must occur after panel registration and monitor discovery but before user interaction is enabled.

Required order:

1. Load `current.json`.
2. Parse and validate against the active schema.
3. Run version migrations sequentially.
4. Enforce viewport and panel placement invariants.
5. Reconcile saved monitors against current monitor topology.
6. Clamp and scale floating bounds.
7. Apply shell regions, split ratios, tabs, and panels.
8. Recalculate primary viewport bounds.
9. Restore contextual state only for entities that still exist.
10. Publish a single `layout-restored` event.
11. Enable interaction.

If any stage fails, repeat using `last-known-good.json`. If that fails, use the shipped default layout. Startup must continue.

## 6. Monitor and DPI reconciliation

- Geometry is stored in logical pixels with the save-time scale recorded.
- Physical-pixel rendering and spatial overlays are recomputed using the current monitor scale.
- Floating windows must be clamped so that at least 64 logical pixels of title/header area remain reachable.
- A window saved on a disconnected monitor moves to the nearest current monitor, preferring the primary monitor.
- Split ratios are normalized, not stored solely as physical pixels.
- A monitor identifier is advisory; position and scale reconciliation must tolerate identifier changes.

## 7. Versioning and migration

- Layout schema uses an integer `schema_version`.
- Every persisted panel owns an integer `state_version`.
- Migrations are deterministic, ordered, side-effect free, and tested with frozen fixtures.
- Unknown optional fields are ignored and preserved where practical.
- Unknown panel IDs are quarantined, not fatal.
- Missing required panels are restored from registry defaults.
- Downgrade is not guaranteed; incompatible newer layouts fall back without overwriting the source file.

## 8. Storage location

Use the platform application-data directory resolved by Tauri. On Windows the effective structure is expected to be equivalent to:

```text
%APPDATA%\SpatialEngineeringPlatform\layouts\
├── current.json
├── last-known-good.json
├── default.json
├── quarantine\
│   └── <timestamp>-invalid.json
└── workspaces\
    ├── view.json
    ├── clean.json
    ├── register.json
    ├── segment.json
    ├── reconstruct.json
    ├── bim.json
    └── inspect.json
```

No absolute user path may be compiled into the application.

## 9. Failure behavior

Layout failures are non-destructive operational errors.

- The user receives a concise status notification: `Layout restored from recovery copy` or `Default layout restored`.
- Full parse, migration, and validation details are available in diagnostics, not sprinkled through normal panels.
- Invalid layouts are copied to quarantine with a reason record.
- A failed layout restore must never corrupt project data.
- Repeated restore failure must not create a startup loop.

## 10. Security and integrity

- Reject path traversal and external file references in layout payloads.
- Reject non-finite numeric values.
- Enforce maximum panel, tab, splitter, and tree-node counts.
- Enforce bounded strings and identifiers.
- Never execute commands, scripts, URLs, or arbitrary component names from a layout file.
- Resolve panels only through the compiled panel registry.

## 11. Acceptance criteria

A persistence implementation is complete only when automated and manual evidence demonstrates:

1. exact layout restoration after normal restart;
2. recovery after truncation and malformed JSON;
3. recovery after forced termination during save;
4. restoration after monitor removal;
5. restoration after DPI changes;
6. restoration after application schema migration;
7. preservation of viewport authority;
8. rejection of illegal panel placement;
9. restoration before first interactive frame;
10. no repeated writes during active splitter or window drag;
11. last-known-good survival after an invalid current save;
12. deterministic default-layout fallback.
