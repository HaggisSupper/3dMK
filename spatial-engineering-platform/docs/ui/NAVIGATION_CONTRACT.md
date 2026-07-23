# Navigation Contract

**Contract ID:** SEP-UI-002

## 1. Navigation model

Navigation is both **object-centric** and **workspace-aware**.

- The project tree answers: *what exists?*
- The workspace selector answers: *what operation am I performing?*
- The inspector answers: *what is true about the current selection?*
- The history/results region answers: *what happened and what evidence supports it?*

These roles SHALL NOT be conflated.

## 2. Permanent project tree

The tree root SHALL use this canonical order:

```text
Project
├── Coordinate Systems
├── Sources
├── Point Clouds
├── Meshes
├── Images
├── Registrations
├── Segments
├── Engineering Objects
├── CAD
├── BIM
├── Measurements
├── Inspections
└── Processing History
```

Empty branches MAY be collapsed but SHALL retain stable identifiers and ordering. Extensions SHALL register under an existing branch unless an ADR establishes a genuinely new domain.

## 3. Tree row contract

Each row MAY show:

- disclosure control;
- type icon;
- name;
- concise state badge;
- visibility toggle;
- selectability lock;
- warning/error marker.

The row SHALL NOT show explanatory sentences. Tree row height is fixed by the density contract. Right-click exposes context commands. Double-click frames or opens the entity according to entity type. `F2` renames where permitted. `Delete` invokes reversible delete or confirmation.

## 4. Selection synchronization

- Viewport pick selects the corresponding tree entity.
- Tree selection highlights the entity in all visible viewports.
- Results-table selection highlights source geometry.
- Processing-history selection highlights inputs and outputs without changing authoritative selection unless the user activates “Select outputs”.
- `Ctrl` adds/removes selection; `Shift` selects a contiguous tree range; marquee/lasso updates the same selection model.

## 5. Workspace selector

Canonical workspaces:

```text
VIEW | CLEAN | REGISTER | SEGMENT | RECONSTRUCT | BIM | INSPECT
```

Workspace switching SHALL preserve:

- project and selection;
- viewport camera;
- visibility state;
- open panels;
- unsaved parameter drafts when safe.

Workspace switching changes command surfaces and inspector sections; it SHALL NOT rebuild the application shell.

## 6. Search and command navigation

- `Ctrl+F` focuses tree/entity search when the tree has focus.
- `Ctrl+K` opens the command palette globally.
- Search SHALL filter by name, type, stable ID, source file, classification, and validation state.
- Search results SHALL preserve hierarchy context or provide a reveal-in-tree action.
- Commands SHALL advertise shortcuts in the palette and menus.

## 7. Breadcrumbs

Breadcrumbs are used only for nested editors or isolated context, such as:

`Project / Engineering Objects / Pump-04 / Discharge Flange`

Breadcrumbs SHALL NOT duplicate the permanent project tree for ordinary selection.

## 8. Multi-view navigation

The user MAY open multiple synchronized or independent 3D views. Each view stores:

- camera;
- projection;
- clipping state;
- rendering mode;
- isolated entities;
- synchronized-selection flag.

Saved views are project entities and may be reused by segmentation, inspection, and reporting workflows.
