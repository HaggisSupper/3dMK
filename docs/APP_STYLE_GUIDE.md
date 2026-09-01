# Universal Application Style Guide

Version 1.0
Status: Reusable baseline for product, internal, mobile, desktop, and web applications

This guide defines a practical visual and interaction system that can be adapted to almost any application. It is intentionally product-neutral: replace the example names and token values with the product's brand, platform, and domain language while keeping the underlying rules.

The goal is not to make every application look identical. The goal is to make every screen feel intentional, understandable, consistent, accessible, and easy to maintain.

## 1. How to use this guide

Use this document as the default decision framework when designing a new screen, adding a component, or reviewing an existing interface.

1. Start with the user's task, not the visual treatment.
2. Reuse an existing pattern before introducing a new one.
3. Prefer the smallest visual system that clearly communicates hierarchy.
4. Design all important states, not only the successful first view.
5. Validate keyboard, touch, assistive-technology, narrow-screen, and slow-network behavior.
6. Record intentional exceptions in the product's design documentation.

The guide describes rules in priority order. A product may change a color, typeface, or radius, but it should not casually change hierarchy, focus behavior, error handling, or accessibility requirements.

## 2. Core principles

### 2.1 Clarity before decoration

Every visible element should answer at least one useful question:

- What can I do here?
- What is happening now?
- What has changed?
- What should I do next?
- What will happen if I continue?

Remove ornamental borders, labels, icons, and controls that do not help the user complete or understand a task.

### 2.2 Hierarchy is a promise

Visual emphasis must match importance. The most important action should be easiest to find, the current location should be obvious, and secondary options should not compete with the primary task.

Use size, position, contrast, spacing, grouping, and progressive disclosure to establish hierarchy. Do not rely on color alone.

### 2.3 Consistency reduces cognitive load

The same action should look, read, and behave the same way everywhere. A button that saves in one area should not discard in another. A close icon should not sometimes mean cancel and sometimes mean hide.

### 2.4 Give the interface a visible state

Users should be able to tell whether an operation is ready, running, successful, blocked, failed, or awaiting input. A state must be communicated through text, structure, or an accessible status—not only through a color change or animation.

### 2.5 Make the safe path easy

Put common actions near the content they affect. Keep destructive actions visually distinct and require a meaningful confirmation when recovery is difficult. Preserve user input when validation fails.

### 2.6 Accessibility is part of quality

Accessibility is not an optional theme or a later compliance pass. It is a core quality requirement that improves usability for everyone, including people using small screens, bright light, a keyboard, magnification, voice control, or a slow connection.

### 2.7 Honest feedback builds trust

Never imply that work succeeded when it is still running, partially complete, or waiting for a dependency. Messages should state what happened, why it happened when useful, and what the user can do next.

## 3. Design foundations

### 3.1 Design tokens

Store shared values as named tokens instead of repeating raw values in component code. Tokens make themes, platform adaptations, and global corrections safe.

Recommended token groups:

| Group | Examples |
| --- | --- |
| Color | `color.bg.canvas`, `color.text.primary`, `color.action.primary` |
| Typography | `type.body.md`, `type.heading.lg`, `type.weight.semibold` |
| Spacing | `space.1` through `space.8` |
| Size | `size.control.min`, `size.icon.md`, `size.content.max` |
| Radius | `radius.sm`, `radius.md`, `radius.pill` |
| Elevation | `shadow.panel`, `shadow.modal`, `shadow.focus` |
| Motion | `motion.fast`, `motion.standard`, `motion.ease.standard` |
| Layering | `layer.header`, `layer.popover`, `layer.modal` |
| Breakpoints | `breakpoint.compact`, `breakpoint.wide` |

Token names should describe purpose rather than appearance. Prefer `color.surface.raised` over `color.light-gray-2`, and `space.3` over `margin-12`.

### 3.2 Spacing system

Use a predictable base unit, normally 4 px or 8 px. Allow exceptions only when they improve legibility or control hit areas.

Suggested scale:

| Token | Value | Typical use |
| --- | ---: | --- |
| `space.1` | 4 px | Icon-to-label gap, compact metadata |
| `space.2` | 8 px | Related controls, small card padding |
| `space.3` | 12 px | Form rows, inline groups |
| `space.4` | 16 px | Standard component padding |
| `space.5` | 24 px | Section separation |
| `space.6` | 32 px | Major group separation |
| `space.7` | 48 px | Page section separation |
| `space.8` | 64 px | Large landing or empty-state spacing |

