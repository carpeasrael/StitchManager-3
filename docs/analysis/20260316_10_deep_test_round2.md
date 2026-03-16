# Analysis: Deep Test Round 2 — Issues #91, #92, #93

Date: 2026-03-16

---

## Issue #91 — Systemic: `deleted_at IS NULL` missing from individual-file operations

### Problem description

The soft-delete feature stores a `deleted_at` timestamp on `embroidery_files` rows instead of physically removing them. List/search queries correctly exclude soft-deleted rows (via `build_query_conditions`), but many individual-file operations query by `id` alone — without checking `deleted_at IS NULL`. This means soft-deleted files can still be read, updated, favorited, tagged, renamed, converted, versioned, transferred, and exported as if they were active.

### Affected files / lines and their missing filters

#### `src-tauri/src/commands/files.rs`

| Function | Line(s) | Query | Fix needed |
|---|---|---|---|
| `get_file` | 551 | `{FILE_SELECT} WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `update_file` | 743 | `UPDATE embroidery_files SET ... WHERE id = ?{idx}` | Add `AND deleted_at IS NULL` |
| `update_file` | 758 | `{FILE_SELECT} WHERE id = ?1` (re-read after update) | Add `AND deleted_at IS NULL` |
| `toggle_favorite` | 500 | `SELECT is_favorite FROM embroidery_files WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `toggle_favorite` | 509 | `UPDATE embroidery_files SET is_favorite = ?2 WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `update_file_status` | 816 | `UPDATE embroidery_files SET status = ?2 ... WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `update_file_status` | 823 | `{FILE_SELECT} WHERE id = ?1` (re-read) | Add `AND deleted_at IS NULL` |
| `set_file_tags` | 849 | `SELECT COUNT(*) > 0 FROM embroidery_files WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `get_thumbnails_batch` | 399 | `SELECT id, thumbnail_path, filepath FROM embroidery_files WHERE id IN (...)` | Add `AND deleted_at IS NULL` |
| `delete_file` | 772 | `SELECT thumbnail_path FROM embroidery_files WHERE id = ?1` | Add `AND deleted_at IS NULL` (prevents hard-deleting already-trashed files without going through purge) |

#### `src-tauri/src/commands/folders.rs`

| Function | Line(s) | Query | Fix needed |
|---|---|---|---|
| `get_folder_file_count` | 169 | `SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1` | Add `AND deleted_at IS NULL` |
| `get_all_folder_file_counts` | 184 | `SELECT folder_id, COUNT(*) FROM embroidery_files GROUP BY folder_id` | Add `WHERE deleted_at IS NULL` |
| `delete_folder` | 121 | `SELECT e.thumbnail_path FROM embroidery_files e JOIN folder_tree ...` | Add `WHERE e.deleted_at IS NULL` (cosmetic — trashed files' thumbnails should also be cleaned, so this one is debatable; leaving it as-is is acceptable) |

#### `src-tauri/src/commands/batch.rs`

| Function | Line(s) | Query | Fix needed |
|---|---|---|---|
| `batch_rename` | 130 | `{FILE_SELECT} WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `batch_organize` | 307 | `{FILE_SELECT} WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `batch_export_usb` | 519 | `SELECT filename, filepath FROM embroidery_files WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `generate_pdf_report` | 634 | `{FILE_SELECT} WHERE id = ?1` | Add `AND deleted_at IS NULL` |

#### `src-tauri/src/commands/versions.rs`

