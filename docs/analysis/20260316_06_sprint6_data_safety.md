# Sprint 6 Analysis: Data Safety & Portability

**Date:** 2026-03-16
**Sprint:** 6 — Data Safety & Portability
**Issues:** S6-01 through S6-06
**Requirements:** UR-057, UR-058, UR-060, UR-061, UR-062, UR-063

---

## S6-01: Soft Delete / Recycle Bin

**Requirement:** UR-057 — The app should provide a recycle bin, archive function, or recovery option for deleted entries.

### Problem Description

Currently `delete_file` in `commands/files.rs` performs a hard `DELETE FROM embroidery_files WHERE id = ?1` with cascading deletes on all child tables (tags, colors, formats, attachments, AI results, custom fields, versions). Once deleted, data is unrecoverable. UR-057 and UR-056 require protection against accidental deletion.

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| DB | `src-tauri/src/db/migrations.rs` | v13: add `deleted_at` column |
| DB | `src-tauri/src/db/queries.rs` | Add `AND deleted_at IS NULL` to FILE_SELECT constants |
| Rust | `src-tauri/src/db/models.rs` | Add `deleted_at: Option<String>` to `EmbroideryFile` |
| Rust | `src-tauri/src/commands/files.rs` | Soft-delete, restore, purge, list-trash commands |
| TS | `src/types/index.ts` | Add `deletedAt` field to `EmbroideryFile` |
| TS | `src/services/FileService.ts` | Add `restoreFile`, `purgeFile`, `getTrash` functions |
| TS | `src/components/Toolbar.ts` | Add "Papierkorb" (trash) button |
| TS | `src/components/FileList.ts` | Render trash view with restore/purge actions |
| TS | `src/state/AppState.ts` | Add `viewMode: 'library' | 'trash'` to State |
| CSS | `src/styles/components.css` | Trash view styling, muted cards for deleted files |

### Root Cause / Rationale

Hard deletes with no undo capability are a data safety risk. Users can accidentally delete files and lose all metadata, AI analysis results, tags, and custom fields. A soft-delete pattern with `deleted_at` timestamp provides:
- Immediate undo (restore from trash)
- Configurable retention period before auto-purge
- No data loss for accidental clicks

### Proposed Approach

**Migration v13** — add `deleted_at` column:
```sql
ALTER TABLE embroidery_files ADD COLUMN deleted_at TEXT;
CREATE INDEX idx_files_deleted_at ON embroidery_files(deleted_at);
```

**Query changes** — all existing queries that use `FILE_SELECT` and `FILE_SELECT_ALIASED` must filter out soft-deleted records. Rather than modifying every query call site, add `WHERE deleted_at IS NULL` awareness:
- Define `FILE_WHERE_ACTIVE` constant = `"WHERE deleted_at IS NULL"` for simple queries
- For queries that already have WHERE clauses, add `AND e.deleted_at IS NULL` condition
- The `build_query_conditions` function in `files.rs` must inject `e.deleted_at IS NULL` into the conditions list by default

**FTS5 interaction** — soft-deleted files remain in the FTS index. The FTS trigger on DELETE fires only for actual row deletion, not for soft-delete UPDATEs. Options:
- Add an FTS delete+reinsert in the UPDATE trigger to remove from search (complex)
- Simply filter soft-deleted rows in `build_query_conditions` (simpler, chosen approach) — the FTS match returns row IDs, but the subsequent JOIN on `embroidery_files` with `deleted_at IS NULL` filters them out

**New Tauri commands:**
1. `soft_delete_file(file_id)` — sets `deleted_at = datetime('now')`, replaces current `delete_file`
2. `restore_file(file_id)` — sets `deleted_at = NULL`
3. `purge_file(file_id)` — hard DELETE (existing logic, renamed)
4. `get_trash()` — `SELECT ... WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC`
5. `empty_trash()` — hard DELETE all where `deleted_at IS NOT NULL`
6. `auto_purge_trash(days)` — DELETE where `deleted_at < datetime('now', '-N days')`, called on app startup

**Auto-purge**: On app startup in `lib.rs`, after DB init, call auto-purge with configurable retention (setting key: `trash_retention_days`, default: 30).

**Frontend:**
- Add `viewMode` to `State` (`'library' | 'trash'`)
- Toolbar gets a trash icon button; clicking toggles to trash view
- In trash view, FileList shows deleted files with a muted style, restore and purge buttons replace edit actions
- Sidebar hides folder tree in trash view, shows "Papierkorb" header with count
- StatusBar shows "Papierkorb: N Dateien" in trash mode