Use less space between items that belong together and more space between items that represent different concepts. Spacing should explain grouping even when borders are removed.

### 3.3 Layout and containers

- Establish one primary reading or work flow per screen.
- Use a maximum content width for text-heavy pages so lines remain readable.
- Align headings, controls, and content to a shared grid.
- Keep persistent navigation visually separate from task content.
- Avoid edge-to-edge text on wide screens and cramped multi-column layouts on narrow screens.
- Reserve space for status, validation, and asynchronous feedback instead of causing avoidable layout jumps.

Use a 12-column grid for complex desktop layouts or a simpler single-column flow for focused tasks. A grid is a tool for alignment, not a requirement to fill every column.

### 3.4 Density

Offer a deliberate density choice only when the product has a real need for it. Compact layouts are useful for professional data-heavy work; comfortable layouts are usually better for forms, onboarding, and general audiences.

Do not achieve density by shrinking text below comfortable reading sizes or by reducing touch targets. Remove unnecessary decoration and combine related metadata first.

## 4. Color system

### 4.1 Semantic color roles

Define colors by meaning and role:

- **Canvas:** the main page or work area.
- **Surface:** cards, panels, fields, and elevated regions.
- **Text:** primary, secondary, muted, inverse, and disabled.
- **Border:** default, strong, focus, and error.
- **Action:** primary, secondary, selected, and destructive.
- **Status:** success, information, warning, and error.

Components should consume semantic roles, not brand hex values. A dark theme or high-contrast theme can then remap roles without redesigning each component.

### 4.2 Contrast and meaning

Meet WCAG 2.2 AA contrast targets as a baseline: 4.5:1 for normal text, 3:1 for large text, and 3:1 for meaningful graphical objects and user-interface boundaries. Verify actual rendered colors, not only token values.

Never communicate meaning with color alone. Pair status colors with text, icons, patterns, position, or an accessible label. Check color combinations for common forms of color-vision deficiency.

### 4.3 Theme behavior

- Respect the operating system or browser preference when a product supports automatic theming.
- Keep semantic roles stable between themes.
- Do not use pure white or pure black everywhere; a near-neutral surface usually reduces glare and preserves hierarchy.
- Preserve visible focus indicators in every theme.
- Test images, charts, disabled controls, overlays, and third-party content in each theme.

## 5. Typography

### 5.1 Type scale

Use a small, intentional scale. A typical starting point is:

| Role | Size | Line height | Use |
| --- | ---: | ---: | --- |
| Display | 32–48 px | 1.1–1.2 | Rare page-level introduction |
| Heading 1 | 28–36 px | 1.2 | Main page title |
| Heading 2 | 22–28 px | 1.25 | Major section |
| Heading 3 | 18–22 px | 1.3 | Subsection or panel title |
| Body | 16 px | 1.45–1.6 | Default reading text |
| Small | 14 px | 1.4–1.5 | Supporting text and metadata |
| Caption | 12–13 px | 1.35–1.45 | Supplementary labels only |

Do not use caption text for essential instructions, errors, or values. Let users enlarge text to at least 200% without loss of content or functionality.

### 5.2 Type choices

- Use one primary type family unless a second family has a clear purpose.
- Choose fonts with distinguishable characters and broad language support.
- Use weight and spacing sparingly; too many weights make hierarchy noisy.
- Use tabular numerals for values that need vertical comparison.
- Keep line lengths near 45–90 characters for sustained reading.
- Avoid all-caps sentences. Use capitalization for short labels and reserve letter spacing for small supporting text.

### 5.3 Content hierarchy

Every page should have one clear primary heading. Heading levels must reflect structure, not visual size. Do not skip levels solely to achieve a visual style; style and semantic level should be independently controllable.

## 6. Iconography and imagery

