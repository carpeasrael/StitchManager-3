# Codex Task-Resolution Review: Issues #91, #92, #93

**Date:** 2026-03-16
**Reviewer:** Codex CLI reviewer 2
**Verdict:** PASS

---

## Issue #91 — `deleted_at IS NULL` missing from individual-file operations

### Verification

Checked every function listed in the analysis against the current source.

| File | Function(s) | Status |
|------|-------------|--------|
| `files.rs` | `get_file` | Uses `FILE_SELECT_LIVE_BY_ID` (includes `AND deleted_at IS NULL`) |
| `files.rs` | `update_file` (UPDATE + re-read) | Both have `AND deleted_at IS NULL` |
| `files.rs` | `toggle_favorite` (SELECT + UPDATE) | Both have `AND deleted_at IS NULL` |
| `files.rs` | `update_file_status` (UPDATE + re-read via `FILE_SELECT_LIVE_BY_ID`) | Fixed |
| `files.rs` | `set_file_tags` existence check | `AND deleted_at IS NULL` present |
| `files.rs` | `get_thumbnails_batch` | `AND deleted_at IS NULL` present |
| `files.rs` | `delete_file` thumbnail lookup | `AND deleted_at IS NULL` present |
| `folders.rs` | `get_folder_file_count` | `AND deleted_at IS NULL` present |
| `folders.rs` | `get_all_folder_file_counts` | `WHERE deleted_at IS NULL` present |
| `batch.rs` | `batch_rename`, `batch_organize`, `generate_pdf_report` | All use `FILE_SELECT_LIVE_BY_ID` |
| `batch.rs` | `batch_export_usb` | `AND deleted_at IS NULL` present |
| `versions.rs` | `create_version_snapshot` | `AND deleted_at IS NULL` present |
| `versions.rs` | `restore_version` | `AND ef.deleted_at IS NULL` present (verified via JOIN) |
| `convert.rs` | `convert_file_inner` | `AND deleted_at IS NULL` present |
| `edit.rs` | `load_segments` | `AND deleted_at IS NULL` present |
| `transfer.rs` | `transfer_files` | `AND deleted_at IS NULL` present |
| `projects.rs` | `duplicate_project`, `set_project_details`, `update_project` | All have `AND deleted_at IS NULL` |
| `projects.rs` | `add_to_collection` file check | `AND deleted_at IS NULL` present |
| `projects.rs` | `get_collection_files` | JOIN with `e.deleted_at IS NULL` present |
| `backup.rs` | `relink_batch`, `relink_file` | `AND deleted_at IS NULL` present |
| `backup.rs` | `import_metadata_json` | `AND deleted_at IS NULL` present |

The `FILE_SELECT_LIVE_BY_ID` constant in `queries.rs` is defined with `WHERE id = ?1 AND deleted_at IS NULL`, providing a centralized guard for all per-ID lookups.

**Result: RESOLVED**

---

## Issue #92 — Escape cascades from dialogs

### Verification

All four dialog components now call `e.stopImmediatePropagation()` on Escape:

| Component | Line | Handler |
|-----------|------|---------|
| `DocumentViewer.ts` | 800 | `e.stopImmediatePropagation()` before `dismiss()` |
| `ImageViewerDialog.ts` | 280 | `e.stopImmediatePropagation()` before `dismiss()` |
| `PrintPreviewDialog.ts` | 141 | `e.stopImmediatePropagation()` before `dismiss()` |
| `ProjectListDialog.ts` | 37 | `e.stopImmediatePropagation()` before `dismiss()` |

`stopImmediatePropagation()` prevents subsequent listeners on `document` from firing for the same event, which stops the cascade into `shortcuts.ts` and the `shortcut:escape` handler in `main.ts`. The `main.ts` escape handler also has dialog-overlay guards as additional protection.

**Result: RESOLVED**

---

## Issue #93 — Collection selection state

### Verification

The `collection:selected` handler in `main.ts` (line 445) now:
1. Calls `ProjectService.getCollectionFiles(collectionId)` to get file IDs from backend.
2. Calls `FileService.getFilesByIds(fileIds)` to fetch full file objects from backend — no in-memory filtering.
3. Sets the fetched files into `appState.set("files", files)`.

The new `get_files_by_ids` Tauri command:
- Exists in `files.rs` (line 328), registered in `lib.rs` (line 128).
- Accepts `Vec<i64>`, builds a parameterized `IN (...)` query with `AND deleted_at IS NULL`.
- Frontend wrapper exists in `FileService.ts` (line 39).

This ensures collection files from any folder are loaded, not just those already in memory.

**Result: RESOLVED**

---

## Final Verdict

All three issues (#91, #92, #93) are fully resolved. No findings.

**PASS**
