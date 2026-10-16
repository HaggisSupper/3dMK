# BHA placement along a wellbore: next bounded scope

Status: approved behavior to specify next; **not implemented or qualified** in this repair candidate. Keep the shared GLBAdapter exchange engine and 3DMK's Rust-owned scene document. Do not add another conversion engine.

## User action and authority

`Place BHA along wellbore` selects an actual trajectory, a bit measured depth (MD), and toolface/roll with an explicit reference convention. The resulting placement has its own stable ID and references immutable BHA assets, the selected well/trajectory identity and revision, and the component source IDs. Placement parameters, visibility and Undo must survive project save/reopen. Import alone must not silently register a BHA to a well.

## Required source contract

- BHA: component order, true physical length and units, local longitudinal axis, bit reference point, connection endpoints, component IDs and immutable asset identity. A declared representation distinguishes physical geometry from compact drafting geometry.
- Trajectory: strictly increasing MD stations, finite coordinates, declared units/axes/origin, well identity and actual-versus-planned role. MD and TVD are distinct quantities. Missing or inconsistent registration, unsupported units, ambiguous bit reference or missing lengths blocks placement.
- Frames: retain double precision through coordinate restoration and common local rebasing. Explicit per-station orientation and toolface conventions take precedence over guessed axes. The station interpolation and frame method must be named, deterministic and shared with the exchange/trajectory contract.

## Placement behavior

Component intervals run uphole from the selected bit MD using cumulative true lengths. The bit lies at the declared bit reference. A component end's MD is not its TVD or straight-line distance. Reject out-of-range intervals rather than extrapolating or clamping silently.

Expose the representation choice explicitly:

1. **Rigid components:** preserve each component's shape and physical dimensions, position it using its connection references and a documented station/chord orientation. In a curved hole, rigid parts need not coincide with the centerline at every point; display that limitation and any connection mismatch.
2. **Curved schematic:** map the assembly longitudinal coordinate along the trajectory with a stable transported frame and toolface roll. Mark the result as a visualization assumption, not a mechanics or deflection calculation. Keep the original physical asset unchanged.

Visible drafting breaks shorten only diagrams and datagrid icons. Their shortened geometry must never be used to calculate length, MD placement, borehole clearance or a physical model. Calculated deflected BHA centerlines remain a separate future capability.

## Acceptance evidence before enabling the action

- Straight, inclined and curved synthetic wells with known bit and joint MD positions.
- Metre/foot conversion, nonzero coordinate origins, station boundaries, singular/straight frame cases and declared roll conventions.
- True-length preservation for rigid components; explicitly reported deformation for curved schematic rendering.
- Distinct IDs for repeated assets and multiple wells; exact placement-parameter and hierarchy save/reopen equality.
- Rejected missing registration, descending/duplicate MDs, intervals outside survey range, missing physical dimensions and diagram-only assets.
- Actual rendered UI verification and Windows runtime qualification remain separate from numerical tests.

The BHA asset worker and GLBAdapter must agree on the exact metadata field names before implementation. This document does not invent a new exchange schema or imply that existing GLBs already carry every required field.