- Use one coherent icon family with consistent stroke, corner, and optical weight.
- Use icons to support a label or represent a familiar action; do not make unfamiliar icons carry essential meaning alone.
- Give icon-only controls an accessible name and a visible tooltip on pointer hover when helpful.
- Keep icon hit areas larger than the visible glyph.
- Do not mix filled and outlined icon styles without a clear hierarchy.
- Use meaningful alternative text for informative images and empty alternative text for decorative images.
- Crop or scale images consistently within a component family.
- Avoid stock imagery that implies unsupported features, people, or outcomes.

## 7. Component standards

### 7.1 Buttons

Use a button for an action and a link for navigation.

Button hierarchy:

1. **Primary:** one dominant action for the current context.
2. **Secondary:** important alternatives that do not compete with the primary action.
3. **Tertiary or quiet:** low-emphasis actions such as reset, dismiss, or learn more.
4. **Destructive:** irreversible or high-impact actions, styled distinctly and confirmed when appropriate.

Rules:

- Use concise, verb-led labels: “Save changes”, “Add item”, “Try again”.
- Keep the label stable while the action runs; show progress beside or within it without causing confusion.
- Provide disabled styling only when the reason is clear nearby. Prefer preventing invalid submission with a useful explanation.
- Maintain a minimum target of 44 × 44 CSS px for touch-oriented controls.
- Keep primary actions in a predictable location and order.

### 7.2 Links

Links should be visibly distinguishable from surrounding text without relying only on hover. Link labels should describe the destination or result. Avoid vague labels such as “click here”. External, downloadable, and destructive destinations should be identified when the context requires it.

### 7.3 Forms

- Give every field a persistent label; do not use placeholder text as the label.
- Place instructions before the field group or directly beside the field they explain.
- Use the input type and autocomplete behavior that match the data.
- Group related fields under a named section or fieldset.
- Keep labels, controls, and error messages aligned.
- Validate at a helpful time: immediate validation for clear local rules, on blur for completed fields, and on submit for cross-field rules.
- Put error text next to the affected field and summarize errors at the top of long forms.
- Preserve entered values and focus when validation fails.
- Explain required, optional, format, length, unit, and privacy expectations.

### 7.4 Selects, menus, and comboboxes

Use a native select when it meets the need. Use a custom menu only when it provides a real capability such as search, grouping, or multi-selection. Support keyboard navigation, escape-to-close, type-ahead where appropriate, clear selection, and an explicit empty state.

Do not hide critical options behind an unlabeled three-dot menu. Keep destructive menu actions separated from routine actions.

### 7.5 Checkboxes, radios, and switches

- Use a checkbox for independent choices.
- Use radio buttons for one choice from a small visible set.
- Use a switch for an immediate on/off setting whose effect is clear.
- For settings that require a separate save action, a checkbox or select is often clearer than a switch.
- Make the whole label area clickable while retaining a distinct focus target.

### 7.6 Tabs

Use tabs to switch between related views at the same hierarchy level, not as a replacement for page navigation. Keep labels short, preserve the selected tab on reload when practical, and do not hide unsaved changes without warning.

### 7.7 Cards and panels

Use a card when content or actions form a meaningful unit. Do not wrap every paragraph in a card. A panel should have a clear title or purpose, consistent internal padding, and a predictable action area.

### 7.8 Tables and data grids

- Use real table semantics for tabular relationships.
- Keep column headings visible and aligned.
- Right-align numeric values and align decimals when comparison matters.
- Provide sorting state, filtering state, and a clear way to reset filters.
- Explain empty, loading, partial, and error states.
- Avoid forcing users to interpret color-only status cells.
- On narrow screens, allow horizontal scrolling or transform rows into labeled records; never silently clip critical values.

### 7.9 Dialogs and drawers

Use a dialog for a focused decision or short task that must be completed before returning. Use a drawer for supplementary context that can remain alongside the current task.

Dialogs and drawers must:

- Have a clear title and accessible name.
- Keep focus inside while open and restore focus when closed.
- Close with an explicit control; support Escape unless the operation is actively running or data loss would result.
- Avoid stacking more than one modal layer.
- Preserve the underlying context and explain unsaved changes before dismissal.

### 7.10 Tooltips

Use tooltips for short, supplemental explanations—not for essential instructions or interactive controls. They should be discoverable by pointer, keyboard, and touch alternatives, and should not cover the target being explained.

## 8. States and feedback

Every meaningful component should define these states where applicable:

