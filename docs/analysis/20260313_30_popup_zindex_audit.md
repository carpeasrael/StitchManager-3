# Analysis: Popup/Dialog Z-Index Audit & Requirements Check

**Date:** 2026-03-13
**Source:** User prompt (no GitHub issue)

---

## Problem Description

Verify that all popup dialogs appear in front when opened, and check the current implementation against open issue requirements.

## Affected Components

- `src/styles/components.css` — z-index declarations for all overlays
- `src/styles/layout.css` — base layout stacking context
- `src/components/SettingsDialog.ts` — settings modal
- `src/components/BatchDialog.ts` — batch progress modal
- `src/components/AiPreviewDialog.ts` — AI prompt preview modal
- `src/components/AiResultDialog.ts` — AI result review modal
- `src/components/ImagePreviewDialog.ts` — stitch preview modal
- `src/components/Toast.ts` — toast notifications
- `src/components/Toolbar.ts` — burger menu

## Z-Index Audit Findings

### Current Hierarchy

| Z-Index | Element | Position | Context |
|---------|---------|----------|---------|
| 0 | `.app-layout::before` (bg image) | absolute | Inside `#app` |
| 1 | `.app-menu`, `.app-sidebar`, etc. | relative | Inside `#app` |
| 10 | Tag input suggestions | — | Inside `.app-right` |
| 50 | `.search-advanced-panel` | — | Inside `.app-center` |
| 51 | `.search-tag-suggestions` | — | Inside `.app-center` |
| 90 | `.burger-menu` | fixed | Inside `.app-menu` (z-index: 1 context) |
| 100 | `.dialog-overlay` | fixed | Direct child of `<body>` |
| 200 | `.toast-container` | fixed | Direct child of `<body>` |

### Stacking Context Analysis

- `#app.app-layout` has `position: relative` → creates stacking context
- All grid children (`.app-menu`, `.app-sidebar`, etc.) have `position: relative; z-index: 1` → each creates own stacking context
- Dialogs appended to `document.body` → outside `#app` stacking context → z-index: 100 always wins
- Toasts appended to `document.body` → z-index: 200 always above dialogs
- Burger menu inside `.app-menu` (z-index: 1 in root context) → even with `position: fixed; z-index: 90`, it's capped by parent's z-index: 1 in root context

### Finding: Burger Menu Stacking Issue

**The burger menu's effective z-index in the root stacking context is limited by its ancestor `.app-menu` which has `z-index: 1`.** While the menu uses `position: fixed; z-index: 90`, it's rendered inside a child of `#app` which has its own stacking context. This means:

1. The burger menu IS correctly behind dialog overlays (z-index: 100) — **OK**
2. The burger menu IS closed before any dialog opens (line 222 in Toolbar.ts calls `closeMenu()` before `onClick()`) — **OK**
3. BUT the burger menu could visually clip or not overlay other panels correctly if z-index: 90 doesn't propagate outside the `z-index: 1` parent stacking context

**Fix:** Move burger menu to `document.body` (like dialogs) so its z-index operates in the root stacking context. This ensures it always appears above all app panels.

### All other dialogs: OK

All dialog overlays (Settings, Batch, AiPreview, AiResult, ImagePreview) are appended to `document.body` with `position: fixed; z-index: 100`. They correctly appear above everything.

## Requirements Check (Open Issues)

### Issue #27 — USB Device Detection: FULLY IMPLEMENTED
### Issue #30 — Thread Color Code Mapping: FULLY IMPLEMENTED
### Issue #34 — Custom Background Image: FULLY IMPLEMENTED
### Issue #45 — PDF Thumbnail Embedding: NOT YET IMPLEMENTED
- PDF report exists but does not embed stitch pattern thumbnails
- Only metadata text + color swatches + QR code are in the PDF

## Proposed Approach

1. Move burger menu DOM insertion from `this.el` to `document.body` for proper z-index layering
2. Adjust burger menu positioning to align with the burger button
3. Update close/cleanup logic accordingly
