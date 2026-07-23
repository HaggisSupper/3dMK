# UI Evaluation Matrix

**Contract ID:** SEP-UI-010  
**Rule:** no UI feature is complete until its applicable rows have evidence attached.

## Required evidence per element

Each control, panel, dialog, menu, tree row, viewport tool, and workflow receives a record containing:

- stable test ID;
- owner;
- applicable contract clauses;
- normal, hover, focus, active, disabled, loading, empty, warning, error states;
- keyboard path;
- pointer path;
- scaling evidence;
- screenshot baseline;
- automated test reference;
- manual review disposition.

## Static matrix

| Evaluation | Acceptance |
|---|---|
| density | all dimensions conform to tokens; exceptions documented |
| alignment | label/value/unit columns align within 1 px at supported scales |
| overflow | no clipped command, value, unit, or state at minimum size |
| prose suppression | no prohibited explanatory prose in standard panels |
| visual hierarchy | selection, focus, warning, error distinguishable without decorative cards |
| states | all applicable states represented and tested |
| contrast | automated AA check plus data-palette review |

## Interaction matrix

| Evaluation | Acceptance |
|---|---|
| keyboard | complete workflow without pointer where non-spatial |
| pointer | target and drag-alternative requirements met |
| cancellation | Esc/cancel leaves authoritative state unchanged |
| undo/redo | accepted mutation restores exact prior/next project state |
| selection sync | tree, viewport, tables, and inspector agree |
| focus restoration | invoking surface regains focus after close/cancel |
| invalid input | value preserved, rule shown, operation blocked safely |
| long operation | job visible; UI remains inspectable; safe cancel behaves deterministically |

## Workflow benchmarks

| Workflow | Maximum intentional actions* | Required evidence |
|---|---:|---|
| import one scan | 3 | recording + action trace |
| isolate an object | 5 | recording + selection-state trace |
| fit plane | 5 | metrics visible without navigation |
| fit cylinder | 6 | metrics visible without navigation |
| register two scans | 8 | residual review included |
| create distance measurement | 4 | numeric result and components |
| return from derived entity to source evidence | 1 | provenance link |
| export selected entity | 4 | format/options confirmation |

`*` File-system browsing and unavoidable OS security prompts are excluded.

## Viewport performance

| Scale | Target |
|---|---:|
| camera interaction input-to-frame p95 | ≤50 ms |
| point/face pick p95 | ≤100 ms |
| tree selection highlight p95 | ≤50 ms |
| inspector update p95 | ≤100 ms |
| command palette open p95 | ≤100 ms |
| tree search first result p95 | ≤150 ms |

Performance tiers SHALL be evaluated at 1M, 100M, and 1B source points using representative hardware. Rendering LOD may change; analysis data SHALL not.

## Screenshot baselines

Required for every workspace at:

- empty project;
- small project;
- mixed scan/mesh project;
- billion-point metadata scenario;
- minimum supported window;
- 100%, 125%, 150%, and 200% scale;
- each dock configuration;
- all validation outcomes;
- every dialog and context menu.

## Release gate

Release fails if:

- any P0/P1 workflow lacks end-to-end evidence;
- any control lacks keyboard/focus evaluation;
- unexplained text or padding violates contract;
- selection synchronization test fails;
- a destructive action lacks undo/confirmation;
- screenshot or interaction regression is unreviewed.
