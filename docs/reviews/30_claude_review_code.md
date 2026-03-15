# Code Review — Delete Menu Items — Round 1
Reviewer: Claude Opus 4.6
Date: 2026-03-13

## Findings

### Finding 1 — `deleteSelectedFiles()` bypasses `FileService.deleteFile()` wrapper (Low severity)

**File:** `src/main.ts`, line 191

The `deleteSelectedFiles()` function calls `invoke("delete_file", { fileId: id })` directly instead of using the existing `FileService.deleteFile(id)` wrapper (defined at `src/services/FileService.ts:50`). Every other command invocation in `main.ts` goes through a service wrapper. This breaks the established service-layer convention and would require updating two places if the command signature ever changes.

**Recommendation:** Replace `await invoke("delete_file", { fileId: id })` with `await FileService.deleteFile(id)` and remove the `invoke` import if it is no longer needed elsewhere in the file (though it is still used for `watcher_auto_import`, `watcher_remove_by_paths`, and `get_usb_devices`).

---

### Finding 2 — Partial deletion without rollback leaves inconsistent state (Medium severity)

**File:** `src/main.ts`, lines 190-200

When deleting multiple files, `deleteSelectedFiles()` loops over IDs sequentially. If the third of five deletes fails, the first two are already committed. The catch block then shows a generic error toast, but the selection is never cleared and `reloadFiles()` is never called — leaving the UI showing stale data (files that were successfully deleted still appear in the list until the user takes another action).

**Recommendation:** Move the `appState.set("selectedFileIds", [])` / `appState.set("selectedFileId", null)` and `reloadFiles()` calls into a `finally` block so the UI is always refreshed regardless of partial failure. Alternatively, report how many succeeded vs. failed (e.g., "3 von 5 Dateien geloescht, 2 fehlgeschlagen").

---

### Finding 3 — `confirm()` is a blocking browser dialog, not a Tauri dialog (Low severity)

**Files:** `src/main.ts`, lines 184, 186, 500

Both `deleteSelectedFiles()` and the `toolbar:delete-folder` handler use the native `confirm()` browser API. In a Tauri webview context, `confirm()` produces a basic OS dialog that is visually inconsistent with the app's UI and cannot be styled. The project already imports `@tauri-apps/plugin-dialog` (used in `open()` calls). Using `ask()` or `confirm()` from the Tauri dialog plugin would be more consistent and provide a non-blocking async dialog.

**Note:** This is a UX consistency point rather than a functional bug. The current `confirm()` works correctly.

---

### Finding 4 — `toolbar:delete-folder` handler does not reload folder sidebar counts (Low severity)

**File:** `src/main.ts`, lines 502-515

After deleting a folder, the handler sets `folders` from `FolderService.getAll()` and calls `reloadFiles()`. However, the Sidebar component also displays per-folder file counts. The `reloadFiles()` call updates the file list but does not trigger a sidebar file-count refresh. Depending on how the Sidebar reacts to the `folders` state change, counts may or may not be stale. Since the deleted folder is no longer in the list this is not visually broken, but if sibling folders had shared files (via future features), counts could drift.

**Verdict:** Currently harmless because folder deletion removes the folder entirely. No action required unless multi-folder file references are introduced.

---

### Summary

| # | Severity | Description |
|---|----------|-------------|
| 1 | Low | Direct `invoke()` instead of `FileService.deleteFile()` |
| 2 | Medium | Partial multi-delete failure skips UI refresh |
| 3 | Low | `confirm()` instead of Tauri dialog plugin |
| 4 | Low | Informational — folder count refresh (no action needed) |

Findings 1 and 2 should be addressed. Finding 3 is a recommended improvement. Finding 4 is informational only.
