# Density and Design Tokens

**Contract ID:** SEP-UI-005

## 1. Density

Compact is the default and normative desktop mode.

| Token | Value |
|---|---:|
| base spacing | 2 px |
| standard control height | 26 px |
| dense control height | 22 px |
| tree row | 24 px |
| table row | 24 px |
| toolbar | 30 px |
| tab | 28 px |
| panel header | 28 px |
| status bar | 22 px |
| panel padding | 6 px |
| panel gap | 3 px |
| inline label/value gap | 4 px |
| standard icon | 16 px |
| primary toolbar icon | 18–20 px |
| border radius | 2 px; max 3 px |

Controls below 24×24 px SHALL satisfy the WCAG spacing/equivalent-control exception and SHALL remain keyboard accessible. Critical or isolated pointer targets SHALL be at least 24×24 px.

## 2. Property grid

Normative row:

```text
Label          Value          Unit        State
Diameter       125.00         mm          ✓
Residual       0.34           mm          Review
Confidence     96.20          %
```

- Labels align right or left consistently within a panel.
- Numeric values use tabular numerals and right alignment.
- Units occupy a dedicated compact column or selector.
- Validation is not conveyed by color alone.
- Vertical stacked labels are permitted only below the minimum panel width.

## 3. Typography

| Role | Size |
|---|---:|
| default UI | 12–13 px |
| compact table/tree | 12 px |
| panel title | 12–13 px semibold |
| workspace title | 14–16 px semibold |
| numeric status | 12 px tabular |

No routine heading SHALL exceed 18 px.

## 4. Neutral palette

Static chrome uses neutral luminance steps. Exact colors are implementation tokens, but hue bias SHALL remain near-neutral. Blue is reserved for data or selected-state accents, not background tint.

Required semantic roles:

- surface-0 through surface-4;
- border-subtle and border-strong;
- text-primary, secondary, disabled;
- selection;
- focus;
- success;
- warning;
- error;
- indeterminate;
- data palettes independent from chrome.

## 5. Responsive lower bound

Desktop minimum is 1100×700 logical px. Below 1280 px width, secondary inspectors may tab or collapse; the viewport and project tree SHALL remain available. No desktop hamburger navigation.