| Function | Line(s) | Query | Fix needed |
|---|---|---|---|
| `create_version_snapshot` | 31 | `SELECT filepath FROM embroidery_files WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `restore_version` | 120 | `JOIN embroidery_files ef ON ef.id = fv.file_id WHERE fv.id = ?1 AND fv.file_id = ?2` | Add `AND ef.deleted_at IS NULL` |

#### `src-tauri/src/commands/convert.rs`

| Function | Line(s) | Query | Fix needed |
|---|---|---|---|
| `convert_file_inner` | 76 | `SELECT filepath FROM embroidery_files WHERE id = ?1` | Add `AND deleted_at IS NULL` |

#### `src-tauri/src/commands/edit.rs`

| Function | Line(s) | Query | Fix needed |
|---|---|---|---|
| `load_segments` | 41 | `SELECT filepath FROM embroidery_files WHERE id = ?1` | Add `AND deleted_at IS NULL` |

#### `src-tauri/src/commands/transfer.rs`

| Function | Line(s) | Query | Fix needed |
|---|---|---|---|
| `transfer_files` | 132 | `SELECT filepath FROM embroidery_files WHERE id = ?1` | Add `AND deleted_at IS NULL` |

#### `src-tauri/src/commands/projects.rs`

| Function | Line(s) | Query | Fix needed |
|---|---|---|---|
| `duplicate_project` | 199 | `SELECT ... FROM projects WHERE id = ?1` | Add `AND deleted_at IS NULL` (projects table also has `deleted_at`) |
| `set_project_details` | 248 | `SELECT COUNT(*) > 0 FROM projects WHERE id = ?1` | Add `AND deleted_at IS NULL` |
| `update_project` | 146 | `SELECT ... FROM projects WHERE id = ?1` (no-op branch) | Add `AND deleted_at IS NULL` |
| `update_project` | 170 | `SELECT ... FROM projects WHERE id = ?1` (re-read after update) | Add `AND deleted_at IS NULL` |

#### `src-tauri/src/commands/backup.rs`

| Function | Line(s) | Query | Fix needed |
|---|---|---|---|
| `relink_batch` | 267 | `SELECT id, filepath FROM embroidery_files WHERE filepath LIKE ?1` | Add `AND deleted_at IS NULL` |
| `relink_file` | 248 | `UPDATE embroidery_files SET filepath = ?1 ... WHERE id = ?2` | Add `AND deleted_at IS NULL` |
| `import_metadata_json` | 569 | `SELECT id FROM embroidery_files WHERE unique_id = ?1` | Add `AND deleted_at IS NULL` (prevents resurrecting trashed files via JSON import) |

### Root cause

The `deleted_at` soft-delete column was added after many of these functions were already written. The list queries were updated via `build_query_conditions`, but per-id lookups and updates were not audited for the new column.

### Proposed approach

1. Add `AND deleted_at IS NULL` to every `WHERE id = ?1` query on `embroidery_files` listed above (except `delete_folder`'s thumbnail cleanup, which should clean up all thumbnails including trashed).
2. Add `AND deleted_at IS NULL` to the `projects` queries listed above.
3. Add `WHERE deleted_at IS NULL` to the folder file-count queries.
4. For `relink_batch`, add the filter to avoid re-linking trashed files.
5. For `import_metadata_json`, add the filter so trashed files are not updated.
6. Update existing tests to verify soft-deleted files are excluded from individual operations.

---

## Issue #92 — Escape cascades from dialogs

### Problem description

Four dialog components (`DocumentViewer`, `ImageViewerDialog`, `PrintPreviewDialog`, `ProjectListDialog`) each register their own `document.addEventListener("keydown", ...)` handler that listens for `Escape` and calls their respective `dismiss()` method. Meanwhile, `shortcuts.ts` registers a global `keydown` handler that emits `shortcut:escape` on every Escape press (line 15-17, no `e.stopPropagation()` or `e.preventDefault()`). The `main.ts` handler for `shortcut:escape` (line 940) then tries to close dialogs via DOM query for `.dialog-overlay` and falls back to clearing file selection.

**The cascade**: when a user presses Escape while a dialog is open:

1. The dialog's own keydown handler fires and calls `dismiss()` (closes the dialog).
2. The `shortcuts.ts` global handler **also** fires and emits `shortcut:escape`.
3. The `main.ts` handler for `shortcut:escape` runs. The dialog overlay is now gone (already dismissed in step 1), so it falls through to clearing the file selection state.

Result: pressing Escape in any dialog **also** clears the file selection, which is unintended. The user loses their selection just by closing a dialog.

Additionally, `PrintPreviewDialog` can be opened from within `DocumentViewer` (via the print button). Pressing Escape in the print preview also dismisses the document viewer (both keydown handlers fire), causing both to close simultaneously.

### Affected files / lines

| File | Line(s) | Handler |
|---|---|---|
| `src/components/DocumentViewer.ts` | 799-801 | `onKeyDown`: `if (e.key === "Escape") { DocumentViewer.dismiss(); return; }` — does NOT call `e.stopPropagation()` |
| `src/components/ImageViewerDialog.ts` | 279-281 | `onKeyDown`: `if (e.key === "Escape") { ImageViewerDialog.dismiss(); }` — does NOT call `e.stopPropagation()` |
| `src/components/PrintPreviewDialog.ts` | 141 | Arrow function: `if (e.key === "Escape") PrintPreviewDialog.dismiss();` — does NOT call `e.stopPropagation()` |
| `src/components/ProjectListDialog.ts` | 37 | Arrow function: `if (e.key === "Escape") ProjectListDialog.dismiss();` — does NOT call `e.stopPropagation()` |
| `src/shortcuts.ts` | 15-17 | `if (e.key === "Escape") { EventBus.emit("shortcut:escape"); return; }` — no stopPropagation, no preventDefault |
| `src/main.ts` | 940-961 | `shortcut:escape` handler clears selection as fallback |

### Root cause

None of the dialog Escape handlers call `e.stopPropagation()` or `e.preventDefault()`. Since all listeners are on the same `document` target, `stopPropagation()` alone would not help — they are all at the same level. The ordering of `addEventListener` calls determines which fires first, but the event always reaches all listeners.

### Proposed approach

1. In each dialog's Escape handler, call `e.stopImmediatePropagation()` after dismissing. `stopImmediatePropagation()` prevents other listeners **on the same element** from firing, which is exactly what's needed here since all listeners are on `document`.
2. Also call `e.preventDefault()` to prevent any browser default Escape behavior.
3. Verify that `PrintPreviewDialog.dismiss()` does not cascade into `DocumentViewer.dismiss()` — the `stopImmediatePropagation()` in PrintPreviewDialog's handler will prevent DocumentViewer's handler from firing for the same event.
4. In the `main.ts` `shortcut:escape` handler, add a guard: check if any dialog overlay (`.document-viewer-overlay`, `.image-viewer-overlay`, `.print-preview-overlay`, `.project-list-overlay`) is present in the DOM before clearing the selection — if a dialog was just dismissed, skip the selection clear.

---

## Issue #93 — Collection selection state

### Problem description

When a user selects a collection via the `collection:selected` event, the handler (main.ts line 445) filters `appState.get("files")` to show only the collection's files. However, the filtering only works for files that are **already loaded** in the current `files` state. If the collection contains files from folders that are not currently loaded, those files will be silently dropped.

The handler sets `selectedFolderId` to `null` (line 457), which removes the folder context, but does not fetch files from the backend by their IDs. It relies entirely on whatever files are already in memory.

### Affected files / lines

| File | Line(s) | Issue |
|---|---|---|
| `src/main.ts` | 445-464 | `collection:selected` handler filters in-memory `files` array |
| `src/main.ts` | 454 | `const allFiles = appState.get("files")` — only has files from the currently selected folder |
| `src/main.ts` | 455 | `const filtered = allFiles.filter((f) => fileIds.includes(f.id))` — misses files not in current view |

### Root cause

The handler assumes all collection files are already loaded in the `files` state. In practice, `files` only contains files for the currently selected folder (or search result). Files from other folders that belong to the collection are not present.

### Proposed approach

1. Instead of filtering in-memory state, fetch the actual file objects from the backend by their IDs. This requires either:
   - a) A new backend command `get_files_by_ids(file_ids: Vec<i64>)` that returns full `EmbroideryFile` objects, or
   - b) Calling `get_file(file_id)` for each ID and collecting results (less efficient but uses existing API).
2. Option (a) is preferred for performance. Add a new Tauri command that accepts a list of IDs and returns the matching (non-deleted) files.
3. Update the `collection:selected` handler to call this new command, then set the result into `appState.set("files", fetchedFiles)`.
4. The `selectedFolderId = null` assignment is correct (collection view is cross-folder).
5. Add a visual indicator in the UI (e.g., StatusBar) showing the active collection name so the user knows they are in collection view.