---

## S6-02: Backup & Restore

**Requirement:** UR-058 — The app should support backup and restore of the pattern library and metadata.

### Problem Description

There is no way to back up the entire library (database + files + thumbnails). If the SQLite DB is corrupted or the app data directory is lost, all metadata, AI results, and configuration are gone.

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| Rust dep | `src-tauri/Cargo.toml` | Add `zip = "2"` crate |
| Rust | `src-tauri/src/commands/backup.rs` | New module: create_backup, restore_backup |
| Rust | `src-tauri/src/commands/mod.rs` | Add `pub mod backup;` |
| Rust | `src-tauri/src/lib.rs` | Register backup commands |
| TS | `src/services/BackupService.ts` | New: `createBackup`, `restoreBackup` |
| TS | `src/components/SettingsDialog.ts` | Add backup/restore section in General tab |
| Tauri | `src-tauri/capabilities/default.json` | May need `fs:default` for save dialog |

### Root Cause / Rationale

Data safety requires the ability to create portable snapshots of the entire library. The backup must include:
- The SQLite database (all metadata, settings, AI results, tags, custom fields)
- Thumbnail cache (so thumbnails don't need regeneration)
- Optionally: the embroidery/document files themselves (user choice, as these can be large)

### Proposed Approach

**Backup format:** ZIP archive with structure:
```
stichman_backup_2026-03-16.zip
├── manifest.json          # version, date, file count, app version
├── stitch_manager.db      # SQLite database copy
├── thumbnails/            # thumbnail cache
│   ├── SM-ABCDEFGH.png
│   └── ...
└── files/                 # (optional) original embroidery/document files
    ├── folder1/
    │   ├── design.pes
    │   └── ...
    └── ...
```

**`manifest.json` schema:**
```json
{
  "version": 1,
  "appVersion": "26.3.3",
  "createdAt": "2026-03-16T12:00:00Z",
  "dbSchemaVersion": 13,
  "fileCount": 42,
  "includesFiles": true,
  "totalSizeBytes": 123456789
}
```

**Rust crate:** `zip = "2"` — mature, well-maintained ZIP library. Features needed: `deflate` compression.

**`create_backup` command:**
1. User triggers from Settings dialog, file save dialog picks destination path
2. Lock the DB, run `VACUUM INTO '<temp_path>/backup.db'` to create a consistent snapshot without locking the main DB for the entire ZIP process. (Requires SQLite 3.27+, bundled rusqlite supports it.)
3. Collect thumbnail directory contents
4. If `include_files` flag is true, walk the `library_root` and include all tracked files
5. Build ZIP with `zip` crate, streaming entries (don't load entire files into memory)
6. Emit `backup:progress` events for UI feedback
7. Return the final file path and size

**`restore_backup` command:**
1. User selects a `.zip` file via open dialog
2. Read `manifest.json`, validate version compatibility
3. Extract and validate the DB file — run `PRAGMA integrity_check` on it
4. Close the current DB connection (requires special handling: temporarily drop the `DbState` mutex guard)
5. Replace `stitch_manager.db` with the backup copy
6. Extract thumbnails to the thumbnail cache directory
7. If backup includes files and user chose a `library_root`, extract files there
8. Re-initialize the DB connection
9. Emit `backup:restored` event so the frontend reloads state

**Safety measures:**
- Before restore, auto-create a backup of the current state (safety net)
- Validate ZIP structure before extracting (check manifest, verify DB integrity)
- Use atomic file replacement: write to temp, then rename

**Frontend (SettingsDialog):**
- Add "Datensicherung" (Backup) section with two buttons: "Sicherung erstellen" and "Sicherung wiederherstellen"
- Checkbox: "Originaldateien einschließen" (include original files)
- Progress bar during backup/restore operations
- Warning dialog before restore: "Alle aktuellen Daten werden durch die Sicherung ersetzt."

---

## S6-03: Library Migration (Portable Export with Path Remapping)

**Requirement:** UR-061 — The app should allow users to move or migrate their collection to another device without rebuilding the library manually.

### Problem Description

File paths stored in `embroidery_files.filepath` are absolute and machine-specific. Moving the library to another computer (or even changing the library root directory) breaks all file references.

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| Rust | `src-tauri/src/commands/backup.rs` | Add `export_library`, `import_library` |
| Rust | `src-tauri/src/lib.rs` | Register new commands |
| TS | `src/services/BackupService.ts` | Add `exportLibrary`, `importLibrary` |
| TS | `src/components/SettingsDialog.ts` | Add migration section |

### Root Cause / Rationale

Library portability is a fundamental requirement for users who:
- Get a new computer
- Reinstall their OS
- Move their file storage to a different drive
- Share their library with another user

The current absolute paths make this impossible without manual SQL editing.

### Proposed Approach

**Export format:** Same ZIP structure as backup (S6-02), but with path remapping in the database:
- Before writing the DB to the ZIP, rewrite all `filepath` values in `embroidery_files` and `file_formats` and `file_attachments` to relative paths (relative to `library_root`)
- Store the original `library_root` in `manifest.json` as `sourceLibraryRoot`
- Always include files in migration exports

**`export_library` command:**
1. Create a temporary copy of the DB (VACUUM INTO)
2. In the temp DB, rewrite all absolute paths to relative:
   - `filepath`: strip `library_root` prefix, store as `files/relative/path.pes`
   - `thumbnail_path`: strip thumbnail cache prefix, store as `thumbnails/SM-XXX.png`
   - `file_attachments.file_path`: similar treatment
3. Bundle into ZIP with all files and thumbnails
4. The resulting ZIP is a self-contained, portable library

**`import_library` command:**
1. User selects the ZIP and a new `library_root` destination
2. Extract files to the new `library_root`
3. Extract thumbnails to the app's thumbnail cache
4. Restore the DB, remapping all relative paths to absolute paths under the new `library_root`
5. Update the `library_root` setting
6. Restart the file watcher on the new root

**Path remapping logic:**
```rust
fn make_relative(absolute: &str, root: &str) -> String {
    Path::new(absolute)
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| absolute.to_string())
}

fn make_absolute(relative: &str, new_root: &str) -> String {
    Path::new(new_root).join(relative).to_string_lossy().to_string()
}
```

---

## S6-04: Re-link Missing Files

**Requirement:** UR-063 — The app should support re-linking missing files if storage paths have changed.

### Problem Description

If a user moves files outside the app (e.g., reorganizes folders in Finder), the stored `filepath` values become stale. There is no mechanism to detect or fix broken references.

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| Rust | `src-tauri/src/commands/files.rs` | Add `check_missing_files`, `relink_file`, `relink_folder` |
| Rust | `src-tauri/src/lib.rs` | Register new commands |
| TS | `src/services/FileService.ts` | Add `checkMissingFiles`, `relinkFile`, `relinkFolder` |
| TS | `src/components/RelinkDialog.ts` | New dialog for managing broken references |
| TS | `src/components/Toolbar.ts` | Add "Fehlende Dateien" indicator |
| TS | `src/components/FileList.ts` | Visual indicator for missing files |
| CSS | `src/styles/components.css` | Missing file indicator styles |

### Root Cause / Rationale

File references break when:
- User renames/moves folders in the OS file manager
- Library root changes without using the migration tool
- External drive is mounted at a different path
- Files are reorganized manually

The app needs to detect this and offer re-linking without losing metadata.

### Proposed Approach

**`check_missing_files` command:**
1. Query all `filepath` values from `embroidery_files`
2. Check `std::path::Path::exists()` for each
3. Return list of `{ fileId, filepath, filename }` for files that don't exist
4. Optimization: batch check, emit progress events for large libraries

**`relink_file` command:**
1. Takes `file_id` and `new_path`
2. Validates the new path exists and has a supported extension
3. Updates `filepath` in `embroidery_files`
4. Updates related `file_formats` entries if the format filepath matches the old path
5. Re-parses the file to update metadata (stitch count, colors, etc.) if it's an embroidery file
6. Regenerates thumbnail

**`relink_folder` command (batch relink):**
1. Takes `old_prefix` and `new_prefix`
2. Updates all `filepath` values that start with `old_prefix` by replacing the prefix with `new_prefix`
3. Verifies each remapped path exists, reports failures
4. Returns count of successfully relinked files

**Frontend — RelinkDialog:**
- Triggered from Toolbar (shows warning badge when missing files detected)
- Also triggered on app startup: run `check_missing_files` silently, show indicator if any found
- Dialog lists missing files with their last known path
- Two actions:
  1. "Ordner zuordnen" (Relink folder): user picks a new folder, app tries to match by filename
  2. "Datei zuordnen" (Relink file): user picks a specific file for a specific record
- Auto-match: when user provides a new folder, iterate missing files and check if a file with the same `filename` exists in the new folder — auto-relink matches

**Visual indicators:**
- FileList cards for missing files show a warning icon overlay
- StatusBar shows "N fehlende Dateien" when missing files exist
- Checking happens on app startup (setting: `check_missing_on_start`, default: true)

---

## S6-05: Structured Metadata Export (JSON/CSV)

**Requirement:** UR-060 (export metadata), UR-062 (export selected pattern records including metadata and file references)

### Problem Description

There is no way to export metadata in a machine-readable format for external tools, spreadsheets, or sharing with other users.

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| Rust dep | `src-tauri/Cargo.toml` | Add `csv = "1"` crate |
| Rust | `src-tauri/src/commands/export.rs` | New module: export_json, export_csv, import_json |
| Rust | `src-tauri/src/commands/mod.rs` | Add `pub mod export;` |
| Rust | `src-tauri/src/lib.rs` | Register export commands |
| TS | `src/services/ExportService.ts` | New: `exportJson`, `exportCsv`, `importJson` |
| TS | `src/components/Toolbar.ts` | Add export menu/button |

### Root Cause / Rationale

Metadata export serves multiple use cases:
- Sharing library catalogs without sharing actual files
- Importing metadata from external sources (batch tagging via spreadsheet)
- Interoperability with other embroidery management tools
- Data portability beyond full backup/restore

### Proposed Approach

**JSON export schema:**
```json
{
  "version": 1,
  "exportedAt": "2026-03-16T12:00:00Z",
  "files": [
    {
      "uniqueId": "SM-ABCDEFGH",
      "filename": "rose.pes",
      "name": "Rose Design",
      "theme": "Blumen",
      "description": "A rose embroidery pattern",
      "tags": ["rose", "flower", "nature"],
      "colors": [
        { "hex": "#FF0000", "name": "Red", "brand": "Madeira", "code": "1147" }
      ],
      "metadata": {
        "widthMm": 100.5,
        "heightMm": 80.2,
        "stitchCount": 12500,
        "colorCount": 5,
        "format": "PES",
        "author": "Designer Name",
        "category": "Blumen",
        "status": "completed",
        "skillLevel": "intermediate"
      },
      "customFields": {
        "Stoffart": "Baumwolle"
      }
    }
  ]
}
```

**`export_json` command:**
1. Takes optional `file_ids: Vec<i64>` (if None, exports all files)
2. Queries files with all related data (tags, colors, custom fields)
3. Serializes to JSON using the schema above
4. Uses `unique_id` as the primary identifier (survives re-imports)
5. Writes to user-selected path via save dialog

**`export_csv` command:**
1. Same selection logic as JSON
2. Flatten to CSV columns: `unique_id, filename, name, theme, description, tags (semicolon-separated), stitch_count, color_count, width_mm, height_mm, format, author, category, status, skill_level, ...`
3. Use `csv` crate for proper escaping

**`import_json` command:**
1. Read and validate the JSON file
2. Match files by `unique_id` — if a file with the same `unique_id` exists, update its metadata
3. For files without a match, skip (metadata-only import doesn't create new file records)
4. Merge tags: add new tags from the import, don't remove existing ones
5. Report: N updated, N skipped, N errors

**CSV import** is deferred — CSV is lossy for nested data (colors, tags). JSON round-trips cleanly.

**Frontend:**
- Toolbar gets an "Exportieren" dropdown or submenu with "JSON exportieren" and "CSV exportieren"
- Export respects current selection: if files are selected, export only those; otherwise export all in current folder/view
- Import option in the same menu: "JSON importieren"

---

## S6-06: Archive Function

**Requirement:** UR-057 (archive function), UR-018 (status: archived)

### Problem Description

The `status` field already supports `"archived"` as a value (added in migration v9, S1-03), and the frontend already shows "Archiviert" in status dropdowns. However, there is no dedicated archive behavior:
- Archived files are shown alongside active files
- There is no "hide archived" toggle
- There is no quick "archive" action (separate from status dropdown)

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| Rust | `src-tauri/src/commands/files.rs` | Add `archive_file`, `unarchive_file`; modify query to exclude archived by default |
| TS | `src/types/index.ts` | Add `showArchived` to `SearchParams` |
| TS | `src/components/Toolbar.ts` | Add archive/unarchive button |
| TS | `src/components/SearchBar.ts` | Add "Archivierte anzeigen" toggle |
| TS | `src/components/FileList.ts` | Muted styling for archived files |
| Rust | `src-tauri/src/db/models.rs` | Add `show_archived` to `SearchParams` |
| CSS | `src/styles/components.css` | Archived file card styling |

### Root Cause / Rationale

Archive is a middle ground between active and deleted: the user wants to keep the record and metadata but remove it from the day-to-day view. The status field already exists but has no behavioral support — it's purely cosmetic.

### Proposed Approach

**Query filtering** — modify `build_query_conditions` in `files.rs`:
- Add a new parameter `show_archived: Option<bool>` to `SearchParams` (Rust and TS)
- Default behavior (when `show_archived` is None or false): add `AND e.status != 'archived'` to conditions
- When `show_archived` is true: don't filter
- This means archived files are hidden by default everywhere (file list, search, dashboard)

**Convenience commands:**
- `archive_file(file_id)` — shortcut for `update_file_status(file_id, "archived")`
- `unarchive_file(file_id)` — shortcut for `update_file_status(file_id, "none")`
- These are thin wrappers but provide clear intent in the frontend

**Frontend changes:**
- Toolbar: when files are selected, show "Archivieren" button (archive icon). In archive view, show "Wiederherstellen" (unarchive)
- SearchBar: add a toggle/checkbox "Archivierte anzeigen" (show archived). When toggled, set `searchParams.showArchived = true` and re-query
- FileList: archived files get a muted/semi-transparent card style with an archive badge
- StatusBar: show archive count if any exist

**Interaction with soft-delete (S6-01):**
- Archived files can be soft-deleted (moved to trash)
- Trash view shows all soft-deleted files regardless of archive status
- Restoring from trash preserves the archived status

---

## Technical Decisions Summary

### Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `zip` | `2` | ZIP archive creation/extraction for backup, migration, export |
| `csv` | `1` | CSV export of metadata |

No new frontend dependencies required — all UI is vanilla TypeScript.

### Database Migration v13

All schema changes are consolidated into a single migration:
```sql
ALTER TABLE embroidery_files ADD COLUMN deleted_at TEXT;
CREATE INDEX idx_files_deleted_at ON embroidery_files(deleted_at);
```

The `deleted_at` column is the only schema change needed. Archive functionality uses the existing `status` column. Backup/restore, migration, export, and re-link are all operational features that don't require schema changes.

### New Rust Modules

| Module | Commands |
|--------|----------|
| `commands/backup.rs` | `create_backup`, `restore_backup`, `export_library`, `import_library` |
| `commands/export.rs` | `export_json`, `export_csv`, `import_json` |

Existing modules modified:
- `commands/files.rs` — soft delete, restore, purge, trash, missing file detection, relink, archive

### New Frontend Files

| File | Purpose |
|------|---------|
| `src/services/BackupService.ts` | Backup, restore, migration invoke wrappers |
| `src/services/ExportService.ts` | JSON/CSV export/import invoke wrappers |
| `src/components/RelinkDialog.ts` | UI for managing broken file references |

### Command Registration Order

All new commands must be registered in `lib.rs` `invoke_handler`:
```rust
commands::files::soft_delete_file,
commands::files::restore_file,
commands::files::purge_file,
commands::files::get_trash,
commands::files::empty_trash,
commands::files::check_missing_files,
commands::files::relink_file,
commands::files::relink_folder,
commands::files::archive_file,
commands::files::unarchive_file,
commands::backup::create_backup,
commands::backup::restore_backup,
commands::backup::export_library,
commands::backup::import_library,
commands::export::export_json,
commands::export::export_csv,
commands::export::import_json,
```

### State Changes

Add to `State` interface in `types/index.ts`:
```typescript
viewMode: 'library' | 'trash';
missingFileCount: number;
```

Add to `SearchParams`:
```typescript
showArchived?: boolean;
```

### Implementation Order

1. **S6-06 (Archive)** — simplest, modifies existing query patterns, sets the foundation for "hidden by default" filtering
2. **S6-01 (Soft delete)** — depends on archive filtering pattern, adds `deleted_at` column (migration v13)
3. **S6-04 (Re-link missing files)** — standalone feature, no dependencies on other S6 items
4. **S6-05 (Metadata export)** — standalone, adds `csv` crate and `export.rs` module
5. **S6-02 (Backup & restore)** — adds `zip` crate and `backup.rs` module
6. **S6-03 (Library migration)** — builds on backup infrastructure (S6-02)

### Event Names

| Event | Payload | Emitter |
|-------|---------|---------|
| `backup:progress` | `{ current, total, status }` | `create_backup` |
| `backup:restored` | `{}` | `restore_backup` |
| `export:progress` | `{ current, total }` | `export_json`, `export_csv` |
| `relink:progress` | `{ current, total, matched }` | `relink_folder` |
| `missing:count` | `{ count }` | startup check |
