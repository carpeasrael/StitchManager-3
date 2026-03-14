# Codex Code Review — Sprint 16 Cycle 2

**Reviewer:** Codex CLI
**Date:** 2026-03-14
**Scope:** Uncommitted diff — main.ts (AI event bridge, version history handler, batch feedback), TagInput.ts & ImagePreviewDialog.ts (Escape stopPropagation), SearchBar.ts (outsideClickHandler cleanup), Toolbar.ts (new menu items with updateItemStates management), files.rs (dedup loop cap)

---

## Files Reviewed

| File | Lines | Focus Area |
|------|-------|------------|
| `src/main.ts` | 991 | AI event bridge, version history handler, batch feedback |
| `src/components/TagInput.ts` | 223 | Escape key stopPropagation in onKeydown |
| `src/components/ImagePreviewDialog.ts` | 319 | Escape key stopPropagation in keydown handler |
| `src/components/SearchBar.ts` | 619 | outsideClickHandler cleanup before re-render |
| `src/components/Toolbar.ts` | 387 | New menu items and updateItemStates management |
| `src-tauri/src/commands/files.rs` | ~1440 | Dedup loop cap at 100,000 in attach_file |

---

## Findings

**No findings.** All reviewed changes are correct, consistent with the codebase architecture, and free of defects.

---

## Detailed Review Notes

### 1. main.ts — AI event bridge (lines 104-139)

The three AI event bridge entries (`ai:start`, `ai:complete`, `ai:error` at lines 135-137) follow the identical Tauri-to-EventBus forwarding pattern as all other bridge entries. They are included in the `Promise.all` array so their unlisten functions are captured in `tauriBridgeCleanup` and torn down on HMR dispose. The `destroyTauriBridge()` function correctly iterates and clears the array. No event leaks.

### 2. main.ts — Version history handler (lines 539-560)

The `toolbar:versions` handler correctly:
- Guards on `selectedFileId` being non-null and the file existing in state.
- Calls `FileService.getFileVersions(fileId)` and handles the empty-versions case with an info toast.
- Formats each version entry with version number, operation, timestamp, and file size.
- Displays results via `showTextPopup()` which has proper overlay-click dismiss and close-button handling.
- Wraps the async call in try/catch with error toast feedback.

### 3. main.ts — Batch feedback patterns

All batch operation handlers (`toolbar:batch-rename` lines 336-356, `toolbar:batch-organize` lines 358-378, `toolbar:batch-export` lines 380-416, `toolbar:batch-ai` lines 522-537, `toolbar:mass-import` lines 458-493) follow a consistent pattern:
- Guard on selection state before proceeding.
- Open `BatchDialog` with appropriate title and count.
- Await the async operation.
- Show differentiated toast messages for success, partial failure, and full error.
- Reload files (and folder counts where appropriate) after completion.

The `toolbar:batch-export` handler (lines 380-416) correctly differentiates single-file export (no BatchDialog) from multi-file export (with BatchDialog). The `toolbar:batch-rename` and `toolbar:batch-organize` handlers both include `result.failed > 0` checks for partial-failure feedback. All handlers reload via `reloadFilesAndCounts()` or `reloadFiles()` as appropriate.

### 4. TagInput.ts — Escape stopPropagation (lines 118-121)

The Escape key handling in `onKeydown()` correctly calls `e.stopPropagation()` before `this.hideSuggestions()` and returns early. This prevents the Escape keypress from bubbling up to the global shortcut handler (which would clear file selection or dismiss a parent dialog) while still closing the suggestion dropdown. The `hideSuggestions()` method properly resets `highlightIndex` to -1 and hides the dropdown.

### 5. ImagePreviewDialog.ts — Escape stopPropagation (lines 106-118)

