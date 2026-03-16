# Claude Code Review — 2026-03-16 R2

**Scope:** `src-tauri/src/commands/versions.rs`, `src-tauri/src/commands/backup.rs`
**Focus:** Verify that all queries against `embroidery_files` correctly filter on `deleted_at IS NULL` (or `IS NOT NULL` where appropriate for trash operations).

## Reviewed Queries

### versions.rs
| Line | Function | Filter | Status |
|------|----------|--------|--------|
| 31 | `create_version_snapshot` | `AND deleted_at IS NULL` | OK |
| 90-93 | `get_file_versions` | queries `file_versions` only | N/A |
| 122 | `restore_version` (JOIN) | `AND ef.deleted_at IS NULL` | OK — **fixed** |
| 164 | `delete_version` | queries `file_versions` only | N/A |
| 179 | `export_version` | queries `file_versions` only | N/A |

### backup.rs
| Line | Function | Filter | Status |
|------|----------|--------|--------|
| 73 | `create_backup` | `AND deleted_at IS NULL` | OK |
| 222 | `check_missing_files` | `AND deleted_at IS NULL` | OK |
| 248 | `relink_file` | `AND deleted_at IS NULL` | OK — **fixed** |
| 267 | `relink_batch` (SELECT) | `AND deleted_at IS NULL` | OK |
| 282 | `relink_batch` (UPDATE) | `AND deleted_at IS NULL` | OK |
| 307 | `export_metadata_json` | `AND deleted_at IS NULL` | OK |
| 372 | `export_metadata_csv` | `AND deleted_at IS NULL` | OK |
| 412 | `soft_delete_file` | `AND deleted_at IS NULL` | OK |
| 429 | `restore_file` | `AND deleted_at IS NOT NULL` | OK (intentional) |
| 445 | `get_trash` | `WHERE deleted_at IS NOT NULL` | OK (intentional) |
| 467 | `purge_file` | `AND deleted_at IS NOT NULL` | OK (intentional) |
| 494 | `auto_purge_trash` | `WHERE deleted_at IS NOT NULL` | OK (intentional) |
| 514 | `archive_file` | `AND deleted_at IS NULL` | OK |
| 530 | `unarchive_file` | `AND deleted_at IS NULL` | OK |
| 569 | `import_metadata_json` | `AND deleted_at IS NULL` | OK |
| 602 | `archive_files_batch` | `AND deleted_at IS NULL` | OK |
| 620 | `unarchive_files_batch` | `AND deleted_at IS NULL` | OK |
| 653 | `export_library` | `WHERE deleted_at IS NULL` | OK |
| 722 | `import_library` | `AND deleted_at IS NULL` | OK |

## Verdict

**PASS**

Code review passed. No findings.