- Default
- Hover
- Focus-visible
- Pressed or selected
- Disabled
- Loading
- Success
- Warning
- Error
- Empty
- Partial or unavailable

### 8.1 Loading

Show loading feedback when an action takes long enough to interrupt user confidence. Use a progress bar for measurable progress, a spinner for indeterminate short waits, and staged status text for multi-step work. Tell users what is happening and whether they can continue elsewhere.

### 8.2 Errors

An error message should be specific, calm, and actionable:

1. State what failed.
2. Explain the likely cause when it helps.
3. Tell the user what to do next.
4. Provide retry, recovery, or support information when available.

Avoid blaming the user, exposing stack traces, or saying only “Something went wrong.” Log technical detail separately from the user-facing message.

### 8.3 Notifications

Use inline feedback for field and task-specific messages. Use a toast for brief, non-blocking confirmation. Use a banner for information that remains relevant across a page or session. Do not make critical errors disappear before they can be understood.

### 8.4 Confirmation and undo

Prefer undo for reversible actions. Use confirmation for destructive, costly, externally visible, or difficult-to-recover actions. State the exact object and consequence in the confirmation action label.

## 9. Navigation and information architecture

- Organize navigation around user goals and tasks, not internal team ownership.
- Keep the current location visible through selection, breadcrumbs, headings, or a combination.
- Use progressive disclosure to keep first-level choices manageable.
- Preserve navigation state when moving between related views.
- Provide a clear route to home, help, settings, and recovery when they are part of the product.
- Avoid duplicate paths that produce different results unless the distinction is explicit.
- Keep URLs, titles, and screen headings aligned in web applications.

## 10. Responsive and platform behavior

Design from content constraints rather than device names alone. Define compact, standard, and wide layouts based on where the interface needs more room.

- Reflow before text becomes cramped.
- Keep primary actions visible and reachable.
- Replace side-by-side panels with a clear sequence on small screens.
- Avoid hover-only functionality on touch devices.
- Respect safe areas, virtual keyboards, window resizing, and orientation changes.
- Support pointer, keyboard, touch, and assistive input without making one mode the only usable path.
- Do not use a mobile layout that merely shrinks desktop controls.

## 11. Accessibility checklist

The following is the minimum release baseline:

- All functionality works with a keyboard.
- Focus order follows the visual and task order.
- Focus is always visible and not hidden behind sticky content.
- Every control has an accessible name, role, and state.
- Headings and landmarks provide a logical document structure.
- Form errors are associated with their fields and announced when appropriate.
- Status updates use an appropriate live-region strategy without excessive interruption.
- Text and controls meet contrast requirements.
- Content remains usable at 200% zoom and reflowed narrow widths.
- Motion can be reduced or disabled through the operating-system preference.
- Touch targets are large enough and separated enough to avoid accidental activation.
- Captions, transcripts, alternatives, and labels exist for non-text content where needed.
- Tables, dialogs, menus, tabs, and custom widgets follow their expected interaction patterns.

## 12. Motion and animation

Motion should explain change, preserve context, and provide feedback. It should never delay a task or distract from a critical message.

Recommended defaults:

- Fast: about 100–150 ms for direct control feedback.
- Standard: about 200–300 ms for panels, menus, and state changes.
- Slow: about 300–500 ms only for large spatial transitions or onboarding.

Rules:

- Use one easing family across the product.
- Animate opacity and transform more often than layout dimensions.
- Avoid looping animation unless it communicates ongoing work.
- Honor `prefers-reduced-motion` by removing nonessential movement and shortening required transitions.
- Do not animate text in a way that harms reading or causes content to move unexpectedly.

## 13. Content and voice

Write for the user's level of knowledge, not the team's vocabulary.

- Prefer plain, direct words.
- Use sentence case for headings, buttons, and labels.
- Use active voice and concrete verbs.
- Put the important information first.
- Explain abbreviations the first time they appear.
- Keep terminology consistent; create a product glossary for unavoidable specialist terms.
- Use examples that reflect real user tasks.
- Avoid jokes, blame, sarcasm, and ambiguous technical error codes in user-facing copy.
- Describe dates, units, time zones, permissions, and data retention clearly.

### 13.1 Labels and actions

Labels should be short but specific. “Export report” is better than “Submit” when the outcome is an export. Action labels should match the result users will see after activation.

