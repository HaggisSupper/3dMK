# Immutable UI Contract

**Contract ID:** SEP-UI-001  
**Status:** Normative  
**Applies to:** every desktop UI surface, plugin panel, dialog, viewport overlay, and workflow added to the Spatial Engineering Platform.  
**Change control:** a modification requires an Architecture Decision Record, before/after evidence, and explicit contract-version increment.

## 1. Product class

The application is a **dense engineering workstation**, not a dashboard, consumer creator, tutorial, or marketing surface. It prioritizes spatial evidence, technical state, high-frequency commands, and deterministic workflow completion.

## 2. Non-negotiable frame

Every main window SHALL contain:

1. application menu and global command strip;
2. persistent project/entity tree;
3. central 3D viewport region;
4. context-sensitive properties/validation inspector;
5. dockable lower results/history/jobs region;
6. persistent status bar.

The viewport SHALL remain visible through normal processing workflows. Routine commands SHALL NOT replace the full workspace with wizard pages.

## 3. Selection invariant

There is exactly one authoritative selection model. Selection in the tree, viewport, results tables, measurements, processing history, and image evidence SHALL synchronize bidirectionally. A surface MAY present a filtered projection of the selection but SHALL NOT maintain a divergent hidden selection.

Every selected entity SHALL expose:

- stable identifier;
- entity type;
- source/provenance;
- coordinate frame;
- units;
- visibility/selectability state;
- validation state;
- parent/child relationships;
- available context commands.

## 4. Information hierarchy

### Always visible

- values and units;
- state and status;
- residual/error where applicable;
- confidence or uncertainty where applicable;
- pass/fail/indeterminate state;
- active coordinate frame;
- active selection count;
- active job state.

### On hover or keyboard focus

- one concise operational clarification;
- shortcut;
- admissible range or input constraint.

### On explicit expansion

- algorithm and version;
- full parameters;
- mathematical definition;
- provenance chain;
- diagnostic trace;
- failure explanation.

### Prohibited in normal panels

- architectural philosophy;
- developer instructions;
- marketing language;
- repeated teaching prose;
- sentences beginning “This feature allows…”;
- generic encouragement;
- implementation readiness notes;
- paragraphs where a state/value table is sufficient.

## 5. Command model

Commands SHALL be available through at least two of:

- menu or command strip;
- context menu;
- command palette;
- keyboard shortcut;
- contextual tool panel.

The most frequent commands for the active workspace SHALL remain visible. Secondary commands MAY move to overflow. Destructive commands SHALL be visually separated and require reversible execution or explicit confirmation when reversal is impossible.

## 6. Preview transaction

Any operation that changes derived geometry, alignment, classification, BIM state, or inspection result SHALL use this transaction:

`configure → preview → inspect metrics → accept | recalculate | cancel`

Preview artifacts SHALL be visually distinguishable and SHALL NOT mutate authoritative project state. Acceptance SHALL create a processing-history record. Cancellation SHALL leave the project unchanged.

## 7. State language

Allowed workflow states are concise and enumerable:

- Not loaded
- Ready
- Running
- Paused
- Preview
- Review
- Accepted
- Rejected
- Failed
- Unverified
- Verified
- Pass
- Fail
- Indeterminate

Free-form status prose SHALL NOT replace these states. Detailed diagnostics belong behind expansion.

## 8. Visual restraint

- Static application chrome uses neutral charcoal/grey surfaces; no blue-tinted dark theme.
- Color is reserved for data, selection, warnings, validation, and scalar visualization.
- No card-dashboard layout in technical workspaces.
- Border radius SHALL be 0–3 px for standard panels and controls.
- Drop shadows SHALL NOT be used for routine hierarchy.
- Typography SHALL use Segoe UI or system UI; data tables MAY use a tabular-numeral face.
- Headings SHALL indicate hierarchy, not consume workspace.

## 9. Error handling

Errors SHALL identify:

1. failed operation;
2. affected entity;
3. consequence;
4. recoverable action;
5. diagnostics link.

Errors SHALL NOT erase partial results without declaration. Long-running failures SHALL retain logs, parameters, and source references.

## 10. Governance gates

A UI change SHALL fail review when it:

- violates density tokens;
- introduces unsynchronized selection;
- adds explanatory prose to normal panels;
- hides required state or units;
- makes routine work modal;
- lacks keyboard operation;
- lacks empty/loading/error/disabled/focus states;
- creates an operation without preview/accept/cancel where required;
- lacks evaluation coverage in `UI_EVALUATION_MATRIX.md`.
