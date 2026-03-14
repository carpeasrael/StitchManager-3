# Sprint 16 Code Review (Cycle 2)

**Reviewer:** Claude (code review)
**Date:** 2026-03-14
**Scope:** #61, #62, #63, #64, #66

---

## Cycle 1 Finding Resolution

The single finding from cycle 1 was that new menu items (Convert, Transfer, Edit/Transform) were not managed in `updateItemStates()` in Toolbar.ts. This has been resolved:

- `menu-item-convert`: `setDisabled("menu-item-convert", !hasAny)` -- correctly disabled when no file is selected (line 316). Matches the handler pattern in main.ts which accepts both single and multi-selection.
- `menu-item-transfer`: `setDisabled("menu-item-transfer", !hasAny)` -- correctly disabled when no file is selected (line 317). Matches the handler pattern.
- `menu-item-edit-transform`: `setDisabled("menu-item-edit-transform", !hasFile || hasMulti)` -- correctly disabled when no single file is selected (line 314). Matches the handler which operates on `selectedFileId` only.
- `menu-item-versions`: `setDisabled("menu-item-versions", !hasFile || hasMulti)` -- correctly disabled when no single file is selected (line 315). Matches the handler which operates on `selectedFileId` only.

All four additions follow the existing disable/hide pattern established by the other menu items.

---

## Re-verification of All Sprint 16 Changes

### #61 -- AI event bridge (main.ts lines 135-137)
Three listeners (`ai:start`, `ai:complete`, `ai:error`) correctly bridged from Tauri to EventBus. No change since cycle 1. Still correct.

### #62 -- stopPropagation in Escape handlers
- TagInput.ts line 119: `e.stopPropagation()` present in Escape handler. Correct.
- ImagePreviewDialog.ts lines 107-108: `e.stopPropagation()` present in Escape handler. Correct.

### #63 -- outsideClickHandler leak fix (SearchBar.ts lines 179-183)
Previous handler removal guard present before `requestAnimationFrame` re-registration. Correct.

### #64 -- New Toolbar menu items and Version History handler
- Four menu items added (Convert, Transfer, Edit/Transform, Versions) with correct event emissions.
- `updateItemStates()` now manages all four items (the cycle 1 finding).
- Version History handler (main.ts lines 539-560) correctly fetches versions, handles empty state with toast, formats output, and displays via `showTextPopup()`.

### #66 -- Dedup loop cap (files.rs lines 914-929)
Loop capped at 100,000 with `AppError::Internal` on exhaustion. No change since cycle 1. Still correct.

---

## Summary

No findings. All Sprint 16 changes are correct and complete.
