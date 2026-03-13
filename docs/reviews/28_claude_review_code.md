# Code Review — Issues #43, #44 — Round 2
Reviewer: Claude Opus 4.6
Date: 2026-03-13
Scope: Verification of Round 1 fixes + new issue scan

## Verification of Round 1 Fixes

### Fix 1: Missing `role="menu"` on burger menu panel (Round 1 Finding 1)
**Status: FIXED**
`src/components/Toolbar.ts` line 182 now sets `this.panel.setAttribute("role", "menu")` immediately after creating the panel element. Individual menu items also correctly have `role="menuitem"` (line 202). The ARIA menu structure is now valid.

### Fix 2: `requestAnimationFrame` race condition (Round 1 Finding 7)
**Status: FIXED**
`src/components/Toolbar.ts` line 235 now includes the guard `if (!this.menuOpen) return;` at the top of the `requestAnimationFrame` callback. This prevents the outside-click handler from being registered after a rapid open-close sequence, eliminating the event listener leak.

### Fix 3: `overflow: hidden` clipping burger menu
**Status: FIXED**
`src/styles/components.css` line 1108 now uses `position: fixed` instead of `position: absolute` for `.burger-menu`. Since `position: fixed` positions relative to the viewport, the menu is no longer subject to any ancestor's `overflow: hidden` clipping. The `top: 40px; left: var(--spacing-3)` coordinates position the menu correctly below the toolbar.

### Fix 4: z-index collision with dialog overlay
**Status: FIXED**
`.burger-menu` z-index is `90` (line 1116), while `.dialog-overlay` z-index is `100` (line 1254). The burger menu correctly renders below dialog overlays. The full z-index layering order is coherent: autocomplete/dropdown(10) < metadata-suggestions(50/51) < burger-menu(90) < dialog-overlay(100) < toast(200).

### Fix 5: `batch.rs` hardcoded `~/Stickdateien` fallback
**Status: FIXED**
`src-tauri/src/commands/batch.rs` lines 277-283: the `batch_organize` function queries `library_root` from the settings table and maps a missing/empty value to `AppError::Validation("library_root ist nicht konfiguriert")`. The string "Stickdateien" no longer appears anywhere in the file.

## New Review

Code review passed. No findings.
