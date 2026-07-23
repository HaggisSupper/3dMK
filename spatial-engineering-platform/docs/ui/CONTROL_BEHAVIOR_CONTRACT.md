# Control Behavior Contract

**Contract ID:** SEP-UI-006

## Numeric inputs

- Accept direct typing, increment/decrement, and expression entry where safe.
- Display explicit unit; convert without changing the underlying physical quantity.
- Validate on input and commit; invalid text remains editable and is not silently coerced.
- Mouse wheel SHALL not alter values unless the control is focused.
- Precision displayed is configurable; stored precision is not truncated.

## Selectors

- Enumerated choices use compact dropdowns or segmented controls when choices ≤4.
- Searchable selectors are required for large entity or classification lists.
- Placeholder text SHALL describe required input, not provide tutorials.

## Buttons

- Primary action appears once per transaction.
- Accept, Recalculate, Cancel ordering remains consistent.
- Icon-only buttons require accessible names and concise tooltips.
- Disabled buttons expose the unmet prerequisite on focus/hover.

## Toggles

Binary persistent states use checkbox/toggle; immediate one-shot actions use buttons. Visibility, selectability, and lock states use consistent icons across tree and inspector.

## Tables

- Sort, filter, resize, and copy are keyboard operable.
- Columns have explicit units in headers where uniform.
- Large tables virtualize rows.
- Selection synchronizes with the authoritative model.
- Errors and invalid values remain visible after sorting/filtering.

## Dialogs

Dialogs are reserved for destructive confirmation, file operations, credentials, global preferences, and compact transactions that cannot coexist with the viewport. Routine modeling and processing SHALL use docked/context panels.

## Context menus

Context commands are filtered by selection type and validation state. The same command IDs are reused in menu, palette, shortcut, and context surfaces.

## Undo/redo

All accepted project mutations SHALL be undoable unless technically impossible. Non-undoable operations require explicit warning before execution. View navigation is not placed in the project undo stack; visibility and saved-view changes are.

## Drag alternatives

Every drag-only operation SHALL provide single-pointer or numeric alternatives unless spatial dragging is essential. Gizmos expose numeric transforms in the inspector.