The keydown listener registered on `document` during `show()` calls `e.stopPropagation()` before `this.close()` for Escape presses. This prevents the Escape from reaching the global shortcut handler, so closing the image preview does not simultaneously clear file selection or close other UI elements. The cleanup chain uses the `prevCleanup` pattern (lines 114-118) to properly remove the listener, and `close()` (lines 303-317) invokes `cleanupListeners`, releases the focus trap, and removes the overlay from DOM. No listener leak.

### 6. SearchBar.ts — outsideClickHandler cleanup (lines 174-183, 158-172)

The `outsideClickHandler` lifecycle is correctly managed in three locations:
- **`renderPanel()`** (lines 179-183): Removes any pre-existing `outsideClickHandler` before creating a new panel, preventing listener accumulation when the panel is re-rendered (e.g., when filter chips trigger a re-render via `this.renderPanel()`).
- **`closePanel()`** (lines 168-171): Removes the listener and nulls the reference.
- **`destroy()`** (lines 611-617): Calls `closePanel()` which handles cleanup, then `super.destroy()`.

The `requestAnimationFrame` wrapper (line 260) correctly defers listener registration to avoid the opening click from immediately closing the panel. The guard `if (!this.panelOpen) return` inside the rAF callback handles the race where the panel is closed before the next frame executes.

### 7. Toolbar.ts — New menu items and updateItemStates (lines 61-324)

The `getMenuGroups()` method returns a five-group menu structure (Ordner, Datei, KI, Batch, System) with new items:
- `menu-item-pdf` (PDF Export)
- `menu-item-convert` (Format konvertieren)
- `menu-item-transfer` (An Maschine senden)
- `menu-item-edit-transform` (Bearbeiten/Transformieren)
- `menu-item-versions` (Versionshistorie)
- `menu-item-info` (Info, in System group)

The `updateItemStates()` method (lines 292-324) correctly manages disable/hide states based on selection:
- Single-file-only items (`reveal`, `ai`, `edit-transform`, `versions`) are disabled when `!hasFile || hasMulti`.
- Any-file items (`convert`, `transfer`) are disabled when `!hasAny`.
- Multi-only items (`batch-rename`, `batch-organize`, `batch-ai`) are hidden when `!hasMulti`.
- `pdf` and `batch-export` are hidden when `!hasAny`.
- `scan` is disabled when `!hasFolder`.

The `setDisabled` and `setHidden` helper closures (lines 301-309) safely use optional chaining on `this.panel`. State subscriptions in the constructor (lines 29-37) listen to all three relevant state keys (`selectedFolderId`, `selectedFileId`, `selectedFileIds`) to trigger `updateItemStates()`. The `EventBus.on("burger:close")` subscription ensures external close requests (e.g., from SearchBar opening its panel) are handled. The `closeMenu()` method properly cleans up the outsideClickHandler.

### 8. files.rs — Dedup loop cap (lines 914-929)

The filename deduplication loop in `attach_file()` uses `for counter in 1..=100_000u32` providing a bounded iteration. The loop:
- Tries incrementing suffixes (`{stem}_{counter}.{ext}`) until a non-existing path is found.
- Breaks immediately when an available name is found (`!dest.exists()`).
- Returns an explicit `AppError::Internal` error when the counter reaches 100,000, preventing unbounded iteration.
- Uses `u32` type which is appropriate for the range.

The check `if counter == 100_000` at the end of the loop body is necessary because the `break` only fires when no collision is found. If all 100,000 names are taken, the error is raised on the final iteration rather than silently accepting a collision. This is correct defensive programming.

---

## Summary

All reviewed changes are well-implemented and consistent with the project's architecture patterns. Event lifecycle management (registration, cleanup, HMR teardown) is thorough across all files. The Escape `stopPropagation` additions in TagInput and ImagePreviewDialog correctly prevent event bubbling conflicts. The outsideClickHandler cleanup in SearchBar prevents listener accumulation on panel re-render. Toolbar menu state management covers all selection scenarios with appropriate disable/hide logic. The dedup loop in files.rs is properly bounded with explicit error handling on exhaustion.

**Result: Zero findings.**
