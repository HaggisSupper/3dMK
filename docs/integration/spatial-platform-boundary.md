# Spatial Engineering Platform Boundary

## Placement

The platform is owned by:

```text
spatial-engineering-platform/
```

It is a bounded subsystem of 3dMK, not a replacement for the existing root backend during the initial integration phase.

## Root application compatibility

The current root commands remain intact until explicit forwarding adapters are introduced. New platform code must not import private root modules directly.

## Allowed integration points

- Existing VWM crates are accessed only through `vwm-adapter`.
- Existing Truck functionality is accessed only through `cad-truck-adapter`.
- Future OpenCascade functionality is isolated behind `cad-occ-adapter`.
- Root CLI forwarding consumes public platform contracts.
- Project artifacts cross boundaries through versioned schemas and stable IDs.

## Prohibited coupling

- No platform crate may depend on root `src/` internals.
- No VWM implementation type may cross into the UI layer.
- No CAD-kernel-native type may cross its adapter boundary.
- No authoritative geometry may be owned by JavaScript.
- No raw docking-library state may become the persistence format.
- No AI output may mutate verified engineering state without deterministic validation.

## Extraction transition

The first publishing PR carries the complete platform source as an encoded archive because the connected publishing environment cannot execute an authenticated native Git push. The payload must be materialized with the included PowerShell script and committed as normal source files before substantive implementation work continues.

## Definition of complete repository integration

- Existing root commands compile and behave as before.
- The platform compiles as a workspace member.
- Platform contracts remain independently testable.
- UI and persistence guardrails run in CI.
- Format support is reported truthfully through capabilities.
- The platform could be extracted into a standalone repository without rewriting its public interfaces.
