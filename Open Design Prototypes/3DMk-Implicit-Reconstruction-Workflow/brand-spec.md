# 3DMk reconstruction workflow — visual binding

Graphite utility workspace with a restrained teal operational accent, compact Segoe-style typography, and dense pinned-panel layout.

```css
:root {
  --bg: oklch(0.175 0.012 240);
  --surface: oklch(0.215 0.013 235);
  --fg: oklch(0.92 0.012 210);
  --muted: oklch(0.68 0.015 220);
  --border: oklch(0.31 0.014 230);
  --accent: oklch(0.72 0.105 190);
  --font-display: "Segoe UI Variable Display", "Segoe UI", sans-serif;
  --font-body: "Segoe UI Variable Text", "Segoe UI", Tahoma, sans-serif;
  --font-mono: "Cascadia Mono", "Consolas", monospace;
}
```

- Panels are low-contrast graphite fields divided by thin borders, not floating cards.
- Teal denotes an active task, selected mode, or deliberate primary action only.
- Important quality failures use amber/red semantic states; they never masquerade as selection state.
- The central canvas remains dominant; rails are compact, pinned, and independently scrollable.
- Labels are sentence case, task-oriented, and information-dense.
