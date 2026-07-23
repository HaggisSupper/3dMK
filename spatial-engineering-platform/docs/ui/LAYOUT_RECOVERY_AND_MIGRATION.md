# Layout Recovery and Migration Guardrails

This document defines the implementation sequence for `SEP-UI-DOCK-PERSISTENCE`.

## State machine

```text
UNINITIALIZED
  → REGISTERING_PANELS
  → DISCOVERING_MONITORS
  → LOADING_CURRENT
      ├─ valid → MIGRATING → VALIDATING → APPLYING
      └─ invalid → LOADING_LAST_KNOWN_GOOD
                      ├─ valid → MIGRATING → VALIDATING → APPLYING
                      └─ invalid → APPLYING_DEFAULT
  → RECALCULATING_VIEWPORT
  → PUBLISHING_RESTORED
  → INTERACTIVE
```

No tool may receive pointer or keyboard input before `INTERACTIVE`.

## Required implementation boundaries

```text
DockLibraryAdapter
  Converts library-specific layout state to and from the canonical contract.

LayoutRepository
  Owns paths, atomic writes, recovery copies, quarantine, and read limits.

LayoutMigrator
  Applies ordered schema and panel-state migrations.

LayoutValidator
  Enforces schema, panel registry, docking, viewport, size, count, and numeric invariants.

MonitorReconciler
  Maps saved monitor/bounds data to current topology and DPI.

LayoutCoordinator
  Runs startup restore and debounced persistence; publishes lifecycle events.
```

No docking-library JSON may be persisted directly. Library-specific state is volatile and must be translated through the canonical schema.

## Persistence lifecycle events

- `layout-dirty`
- `layout-save-scheduled`
- `layout-saved`
- `layout-save-failed`
- `layout-restore-started`
- `layout-restored`
- `layout-recovered`
- `layout-defaulted`

Events carry codes and identifiers, not explanatory paragraphs.

## Save pseudocode

```text
on_layout_changed(change):
    canonical = adapter.capture()
    validation = validator.validate(canonical)
    if validation.failed:
        report diagnostics
        do not replace recovery files
        return
    coordinator.mark_dirty()
    coordinator.debounce(500 ms, save)

save():
    payload = canonical_json(adapter.capture())
    repository.write_temp_and_flush(payload)
    validator.validate(repository.read_temp())
    repository.rotate_current_to_last_known_good()
    repository.atomic_replace_temp_as_current()
    coordinator.mark_clean()
```

## Restore pseudocode

```text
restore_before_interaction():
    for candidate in [current, last_known_good, shipped_default]:
        parsed = repository.load_bounded(candidate)
        migrated = migrator.migrate(parsed)
        checked = validator.validate(migrated)
        if checked.valid:
            reconciled = monitor_reconciler.apply(checked.layout)
            adapter.apply(reconciled)
            viewport.recalculate_bounds()
            publish layout-restored
            enable_interaction()
            return
    abort startup only if the shipped default embedded in the application is invalid
```

## Prohibited implementation shortcuts

- Persisting raw Golden Layout, FlexLayout, Lumino, or custom widget objects.
- Storing component constructors or executable names in JSON.
- Using screen coordinates without monitor and DPI metadata.
- Saving on every pointer-move event.
- Restoring floating windows before monitor discovery.
- Allowing a missing optional panel to fail the entire layout.
- Applying layout asynchronously after the user begins interacting.
- Mutating `last-known-good.json` before the new current payload passes validation.
- Treating the viewport as a normal dock item.

## Migration test fixtures

Each schema version must retain fixtures for:

- minimal valid layout;
- maximal valid layout;
- previous-version layout;
- removed-panel layout;
- unknown-panel layout;
- disconnected-monitor layout;
- corrupt current with valid recovery;
- future-version layout.
