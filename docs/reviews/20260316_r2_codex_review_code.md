# Codex Code Review -- Round 2

**Date:** 2026-03-16
**Reviewer:** Codex CLI reviewer 1
**Scope:** `src-tauri/src/commands/versions.rs`, `src-tauri/src/commands/backup.rs`
**Focus:** Verify `restore_version` and `relink_file` include `deleted_at IS NULL` guard

## Checked Queries

| Function | File | Line | `deleted_at IS NULL` present |
|----------|------|------|------------------------------|
| `create_version_snapshot` | versions.rs | 31 | Yes |
| `restore_version` | versions.rs | 122 | Yes (JOIN condition) |
| `relink_file` | backup.rs | 248 | Yes |
| `relink_batch` (SELECT) | backup.rs | 267 | Yes |
| `relink_batch` (UPDATE) | backup.rs | 282 | Yes |
| `check_missing_files` | backup.rs | 223 | Yes |
| `create_backup` | backup.rs | 73 | Yes |
| `export_metadata_json` | backup.rs | 307 | Yes |
| `export_metadata_csv` | backup.rs | 372 | Yes |
| `export_library` | backup.rs | 653 | Yes |
| `import_metadata_json` | backup.rs | 569 | Yes |
| `import_library` | backup.rs | 722 | Yes |
| `soft_delete_file` | backup.rs | 412 | Yes |
| `archive_file` | backup.rs | 514 | Yes |
| `unarchive_file` | backup.rs | 530 | Yes |
| `archive_files_batch` | backup.rs | 602 | Yes |
| `unarchive_files_batch` | backup.rs | 620 | Yes |

All queries that operate on live (non-deleted) files correctly include the `deleted_at IS NULL` guard. Trash-specific queries (`purge_file`, `restore_file`, `get_trash`, `auto_purge_trash`) correctly use `deleted_at IS NOT NULL`.

## Findings

None.

## Verdict

**PASS**
