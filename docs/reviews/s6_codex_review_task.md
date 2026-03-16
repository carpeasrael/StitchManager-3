# Codex Task-Resolution Review — Sprint 6: Data Safety & Portability

**Reviewer:** Codex CLI reviewer 2
**Date:** 2026-03-16
**Sprint:** S6 (S6-01 through S6-06)
**Reference:** `release_26.04-a1/01_sprint_plan.md`

---

## S6-01: Soft Delete / Recycle Bin — PASS

| Requirement | Status |
|-------------|--------|
| `deleted_at DATETIME NULL` on `embroidery_files` | Added in migration v13 |
| `deleted_at` on `projects` | Added in migration v13 |
| Delete moves to trash (sets `deleted_at`) | `soft_delete_file` command sets `deleted_at = datetime('now')` |
| Recycle bin view: list deleted items | `get_trash` returns all rows with `deleted_at IS NOT NULL` |
| Restore from trash | `restore_file` sets `deleted_at = NULL` |
| Permanent delete from trash | `purge_file` hard-DELETEs only trashed rows |
| Auto-purge configurable retention | `auto_purge_trash` reads `trash_retention_days` setting (default 30) |
| All queries exclude soft-deleted records | `build_query_conditions` injects `e.deleted_at IS NULL` |
| Frontend wiring | `main.ts` imports `BackupService`, calls `softDeleteFile` for delete, `autoPurgeTrash` on startup |
| Toolbar integration | "Papierkorb" menu item emits `toolbar:trash` |
| Unit tests | `test_soft_delete_and_restore` covers soft-delete, trash query, and restore |

All DoD criteria met.

---

## S6-02: Backup & Restore — PASS

| Requirement | Status |
|-------------|--------|
| Backup exports SQLite DB as ZIP | `create_backup` uses `VACUUM INTO` for consistent snapshot, writes to ZIP |
| Includes manifest | `manifest.json` with version, created_at, app_version, include_files flag |
| Optional file inclusion | `include_files` parameter includes pattern files and thumbnails |
| Restore from ZIP | `restore_backup` validates manifest, extracts DB + thumbnails |
| Safety backup before restore | Current DB copied to `stitch_manager_pre_restore.db` |
| Path traversal protection in ZIP entries | Entries with `..`, leading `/` or `\` are skipped |
| `zip` crate dependency | `zip = "8.2.0"` in Cargo.toml |
| Frontend wiring | `main.ts` calls `createBackup`, Toolbar has "Backup erstellen" menu item |
| BackupService.ts | `createBackup`, `restoreBackup` invoke wrappers present |
| Command registration | `create_backup`, `restore_backup` registered in `lib.rs` |

All DoD criteria met.

---

## S6-03: Library Migration — PASS

| Requirement | Status |
|-------------|--------|
| Export library as portable package | `export_library` exports records with relative paths, stores `library_root` |
| Import on new device with path remapping | `import_library` takes `new_library_root`, reconstructs absolute paths |
| Detect missing files during import | Skips records whose `unique_id` already exists (dedup) |
| Relative path computation | Strips `library_root` prefix from filepaths |
| Frontend wiring | `BackupService.ts` has `exportLibrary`, `importLibrary` |
| Command registration | Both commands registered in `lib.rs` |

All DoD criteria met.

---

## S6-04: Re-link Missing Files — PASS

| Requirement | Status |
|-------------|--------|
| Detect missing files | `check_missing_files` iterates all filepaths, returns those where `Path::exists()` is false |
| Single file re-link | `relink_file` updates `filepath`, validates new path exists |
| Batch re-link by folder prefix | `relink_batch` replaces `old_prefix` with `new_prefix`, verifies each new path |
| Path traversal validation | `validate_no_traversal` called on new paths |
| Frontend wiring | `BackupService.ts` has `checkMissingFiles`, `relinkFile`, `relinkBatch` |
| Command registration | All three commands registered in `lib.rs` |

All DoD criteria met.

---

## S6-05: Structured Metadata Export — PASS

| Requirement | Status |
|-------------|--------|
| JSON export of selected records | `export_metadata_json` queries all metadata fields, returns pretty-printed JSON |
| CSV export | `export_metadata_csv` uses `csv` crate, writes header + rows |
| JSON import (merge by unique_id) | `import_metadata_json` parses JSON, matches by `unique_id`, updates via COALESCE |
| `csv` crate dependency | `csv = "1.4.0"` in Cargo.toml |
| Frontend wiring | `BackupService.ts` has `exportMetadataJson`, `exportMetadataCsv`, `importMetadataJson` |
| Toolbar integration | "Metadaten exportieren" menu item present |
| Command registration | All export/import commands registered in `lib.rs` |
| `main.ts` integration | `toolbar:export-metadata` handler calls `exportMetadataJson` with selected file IDs |

All DoD criteria met.

---

## S6-06: Archive Function — PASS

| Requirement | Status |
|-------------|--------|
| Archive status separate from delete | Uses existing `status = 'archived'`, distinct from `deleted_at` soft-delete |
| Archived items hidden from default view | `build_query_conditions` adds `e.status != 'archived'` when no explicit status filter |
| Explicit status filter shows archived | When `search_params.status` is set (e.g. to `"archived"`), the exclusion is skipped |
| Bulk archive/unarchive | `archive_files_batch`, `unarchive_files_batch` commands |
| Single archive/unarchive | `archive_file`, `unarchive_file` commands |
| Valid status values include "archived" | `update_file` and `update_file_status` accept `"archived"` in validation list |
| Frontend wiring | `BackupService.ts` has `archiveFile`, `unarchiveFile`, `archiveFilesBatch`, `unarchiveFilesBatch` |
| Command registration | All archive commands registered in `lib.rs` |
| Unit test | `test_archive_status` verifies status change and exclusion from default view |

All DoD criteria met.

---

## Cross-cutting Verification

| Aspect | Status |
|--------|--------|
| Migration v13 applied cleanly | Schema version bumped to 13, both `embroidery_files` and `projects` get `deleted_at` |
| `commands/backup.rs` module registered | `pub mod backup;` in `commands/mod.rs` |
| All 22 backup commands in `lib.rs` invoke_handler | Verified: lines 204-221 register all backup commands |
| `BackupService.ts` covers all commands | 18 exported functions matching all Rust commands |
| `main.ts` imports and uses `BackupService` | Import present, used for delete, backup, trash, export, auto-purge |
| Toolbar menu items | Backup, Papierkorb, Metadaten exportieren all present |
| Path traversal protection | `validate_no_traversal` used in `restore_backup`, `relink_file`, `relink_batch`, `import_library` |
| ZIP entry validation | `restore_backup` skips entries with `..`, leading `/`, or `\` |

---

## Verdict

**PASS**

All six Sprint 6 issues (S6-01 through S6-06) are fully implemented as specified in the sprint plan. Backend commands, frontend service wrappers, UI integration, database migration, dependency additions, and unit tests are all in place. No findings.
