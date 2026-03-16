# Codex Task-Resolution Review — Issues #85–#90

**Reviewer:** Codex CLI reviewer 2
**Date:** 2026-03-16
**Verdict:** PASS

---

## #85 — Soft-deleted files leak into queries

**Status: RESOLVED**

All identified query leaks have been fixed:

- `get_recent_files` (files.rs:472): `WHERE deleted_at IS NULL` present.
- `get_favorite_files` (files.rs:485): `WHERE is_favorite = 1 AND deleted_at IS NULL` present.
- `get_library_stats` (files.rs:521-533): All three queries (`total_files`, `total_stitches`, format counts via JOIN) filter on `deleted_at IS NULL`.
- `add_to_collection` (projects.rs:366): File existence check includes `AND deleted_at IS NULL`.
- `get_collection_files` (projects.rs:398-401): JOINs `embroidery_files` with `e.deleted_at IS NULL`.
- `create_backup` file query (backup.rs:72-73): Includes `AND deleted_at IS NULL`.
- `check_missing_files` (backup.rs:222-223): Also filters `deleted_at IS NULL`.

---

## #86 — restore_backup overwrites live DB

**Status: RESOLVED**

`restore_backup` (backup.rs:159-213) now:
1. Creates a safety backup of the current DB (`stitch_manager_pre_restore.db`).
2. Writes the restored DB to the same path.
3. Triggers `app_handle.exit(0)` after a 500ms delay to force a clean restart, ensuring the old in-memory connection is discarded.

This approach avoids the complexity of swapping the `Mutex<Connection>` in-place and guarantees a clean state on restart.

---

## #87 — Backup ZIP filename collision

**Status: RESOLVED**

`create_backup` (backup.rs:82-90) now:
1. Queries `SELECT id, filepath` instead of just filepath.
2. Uses `format!("files/{id}_{basename}")` for ZIP entry names, guaranteeing uniqueness even when files in different directories share the same basename.

---

## #88 — Trash dialog dangerous UX

**Status: RESOLVED**

The Cancel-triggers-purge chain has been eliminated. Two separate menu items now exist in the Toolbar:

- **"Papierkorb (Wiederherstellen)"** (`toolbar:trash`, Toolbar.ts:200-203): Shows trash count, offers restore-all via `confirm()`. Cancel simply aborts — no purge path.
- **"Papierkorb leeren"** (`toolbar:purge-trash`, Toolbar.ts:206-209): Separate destructive action with its own confirmation dialog warning that the action is irreversible.

The `toolbar:trash` handler (main.ts:376-396) now only offers restore. The `toolbar:purge-trash` handler (main.ts:398-414) is fully independent. There is no Cancel-to-purge chain.

---

## #89 — DocumentViewer cleanup

**Status: RESOLVED**

1. **Pan handler leak fixed**: Three handler references stored as class properties (`panMouseDown`, `panMouseMove`, `panMouseUp`) at DocumentViewer.ts:47-49. Registered in `buildUI()` (lines 344-350). All four event listeners (`mousedown`, `mousemove`, `mouseup`, `mouseleave`) are removed in `close()` (lines 881-890) before nulling the references.

2. **Missing `.catch()` fixed**: `getPage().then()` in `updateNavUI()` (line 444) now has `.catch(() => {})` to suppress unhandled rejections when the viewer is closed during a pending promise.

---

## #90 — import_library fragile folder lookup

**Status: RESOLVED**

`import_library` (backup.rs:730-743) now uses a proper `match` instead of `unwrap_or(1)`:
- If a folder exists, uses its ID.
- If no folder exists, creates a default folder named "Importiert" with the `new_library_root` path, then uses `last_insert_rowid()`.

This guarantees a valid `folder_id` for all imported records regardless of database state.

---

## Conclusion

All six issues (#85–#90) are fully resolved. No findings.

Task resolved. No findings.
