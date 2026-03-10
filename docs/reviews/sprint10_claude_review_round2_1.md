# Sprint 10 — Claude Review Round 2, Reviewer 1 (Code Review)

## Findings

### 1. Escape shortcut bypasses SettingsDialog.close(), skipping theme/font revert

**File:** `src/main.ts`, line 268-274
**Description:** The `shortcut:escape` handler does `overlay.remove()` directly via DOM query. This bypasses `SettingsDialog.close(saved=false)`, which is responsible for reverting live-preview theme and font-size changes. If a user changes the theme to "dunkel" in the Appearance tab (which applies immediately as a live preview) and then presses Escape, the theme change persists visually even though it was never saved.
**Fix:** Instead of removing the overlay directly, emit a dedicated event (e.g., `"dialog:close"`) that each dialog listens for and calls its own `close()` method. Alternatively, the Escape handler should not directly remove `.dialog-overlay` elements — the SettingsDialog already listens for overlay clicks and calls `this.close()`, so Escape should trigger the same path. A minimal fix: have SettingsDialog listen for `shortcut:escape` and call `this.close()`, and remove the generic overlay removal from main.ts (or gate it so SettingsDialog handles its own escape).

### 2. File watcher ignores Modify events — renamed-in-place or overwritten files are undetected

**File:** `src-tauri/src/services/file_watcher.rs`, lines 64-72
**Description:** The watcher only handles `EventKind::Create` and `EventKind::Remove`. `EventKind::Modify` is silently ignored. If a user overwrites an existing embroidery file (e.g., re-exports from digitizing software), the file content changes but no event is emitted to the frontend. This means the cached metadata/thumbnails will be stale until the user manually reloads.
**Fix:** Add handling for `EventKind::Modify` — either add a separate `modified_files` set and emit an `fs:files-modified` event, or simply include modified paths in `new_files` so a reload is triggered.

### 3. Toast removal animation races with re-render on rapid toasts

**File:** `src/components/Toast.ts`, lines 20-26
**Description:** When a toast is removed from state, the `render()` method adds `toast-exit` class and schedules `el.remove()` after 300ms. However, if new toasts arrive during those 300ms, `render()` is called again. The loop iterates `this.el.children` and checks against `existingIds`. The exiting toast (still in DOM for 300ms) will be found again, get another `toast-exit` class addition (no-op), and get another `setTimeout(() => el.remove(), 300)` queued. The double `el.remove()` is harmless (second call is a no-op on a detached element), but it is wasteful. More importantly, if a removed toast's ID is re-added to state within the 300ms window (unlikely but possible with rapid show/hide cycles), the element would have `toast-exit` applied and would be removed even though it should be visible.
**Fix:** Track exiting toast IDs in a `Set` and skip them in both the removal and addition loops. Or use a `data-exiting` attribute to mark elements that are animating out.

### 4. Virtual scrolling: `renderVisible()` clears `innerHTML` of spacer, destroying total height

**File:** `src/components/FileList.ts`, line ~115 (`this.listEl.innerHTML = "";`)
**Description:** `this.listEl` is the spacer `div`. Its `style.height` is set to `files.length * CARD_HEIGHT` to provide accurate scrollbar sizing. However, `renderVisible()` does `this.listEl.innerHTML = ""` which removes all child cards, then immediately re-sets `this.listEl.style.height`. The sequence is correct logically but causes a potential layout flash: the browser briefly sees a 0-height inner div (no children, height set by style but content removed). This is a minor visual flicker concern. More critically, since `listEl` IS the spacer and has `style.height` set, the `innerHTML = ""` does NOT reset the height (it only removes children), so this is actually fine. No real bug here — withdrawing this finding.

### 5. `searchQuery` state change does not reset scroll position in virtual FileList

**File:** `src/components/FileList.ts`
**Description:** When `files` state changes (due to search/filter), `render()` is called, which creates a new `scrollContainer`. This resets scroll to top, which is correct. However, if the file list length changes from a `reloadFiles()` call but `render()` is not re-triggered (only `renderVisible()` is called when `selectedFileId` changes), the spacer height could be stale. On inspection, `files` state change does trigger `render()`, so this is fine. Withdrawing.

### 6. File watcher debounce thread never exits when watcher is dropped

**File:** `src-tauri/src/services/file_watcher.rs`, lines 51-124
**Description:** When `WatcherState` is dropped (e.g., by setting `*guard = None` in `watcher_start` or `watcher_stop`), the `RecommendedWatcher` is dropped, which drops the sender side of the `mpsc` channel. The debounce thread detects `Disconnected` on line 81 and breaks. This is correct — the thread will exit. No bug here. Withdrawing.

### 7. `Splitter` does not handle touch events — unusable on touch-enabled displays

**File:** `src/components/Splitter.ts`
**Description:** This is a desktop Tauri app, so touch support is not critical. Withdrawing as a style preference.

### 8. Settings dialog: save failure still closes dialog

**File:** `src/components/SettingsDialog.ts`, line 134
**Description:** When `allOk` is false (some settings failed to save), the dialog still calls `this.close(true)`. Since `saved=true`, the live-preview changes are kept even though the save partially failed. The user sees an error toast but has no way to retry because the dialog is closed. The unsaved settings are now applied visually but not persisted.
**Fix:** When `allOk` is false, do not close the dialog. Re-enable the save button and let the user retry or cancel. Change line 134 to only call `this.close(true)` inside the `if (allOk)` block, and re-enable the button in the `else` block.

---

## Summary

4 real findings (issues 1, 2, 3, 8).
