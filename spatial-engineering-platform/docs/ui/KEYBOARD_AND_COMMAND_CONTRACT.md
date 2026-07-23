# Keyboard and Command Contract

**Contract ID:** SEP-UI-008

## Global commands

| Command | Default |
|---|---|
| command palette | Ctrl+K |
| open project/import | Ctrl+O |
| save | Ctrl+S |
| undo/redo | Ctrl+Z / Ctrl+Y |
| find in active navigator | Ctrl+F |
| focus viewport | Ctrl+1 |
| focus project tree | Ctrl+2 |
| focus inspector | Ctrl+3 |
| focus jobs/results | Ctrl+4 |
| frame selection | F |
| frame all | Home |
| cancel active tool | Esc |
| accept active tool | Enter |
| delete selection | Delete |
| rename | F2 |
| hide selection | H |
| isolate selection | Shift+H |
| reveal all | Alt+H |

Shortcuts are user-remappable and exportable. Conflicts are detected before assignment.

## Focus

- Focus is always visible.
- Tab order follows visual and operational order.
- Opening a tool moves focus to its first required input without stealing focus during active viewport manipulation.
- Closing a panel returns focus to the invoking control.
- Escape cancels the narrowest active interaction first: popup, edit, tool preview, then selection operation.

## Command IDs

Every command has one stable machine ID, label, description, shortcut, enabled predicate, visibility predicate, execution handler, undo policy, telemetry classification, and test identifier. UI surfaces reference this registry rather than implementing duplicate actions.
