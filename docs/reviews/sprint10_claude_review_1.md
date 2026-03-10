# Sprint 10 - Claude Review 1

**Reviewer:** Claude (Opus 4.6)
**Date:** 2026-03-09
**Scope:** Toast notifications, SettingsDialog (5 tabs), keyboard shortcuts, splitter handles, virtual scrolling for FileList, filesystem watcher (notify crate), bundle configuration.

---

## FINDING 1 — XSS vulnerability in SettingsDialog legend

**Severity:** HIGH
**File:** `src/components/SettingsDialog.ts`, line 387
**Problem:** The `buildFilesTab` method sets `legend.innerHTML` with a hard-coded string. While the current content is static and safe, this establishes a pattern of using `innerHTML` for content rendering. More critically, in `renderCustomFieldsList` (line 538), `field.options` is rendered via `textContent` (safe), but the header at line 31 uses `innerHTML` for the dialog title. The `innerHTML` usage at line 387 is a minor concern since it is a static string, but the pattern should be noted for consistency. However, this is low risk since all dynamic data uses `textContent`.

**Recommendation:** Replace `header.innerHTML = '<span class="dialog-title">Einstellungen</span>'` (line 31-32) with DOM API calls (`createElement`/`textContent`) for consistency, matching the pattern used everywhere else in the codebase. Same for `legend.innerHTML` at line 387. This is a best-practice finding; no actual XSS exists currently since all values are static strings.

---

## FINDING 2 — Splitter does not persist resized widths

**Severity:** LOW
**File:** `src/components/Splitter.ts`
**Problem:** The Splitter component updates CSS custom properties on the document root during drag, but the values are not persisted anywhere (e.g., to `localStorage` or the database). When the user reloads the app, the sidebar and center pane revert to their default widths (240px and 480px).

**Recommendation:** After `onMouseUp`, persist the final value to the settings database (via `SettingsService.setSetting`) so it survives app restarts. Load saved values in `initComponents()` before constructing the splitters.

---

## FINDING 3 — File watcher thread never flushes events that arrive just before channel disconnection

**Severity:** LOW
**File:** `src-tauri/src/services/file_watcher.rs`, lines 55-109
**Problem:** When the `recv_timeout` returns `Disconnected` (line 84), the loop breaks immediately without flushing any accumulated `new_files` or `removed_files`. If events arrived in the last debounce window before the watcher was dropped, they are silently lost.

**Recommendation:** Before `break`, add a final flush of accumulated events:
```rust
Err(mpsc::RecvTimeoutError::Disconnected) => {
    // Flush remaining events before exiting
    if !new_files.is_empty() {
        let _ = handle.emit("fs:new-files", FsEventPayload { paths: new_files.drain(..).collect() });
    }
    if !removed_files.is_empty() {
        let _ = handle.emit("fs:files-removed", FsEventPayload { paths: removed_files.drain(..).collect() });
    }
    break;
}
```

---

## FINDING 4 — Watcher `unwrap()` on Mutex lock in setup can panic and crash the app

