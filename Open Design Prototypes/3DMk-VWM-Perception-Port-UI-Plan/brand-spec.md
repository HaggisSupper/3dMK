# 3DMk workflow concept — brand binding

System: a dark, compact inspection workspace with graphite surfaces, restrained teal interaction cues, and high-legibility operational text.

Observed from the existing 3DMk `public/index.html`, converted to OKLch:

```css
:root {
  --bg: oklch(0.1822 0 89.9);          /* observed #121212 */
  --surface: oklch(0.2350 0 89.9);     /* observed #1e1e1e */
  --fg: oklch(0.9668 0.0044 179.7);    /* observed #f1f5f4 */
  --muted: oklch(0.7182 0.0139 184.9); /* observed #9ba7a5 */
  --border: oklch(0.3630 0.0173 185.1);/* observed #34413f */
  --accent: oklch(0.7038 0.1230 182.5);/* observed #14b8a6 */
}
```

Typography: `Segoe UI Variable, Segoe UI, Tahoma, sans-serif` for display and UI; `Cascadia Mono, Consolas, monospace` for provenance and calibration values.

Posture rules:

- Keep the canvas dominant; tools are narrow, fixed workbench edges.
- Use teal for one active state or primary action at a time, never as decoration.
- Stack workspace controls as nested accordions; reveal downstream work only after its prerequisite.
- Use thin graphite dividers, compact rows, and square-leaning corners rather than a dashboard-card grid.
- Keep settings and help in one right-side dock with mutually exclusive tabs.
