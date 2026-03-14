# Claude Task-Resolution Review

**Date:** 2026-03-14
**Task:** Ensure that all menus are in front when open
**Reviewer:** Claude (task-resolution)

## Verification

### 1. Z-index stacking context issue resolved

The layout panels (`.app-menu`, `.app-sidebar`, `.app-center`, `.app-right`, `.app-splitter-l`, `.app-splitter-r`, `.app-status`) in `layout.css` lines 33-41 have `position: relative` but **no** `z-index` property. Without `z-index`, `position: relative` alone does not create a new stacking context. This means fixed-positioned children (menus, dialogs, toasts) now participate in the root stacking context and their z-index values are resolved against the viewport, not against a parent panel.

### 2. Background pseudo-element stays behind content

`.app-layout::before` at line 19-31 uses `z-index: -1`, which correctly places the decorative background image behind all content while still being visible (it paints below the root stacking level of the layout).

### 3. Menu/overlay z-index hierarchy verified

All overlay elements use `position: fixed` and their z-index values form a correct hierarchy:

| Element | Position | z-index | Status |
|---------|----------|---------|--------|
| `.tag-suggestion-list` | absolute (within panel) | 10 | OK - local dropdown |
| `.search-tag-suggestions` | absolute | 51 | OK - above tag suggestions |
| `.search-advanced-panel` | fixed | 90 | OK - overlays content |
| `.burger-menu` | fixed | 90 | OK - overlays content |
| `.dialog-overlay` | fixed | 100 | OK - above menus |
| `.toast-container` | fixed | 200 | OK - topmost |
| `.image-preview-close` | absolute (within dialog) | 2 | OK - local to preview |

Since no layout panel creates a stacking context, all fixed-positioned elements (burger menu, search panel, dialogs, toasts) correctly participate in the root stacking context. Their z-index values are globally comparable, ensuring proper layering.

## Verdict

Task resolved. No findings.

**PASS**
