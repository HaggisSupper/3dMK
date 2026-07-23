# Real-World Workflow Research and Design Convergence

**Contract ID:** SEP-UI-011  
**Research date:** 2026-07-23

## Sources examined

### CloudCompare

CloudCompare’s established layout combines menus/toolbars, a database entity tree, properties view, one or more 3D views, and a console. Its tool model places temporary controls in or near the viewport while maintaining object selection in the database tree. Interactive segmentation supports repeated polygon/rectangle operations, inside/outside decisions, paused navigation, explicit apply/cancel, reusable view state, and keyboard shortcuts. Point picking exposes source coordinates and attributes. These patterns support the persistent-tree, synchronized-selection, contextual-tool, and preview-transaction contracts.

### Autodesk ReCap

ReCap organizes work around scan/photo projects, import, registration, point-cloud generation, navigation/display, project orientation, saved views, and annotations. This validates workflow-level separation without abandoning the central spatial view, and the requirement that orientation and view states are persistent project artifacts.

### Agisoft Metashape

Metashape’s professional workflow is staged around image alignment, dense reconstruction, mesh/model generation, orthomosaic/elevation products, markers/control, and batch processing. This supports explicit processing stages, jobs/history, reproducible parameters, and reviewable intermediate artifacts rather than a single opaque “process” action.

### PolyWorks Inspector

PolyWorks presents a universal metrology workflow spanning scan/probe inputs, CAD definitions, GD&T controls, measurement sequences, review, reporting, and parametric updates. Its emphasis on mathematical validity, inspection planning, and result review supports persistent nominal/measured/deviation/tolerance/uncertainty fields and the pass/fail/indeterminate rule.

### Blender and Autodesk Fusion

Blender’s Outliner provides a hierarchical scene model, synchronized selection, visibility/selectability restriction columns, filtering, and stable data relationships. Fusion links browser, canvas, and timeline/find operations. These validate the project-tree authority, restriction columns, reveal-in-tree, and processing-history linkages.

### Microsoft Windows command guidance

Microsoft’s command guidance separates primary frequent commands, overflow/secondary commands, menus/context menus, and keyboard-accessible command surfaces. This supports compact visible commands without forcing every function into permanent chrome.

### WCAG 2.2

WCAG requires dragging alternatives where dragging is not essential and defines a 24×24 CSS-pixel target minimum with spacing/equivalent/essential exceptions. The density contract therefore permits compact desktop controls only with keyboard access, spacing, or equivalent targets.

## Converged principles

1. **Persistent scene/project hierarchy beats page navigation.** Engineering users manipulate entities and relationships, not dashboard cards.
2. **Selection is a system service.** Every surface must agree on what is selected.
3. **Viewport context must survive processing.** Tools appear around the evidence rather than replacing it.
4. **Complex operations are transactions.** Preview, metrics, accept/recalculate/cancel.
5. **Derived artifacts remain linked to sources.** History and provenance are navigable in one action.
6. **Metrics replace prose.** Residuals, uncertainty, state, and validation are always visible; explanations are progressive disclosure.
7. **High-frequency commands stay visible; breadth moves to menus/palette.** Density does not require discoverability loss.
8. **Professional workflows preserve intermediate states and jobs.** Long processing remains inspectable and recoverable.
9. **Metrology authority is earned, not styled.** Calibration and uncertainty constrain UI claims.
10. **Compactness is governed.** Minimal padding is tokenized and tested, not left to visual taste.

## Rejected patterns

- dashboard card grids;
- route-per-tool page replacement;
- explanatory paragraphs in technical panels;
- chat-first command surface;
- unsynchronized object lists;
- destructive direct editing of source scans;
- AI-generated classifications accepted without validation;
- hidden units or implied coordinate frames;
- modal wizards for iterative geometric work;
- color-only validation.

## Reference URLs

- https://www.cloudcompare.org/doc/wiki/index.php/Graphical_User_Interface
- https://cloudcompare.org/doc/wiki/index.php/Interactive_Segmentation_Tool
- https://cloudcompare.org/doc/wiki/index.php/Point_picking
- https://cloudcompare.org/doc/wiki/index.php/Toolbars_and_icons
- https://help.autodesk.com/view/RECAP/ENU/
- https://app.learn-one.autodesk.com/learn/ondemand/curated/recap-pro-quick-start-guide
- https://www.agisoft.com/downloads/user-manuals/
- https://www.polyworks.com/en-us/products/polyworks-inspector
- https://docs.blender.org/manual/en/5.0/editors/outliner/introduction.html
- https://help.autodesk.com/view/fusion360/ENU/?query=browser
- https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/command-bar
- https://learn.microsoft.com/en-us/windows/apps/design/basics/commanding-basics
- https://www.w3.org/TR/WCAG22/