### 13.2 Empty states

An empty state should explain why the area is empty and give the most useful next action. Distinguish “nothing created yet,” “no results for these filters,” “not available,” and “failed to load.”

## 14. Data visualization

- Choose a chart type based on the question: comparison, trend, distribution, relationship, or composition.
- Label axes, units, date ranges, and aggregation methods.
- Start axes at zero when truncation could mislead comparison; explain intentional exceptions.
- Use color palettes that remain distinguishable without color alone.
- Provide a table or text summary for important data.
- Show loading, empty, partial, error, and stale-data states.
- Do not imply precision beyond the source data.
- Keep legends close to the marks they describe and make them interactive only when the interaction is discoverable.

## 15. Privacy, security, and trust cues

The interface should make important data and permission behavior understandable without exposing sensitive information.

- Explain why a permission is needed before requesting it.
- Show what data will be stored, shared, exported, or deleted.
- Mask secrets and personal data by default.
- Make security-relevant actions visually distinct and confirm consequential changes.
- Use trustworthy status language; never claim encryption, completion, or verification that the system has not actually performed.
- Provide recovery and support paths for account, access, and data-loss issues.

## 16. Implementation guidance

### 16.1 Component contracts

Each shared component should document:

- Purpose and appropriate use.
- Required and optional properties.
- Supported states.
- Keyboard and touch behavior.
- Accessible name and semantics.
- Content limits and truncation behavior.
- Responsive behavior.
- Examples and known anti-patterns.

### 16.2 Styling rules

- Consume tokens rather than raw values.
- Keep component styles local and predictable.
- Avoid selectors that depend on page position or DOM accidents.
- Use semantic HTML before adding ARIA.
- Keep interaction logic separate from presentation where the platform allows it.
- Avoid disabling browser defaults unless the replacement preserves equivalent behavior.
- Treat focus, reduced motion, and high-contrast behavior as first-class styles.

### 16.3 Performance

- Load only the assets needed for the current view.
- Reserve dimensions for images and asynchronous content to reduce layout shift.
- Prefer CSS transitions and platform primitives over expensive continuous JavaScript animation.
- Keep interaction feedback immediate even when the underlying operation is asynchronous.
- Test on representative low-power hardware and slower connections, not only on a development workstation.

## 17. Review and quality gates

Before releasing a screen or component, review it against this list:

### Visual

- Is the primary action obvious?
- Is hierarchy clear without relying on color?
- Are spacing, alignment, type, and icon styles consistent?
- Are dense areas grouped and scannable?
- Does the screen look intentional in light and dark themes?

### Interaction

- Are all actions labelled with their real outcome?
- Are loading, success, error, empty, and unavailable states designed?
- Can users recover from mistakes?
- Does focus move logically and return appropriately?
- Does the layout remain usable at narrow widths and high zoom?

### Accessibility

- Can the complete task be finished with keyboard and assistive technology?
- Are labels, roles, values, and announcements correct?
- Are contrast, target size, and reduced motion requirements met?

### Content

- Is the language understandable to the intended audience?
- Are terms, units, dates, and permissions unambiguous?
- Are technical details separated from user-facing guidance?

### Engineering

- Are tokens and existing components reused?
- Are semantic elements used where available?
- Are errors handled without data loss?
- Are automated and manual tests recorded for the changed states?

## 18. Governance and maintenance

Treat this guide as a living product contract.

- Add a new pattern only when existing patterns cannot solve the task.
- Record the reason for a deliberate exception and its affected surfaces.
- Deprecate old patterns with migration guidance rather than silently supporting both forever.
- Review shared tokens and components together so visual changes do not drift across screens.
- Include screenshots, interaction notes, accessibility evidence, and known limitations in significant design reviews.
- Recheck the guide when the product adds a new platform, theme, input mode, or data type.

## 19. Quick reference

When in doubt:

1. Make the user's next action obvious.
2. Use the existing component and token that already expresses the intent.
3. Use hierarchy and spacing before adding decoration.
4. Design every state, including failure and recovery.
5. Keep labels plain, specific, and honest.
6. Make the task work with keyboard, touch, zoom, and reduced motion.
7. Test the real content and the smallest supported screen.
8. Document exceptions so the system can remain coherent.
