# Accessibility Contract

**Contract ID:** SEP-UI-009

The desktop workstation targets WCAG 2.2 AA principles where applicable and Windows accessibility conventions.

## Requirements

- Every interactive element has an accessible name, role, state, and value.
- All workflows are keyboard operable except spatial operations whose essence is positional; those provide numeric or pick-list alternatives.
- Focus indicators meet contrast requirements and are not clipped.
- Status is never communicated by color alone.
- Text and essential non-text contrast are validated automatically.
- UI supports 100%, 125%, 150%, and 200% scale without hidden commands or overlapping content.
- Reduced-motion preference disables nonessential animation.
- Screen-reader announcements are concise for job completion, validation changes, and selection changes; high-frequency pointer movement is not announced.
- Dense target exceptions require spacing or an equivalent ≥24 px target.
- Viewport tools expose a non-visual entity list and numeric parameters.

## Data visualization

Palettes SHALL be perceptually ordered where magnitude is encoded, offer color-vision-deficiency-safe alternatives, and pair threshold states with symbols/labels. Legends SHALL remain keyboard focusable and copyable.
