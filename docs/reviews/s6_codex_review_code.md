# Sprint 6 — Codex Code Review

**Reviewer:** Codex CLI reviewer 1
**Scope:** Sprint 6 backup/restore/trash/export/import/archive changes
**Verdict:** PASS

## Files Reviewed

- `src-tauri/src/commands/backup.rs` (new)
- `src-tauri/src/commands/files.rs` (soft-delete filter)
- `src-tauri/src/db/migrations.rs` (apply_v13)
- `src-tauri/src/commands/mod.rs` (pub mod backup)
- `src-tauri/src/lib.rs` (command registration)
- `src/services/BackupService.ts` (new)
- `src/main.ts` (event handlers, soft-delete integration)
- `src/components/Toolbar.ts` (backup/trash/export menu items)

## Findings

Code review passed. No findings.

## Summary

All Sprint 6 changes are well-structured and consistent with the existing codebase:

- **backup.rs:** Proper error handling throughout. Path traversal validation applied to user-supplied paths (`restore_backup`, `relink_file`, `relink_batch`, `import_library`). ZIP entry names validated against `..`, leading `/`, and leading `\` to prevent extraction-based path traversal. VACUUM INTO used for safe DB backup without locking. Temp file cleaned up. All commands correctly use `lock_db()` and return `AppError`.
- **files.rs:** `build_query_conditions` correctly injects `e.deleted_at IS NULL` as a mandatory filter, ensuring soft-deleted files are excluded from all normal queries. Archived files filtered with `e.status != 'archived'` where appropriate.
- **migrations.rs:** `apply_v13` adds `deleted_at TEXT` column to both `embroidery_files` and `projects` tables with indexes. Migration version correctly bumped to 13. `CURRENT_VERSION` matches.
- **mod.rs:** `pub mod backup` properly declared alongside existing modules.
- **lib.rs:** All 19 backup commands registered in `generate_handler![]` macro.
- **BackupService.ts:** Clean TypeScript wrappers with correct camelCase parameter mapping to Rust snake_case. Return types match Rust structs.
- **main.ts:** `deleteSelectedFiles()` correctly uses `BackupService.softDeleteFile()` instead of hard delete. Trash UI via confirm dialogs. Auto-purge on startup (fire-and-forget). Backup/trash/export-metadata event handlers properly wired.
- **Toolbar.ts:** Backup, trash, and metadata export items added to System menu group with appropriate icons and event emissions.
