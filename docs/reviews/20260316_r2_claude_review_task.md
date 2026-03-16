# Claude Task-Resolution Review — Issues #91, #92, #93

**Date:** 2026-03-16
**Reviewer:** Claude CLI (task-resolution)
**Scope:** Verify issues #91, #92, #93 are fully resolved

---

## Issue #91 — `deleted_at IS NULL` missing from individual-file operations and folder counts

**Status: RESOLVED**

Verification:

1. **`FILE_SELECT_LIVE_BY_ID`** (defined in `src-tauri/src/db/queries.rs:26-34`) includes `WHERE id = ?1 AND deleted_at IS NULL`. This constant is used by `get_file`, `update_file`, `update_file_status`, `batch_rename`, `batch_organize`, `generate_pdf_report`, `build_prompt_for_file`, and `ai_accept_result`.

2. **files.rs** — All individual-file operations include the guard:
   - `get_file` (line 573): uses `FILE_SELECT_LIVE_BY_ID`
   - `update_file` (line 765): `WHERE id = ?{idx} AND deleted_at IS NULL`
   - `toggle_favorite` (lines 522, 531): both SELECT and UPDATE include the guard
   - `update_file_status` (line 838): includes guard
   - `set_file_tags` (line 871): existence check includes guard
   - `get_thumbnails_batch` (line 421): includes guard
   - `build_query_conditions` (line 26): always pushes `e.deleted_at IS NULL`

3. **folders.rs** — Folder counts correctly exclude deleted files:
   - `get_folder_file_count` (line 169): `AND deleted_at IS NULL`
   - `get_all_folder_file_counts` (line 184): `WHERE deleted_at IS NULL`

4. **batch.rs** — All batch operations protected:
   - `batch_rename` (line 130): uses `FILE_SELECT_LIVE_BY_ID`
   - `batch_organize` (line 307): uses `FILE_SELECT_LIVE_BY_ID`
   - `batch_export_usb` (line 519): explicit `AND deleted_at IS NULL`

5. **Additional modules** all include the guard:
   - `versions.rs`: lines 31, 122
   - `convert.rs`: line 76
   - `edit.rs`: line 41
   - `transfer.rs`: line 132
   - `projects.rs`: multiple queries with guard (lines 58, 72, 105, etc.)
   - `backup.rs`: comprehensive coverage (lines 73, 223, 248, 267, etc.)
   - `ai.rs`: lines 103, 470 use `FILE_SELECT_LIVE_BY_ID`

---

## Issue #92 — Escape key from singleton dialogs cascades into global handler, clears file selection

**Status: RESOLVED**

Verification:

1. **All four named dialog components** use `e.stopImmediatePropagation()` on Escape:
   - `DocumentViewer.ts` (line 800): `e.stopImmediatePropagation()`
   - `PrintPreviewDialog.ts` (line 141): `e.stopImmediatePropagation()`
   - `ProjectListDialog.ts` (line 37): `e.stopImmediatePropagation()`
   - `ImageViewerDialog.ts` (line 280): `e.stopImmediatePropagation()`

2. **Defensive overlay check in `shortcuts.ts`** (lines 17-19): The global shortcut handler checks for overlay classes (`.document-viewer-overlay, .image-viewer-overlay, .print-preview-overlay, .project-list-overlay`) and skips the Escape action if any is present. This provides a belt-and-suspenders defense.

3. The combination of `stopImmediatePropagation()` in dialogs and the overlay check in shortcuts.ts ensures the Escape key cannot cascade to the global handler while a dialog is open.

---

## Issue #93 — Collection selection does not clear `selectedFileId`, causing state inconsistency

**Status: RESOLVED**

Verification:

In `src/main.ts` (lines 455-456), the `collection:selected` event handler now clears selection state before updating the file list:

```typescript
appState.set("selectedFileId", null);
appState.set("selectedFileIds", []);
appState.set("selectedFolderId", null);
appState.set("files", files);
```

Both `selectedFileId` and `selectedFileIds` are cleared to `null` and `[]` respectively before the new filtered file list is applied. This prevents the MetadataPanel from displaying a stale file that is no longer in the visible file list.

---

## Verdict

**PASS**

Task resolved. No findings.