**Severity:** MEDIUM
**File:** `src-tauri/src/lib.rs`, line 63
**Problem:** The line `*watcher_holder.0.lock().unwrap() = Some(state);` uses `unwrap()` on a Mutex lock. If the mutex is poisoned (which shouldn't normally happen at startup, but is possible if a panic occurs in another thread), this will crash the application during initialization.

**Recommendation:** Use `.map_err()` or `.expect()` with a descriptive message, or handle the error gracefully with a `match`/`if let`, consistent with how `lock()` is handled elsewhere in the codebase (via `lock_db` helper in `error.rs`).

---

## FINDING 5 — Virtual scrolling `file-list` CSS has `gap` that conflicts with absolute positioning

**Severity:** MEDIUM
**File:** `src/styles/components.css`, lines 181-186 and `src/components/FileList.ts`
**Problem:** The `.file-list` CSS class has `gap: var(--spacing-2)` (line 184) and `display: flex; flex-direction: column`. However, in `FileList.ts`, the virtual scrolling implementation uses `position: absolute` with calculated `top` values for each card (line 131). Absolute-positioned children are removed from the flex flow, so the `gap` property has no effect on them. The `display: flex; flex-direction: column` and `gap` are dead CSS for the virtual scrolling use case. While functionally harmless, the spacer's height calculation (`files.length * CARD_HEIGHT`) does not account for any gap, which is correct only because the gap does not apply. If someone later changes the positioning approach, it would break.

**Recommendation:** Remove `gap` and `flex-direction: column` from `.file-list` or add a comment explaining that virtual scrolling uses absolute positioning and these flex properties are intentionally inert.

---

## FINDING 6 — Escape shortcut removes dialog overlay without calling close() method, bypassing cleanup

**Severity:** MEDIUM
**File:** `src/main.ts`, lines 274-284
**Problem:** The `shortcut:escape` handler directly removes the `.dialog-overlay` element from the DOM (`overlay.remove()`). However, the `SettingsDialog` and other dialog classes maintain internal state (e.g., `this.overlay` in `SettingsDialog`). Removing the overlay via DOM manipulation without calling the dialog's `close()` method leaves the dialog instance in an inconsistent state where `this.overlay` still references a detached DOM node. If any further interaction with the dialog instance occurs, it could cause unexpected behavior.

**Recommendation:** Instead of directly removing the overlay, emit a dedicated event (e.g., `EventBus.emit("dialog:close")`) that each dialog listens for and handles via its own `close()` method. Alternatively, since the overlay is already removed from the DOM, the next `SettingsDialog.open()` creates a fresh instance, so this is a minor issue in practice. Adding `this.overlay = null` after removal would be a minimal fix, but the current approach still doesn't call `close()` properly.

---

## FINDING 7 — Toast accumulation: no upper limit on concurrent toasts

**Severity:** LOW
**File:** `src/components/Toast.ts`
**Problem:** `ToastContainer.show()` appends toasts to the state array without any limit. If many operations complete rapidly (e.g., batch AI analysis of 100 files emitting a toast per completion, or rapid filesystem events), the toast container could display dozens of overlapping notifications, potentially overflowing the viewport.

**Recommendation:** Add a maximum toast count (e.g., 5). When a new toast would exceed the limit, remove the oldest one immediately.

---

## FINDING 8 — `ThemeMode` type allows only "hell"/"dunkel" but theme select values use the same strings

**Severity:** NONE (confirmation of correctness)
**File:** `src/components/SettingsDialog.ts`, lines 182-185 and `src/types/index.ts`, line 100
**Problem:** No issue. The select options use `"hell"` and `"dunkel"` which match the `ThemeMode` type definition. The cast at line 194 (`themeSelect.value as ThemeMode`) is safe because the select only contains these two values.

**Recommendation:** N/A -- this is correct.

---

## FINDING 9 — Custom field type "date" accepted on backend but not offered in frontend UI

**Severity:** LOW
**File:** `src-tauri/src/commands/settings.rs`, line 100 vs `src/components/SettingsDialog.ts`, lines 446-455
**Problem:** The Rust backend validates custom field types against `["text", "number", "date", "select"]` (includes "date"), but the frontend's type select only offers `["text", "number", "select"]` (missing "date"). This inconsistency means the "date" type can never be created through the UI, making it dead validation code on the backend.

**Recommendation:** Either add "date" (`{ value: "date", label: "Datum" }`) to the frontend select options, or remove "date" from the backend validation list to keep them in sync.

---

## FINDING 10 — `reloadFiles()` in main.ts does not pass `searchQuery` or `formatFilter`

**Severity:** MEDIUM
**File:** `src/main.ts`, lines 287-291
**Problem:** The `reloadFiles()` helper function only passes `folderId` to `FileService.getFiles()`:
```typescript
async function reloadFiles(): Promise<void> {
  const folderId = appState.get("selectedFolderId");
  const updatedFiles = await FileService.getFiles(folderId);
  appState.set("files", updatedFiles);
}
```
This ignores the current `searchQuery` and `formatFilter` state. When a filesystem watcher event triggers `reloadFiles()`, or after a file deletion, the file list will reset to show all files regardless of the active search query or format filter. The same issue exists in the `file:updated`, `toolbar:batch-rename`, `toolbar:batch-organize`, and `toolbar:batch-ai` event handlers (lines 212-210), which also call `FileService.getFiles(folderId)` without filters.

**Recommendation:** Update `reloadFiles()` to include current filters:
```typescript
async function reloadFiles(): Promise<void> {
  const folderId = appState.get("selectedFolderId");
  const search = appState.get("searchQuery");
  const formatFilter = appState.get("formatFilter");
  const updatedFiles = await FileService.getFiles(folderId, search, formatFilter);
  appState.set("files", updatedFiles);
}
```
Also update the inline `FileService.getFiles(folderId)` calls in the batch/AI event handlers to use `reloadFiles()` instead.

---

## FINDING 11 — Missing event listener permission for Tauri event system

**Severity:** LOW
**File:** `src-tauri/capabilities/default.json`
**Problem:** The capabilities file only lists `core:default`, `sql:default`, and `dialog:default`. The `core:default` permission may or may not include event listening depending on the Tauri v2 version. The file watcher emits events via `app_handle.emit()` and the frontend listens via `listen()` from `@tauri-apps/api/event`. In Tauri v2, event listening is typically covered by `core:default`, but this should be verified. If event permissions are separate, the frontend listeners for `fs:new-files` and `fs:files-removed` would silently fail.

**Recommendation:** Verify that `core:default` includes `event:default` permissions. If not, add `"event:default"` to the permissions array.

---

## Summary

| # | Severity | Finding |
|---|----------|---------|
| 1 | HIGH (best practice) | `innerHTML` usage for static strings in SettingsDialog |
| 2 | LOW | Splitter widths not persisted across restarts |
| 3 | LOW | File watcher thread drops events on channel disconnect |
| 4 | MEDIUM | `unwrap()` on Mutex lock in app setup |
| 5 | MEDIUM | CSS `gap`/`flex` properties inert with virtual scroll absolute positioning |
| 6 | MEDIUM | Escape shortcut bypasses dialog `close()` method |
| 7 | LOW | No upper limit on concurrent toast notifications |
| 8 | NONE | Theme mode type correctness confirmed |
| 9 | LOW | "date" field type in backend but not in frontend UI |
| 10 | MEDIUM | `reloadFiles()` ignores active search/format filters |
| 11 | LOW | Event permissions should be verified in capabilities |

**Total findings requiring action: 10** (Finding 8 is a confirmation of correctness, not an issue)
