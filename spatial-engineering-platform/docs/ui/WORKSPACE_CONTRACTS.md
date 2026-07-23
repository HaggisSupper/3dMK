# Workspace and Workflow Contracts

**Contract ID:** SEP-UI-003

## Shared workflow law

Every processing workflow SHALL declare:

- admissible input entity types;
- required selection cardinality;
- parameters and units;
- preview representation;
- quality metrics;
- accept/recalculate/cancel behavior;
- produced entities;
- provenance record;
- failure and recovery states.

## VIEW

Purpose: inspect and navigate without mutating geometry.

Visible controls: view orientation, projection, render mode, point size, scalar field, clipping, isolate/hide, saved views, measurements. Viewer changes SHALL be reversible and SHALL NOT alter source data.

## CLEAN

Canonical sequence:

`select source → choose operation → preview affected points/triangles → inspect retained/removed counts → accept`

Operations include crop, lasso, box, plane clip, statistical outlier removal, radius filter, connected components, dynamic-object removal, decimation, and normal recomputation.

Source geometry remains immutable by default. Accepted output is a derived entity linked to its source.

## REGISTER

```text
Source        Scan 002
Target        Scan 001
Initial       Features | Picked pairs | Control points | Existing transform
Refinement    Point-to-plane ICP | GICP | Colored ICP
Max distance  value + unit
Iterations    integer
```

Required review metrics:

- RMS residual;
- median and maximum residual;
- inlier ratio;
- overlap;
- transform uncertainty/covariance when available;
- residual spatial distribution.

The viewport SHALL support before/after toggle, split view, color-by-distance, and transform gizmo. Acceptance writes an explicit transform edge; it does not bake coordinates into the source.

## SEGMENT

Canonical interactions:

- 2D polygon/rectangle selection projected through the current view;
- 3D box/sphere/lasso;
- seed-and-grow;
- connected components;
- primitive-assisted segmentation;
- model-proposed instance mask with positive/negative correction clicks.

The user SHALL be able to keep inside, keep outside, add, subtract, reset, preview, and accept. Camera manipulation SHALL remain available in a paused selection mode. Accepted segments retain source-point membership and view/selection provenance.

## RECONSTRUCT

Primitive fitting contract:

```text
Primitive      Plane | Line | Circle | Cylinder | Cone | Sphere | Torus | Sweep
Method         Deterministic fitting method
Constraints    Free | axis | datum | dimension | symmetry
Tolerance      value + unit
Outlier policy method + threshold
```

Required review metrics:

- support count;
- RMS and maximum deviation;
- coverage;
- confidence/stability;
- competing fit warning.

Fit preview SHALL remain editable. Accept creates a parametric engineering entity linked to source geometry. VLM output MAY propose an entity type but SHALL NOT accept it.

## BIM

Canonical sequence:

`select geometry/segment → choose BIM class → fit placement/shape → assign storey/system → validate relationships → accept`

Inspector SHALL separate:

- geometry;
- placement;
- classification;
- properties;
- containment;
- connectivity;
- validation.

IFC schema errors, missing property requirements, and geometric deviation SHALL be visible as structured issues, not prose embedded in forms.

## INSPECT

Canonical sequence:

`choose reference → choose measured entity → establish datum/alignment → select control → preview result → review uncertainty → accept report item`

Required result fields:

- nominal;
- measured;
- deviation;
- tolerance;
- uncertainty;
- decision: pass/fail/indeterminate;
- data source and calibration state.

Color maps SHALL have visible scale, units, clipping limits, and uncertainty indication. A pass/fail result SHALL never be shown without uncertainty state where the workflow claims metrological authority.

## Background jobs

Long operations SHALL enter the jobs region with progress, stage, elapsed time, cancel/pause capability when technically safe, resource backend, and diagnostic expansion. The user SHALL remain able to inspect the project during background processing.
