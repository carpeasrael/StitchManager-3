# Task Resolution Review -- Issues #85 through #90

**Reviewer:** Claude (task-resolution)
**Date:** 2026-03-16

## Verdict: PASS

Task resolved. No findings.

---

## Issue-by-Issue Verification

### #85 -- Soft-deleted files leak into queries

All six locations identified in the analysis have been patched:

| Query | File | Fix |
|-------|------|-----|
| `get_recent_files` | `files.rs:472` | `WHERE deleted_at IS NULL` added |
| `get_favorite_files` | `files.rs:485` | `AND deleted_at IS NULL` added |
| `get_library_stats` (total_files) | `files.rs:522` | `WHERE deleted_at IS NULL` added |
| `get_library_stats` (total_stitches) | `files.rs:526` | `WHERE deleted_at IS NULL` added |
| `get_library_stats` (format_counts) | `files.rs:530-533` | JOIN with `e.deleted_at IS NULL` |
| `add_to_collection` | `projects.rs:366` | `AND deleted_at IS NULL` in existence check |
| `get_collection_files` | `projects.rs:399-401` | JOIN filtering `e.deleted_at IS NULL` |
| `create_backup` file query | `backup.rs:73` | `AND deleted_at IS NULL` added |

**Status: RESOLVED**

### #86 -- restore_backup overwrites live DB

The restore procedure now:
1. Creates a safety backup of the current DB (`stitch_manager_pre_restore.db`, backup.rs:163-166)
2. Extracts the restored DB to the target path
3. Triggers an app exit after 500ms via `app_handle.exit(0)` (backup.rs:207-210)

This avoids the stale connection problem by forcing a full restart. The approach matches the analysis alternative (exit instead of reconnection swap).

**Status: RESOLVED**

### #87 -- Backup ZIP filename collision

The backup file query now selects `id` alongside `filepath` (backup.rs:72). ZIP entry names use `format!("files/{id}_{basename}")` (backup.rs:90), guaranteeing uniqueness via the database primary key.

**Status: RESOLVED**

### #88 -- Trash dialog dangerous UX

The dangerous Cancel-triggers-purge flow has been eliminated. The implementation splits the operations into two separate events:
- `toolbar:trash` (main.ts:376-396): Shows trash count and offers restore-all only. Cancel aborts.
- `toolbar:purge-trash` (main.ts:398-414): Separate action with its own explicit confirmation for permanent deletion.

This is a valid alternative to the proposed 3-button dialog. The core issue (Cancel meaning purge) is fully resolved.

**Status: RESOLVED**

### #89 -- DocumentViewer cleanup

1. **Pan handler leak fixed:** Three handler properties added (`panMouseDown`, `panMouseMove`, `panMouseUp` at DocumentViewer.ts:47-49). Registered in `buildUI()` (lines 344-350). All four listeners (mousedown, mousemove, mouseup, mouseleave) are removed in `close()` (lines 880-890).

2. **Missing `.catch()` fixed:** `getPage().then()` in `updateNavUI()` now chains `.catch(() => {})` (line 444).

**Status: RESOLVED**

### #90 -- import_library fragile folder lookup

The `unwrap_or(1)` pattern has been replaced with a match block (backup.rs:730-743) that creates a default folder named "Importiert" with the new library root path when no folder exists. This prevents FK constraint violations on fresh databases.

**Status: RESOLVED**
