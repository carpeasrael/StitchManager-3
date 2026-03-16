# Analysis: Deep Test Fixes (#85 -- #90)

**Date:** 2026-03-16
**Issues:** #85, #86, #87, #88, #89, #90

---

## #85 -- Soft-deleted files leak into queries

### Problem description

Several standalone queries omit the `deleted_at IS NULL` filter, causing soft-deleted (trashed) files to appear in active views:

1. **`get_recent_files`** (files.rs:472): Uses `FILE_SELECT` with no WHERE clause. Trashed files appear in the dashboard "Recent Files" list.
2. **`get_favorite_files`** (files.rs:485): Filters on `is_favorite = 1` but not `deleted_at IS NULL`. Trashed favorites still appear.
3. **`get_library_stats`** (files.rs:521-524): All three COUNT/SUM queries (`total_files`, `total_stitches`, format counts) include trashed files in totals. `total_folders` is fine (folders table has no soft-delete).
4. **`get_projects`** / **`get_project`** / **`update_project`** / **`delete_project`** / **`duplicate_project`** (projects.rs): These query the `projects` table which does not have a `deleted_at` column, so no filter is needed there. However, `add_to_collection` (projects.rs:365) validates file existence without checking `deleted_at IS NULL`, allowing trashed files to be added to collections. Similarly, `get_collection_files` returns IDs of trashed files.
5. **`create_backup`** (backup.rs:72): The file inclusion query selects all filepaths without filtering `deleted_at IS NULL`, so trashed files are backed up as active.

### Affected components

- `src-tauri/src/commands/files.rs` -- `get_recent_files`, `get_favorite_files`, `get_library_stats`
- `src-tauri/src/commands/projects.rs` -- `add_to_collection`, `get_collection_files`
- `src-tauri/src/commands/backup.rs` -- `create_backup` file inclusion query

### Root cause

The `build_query_conditions` function used by the main file listing correctly adds `deleted_at IS NULL`, but these standalone queries were written independently and missed the filter.

### Proposed approach

1. In `get_recent_files`: Change SQL to `"{FILE_SELECT} WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT ?1"`.
2. In `get_favorite_files`: Change SQL to `"{FILE_SELECT} WHERE is_favorite = 1 AND deleted_at IS NULL ORDER BY updated_at DESC"`.
3. In `get_library_stats`:
   - `total_files`: Add `WHERE deleted_at IS NULL`.
   - `total_stitches`: Add `WHERE deleted_at IS NULL`.
   - Format counts query: JOIN or filter against `embroidery_files` with `deleted_at IS NULL`. The `file_formats` table is a separate table; the query must be adjusted to only count non-deleted files. Add `JOIN embroidery_files ef ON ef.id = ff.file_id WHERE ef.deleted_at IS NULL` or use a subquery.
4. In `add_to_collection`: Add `AND deleted_at IS NULL` to the file existence check.
5. In `get_collection_files`: JOIN against `embroidery_files` to exclude trashed files: `SELECT ci.file_id FROM collection_items ci JOIN embroidery_files ef ON ef.id = ci.file_id WHERE ci.collection_id = ?1 AND ef.deleted_at IS NULL`.
6. In `create_backup` file inclusion: Add `AND deleted_at IS NULL` to the filepath query.

---

## #86 -- restore_backup overwrites live DB

### Problem description

`restore_backup` (backup.rs:155-171) overwrites the database file on disk (`stitch_manager.db`) by writing the ZIP-extracted bytes directly via `std::fs::write`. The app's `DbState` still holds a `Mutex<Connection>` to the old file. On macOS/Linux the old connection becomes detached; any DB operations between restore and app restart operate on stale/orphaned state.

The function returns a message asking the user to restart, but nothing prevents continued use.

### Affected components

- `src-tauri/src/commands/backup.rs` -- `restore_backup`
- `src-tauri/src/lib.rs` -- `DbState` (holds the connection)

### Root cause

The restore procedure replaces the file on disk without coordinating with the in-memory connection. SQLite on Unix uses the file descriptor, so the old connection writes to the now-unlinked inode while the new file sits at the same path.

### Proposed approach

1. After extracting the DB file from ZIP, do not overwrite the live DB file directly.
2. Instead, extract to a staging file (e.g., `stitch_manager_restored.db`).
3. Close the active connection: acquire the `DbState` mutex, drop the old `Connection`, then rename the staging file over the live file, and open a new connection to it, storing it back in the mutex.
4. Since the `DbState` uses `Mutex<Connection>` (not a connection pool), this requires:
   - Accept `db: State<'_, DbState>` in `restore_backup` (currently missing).
   - Lock the mutex, swap the connection.
5. Alternative simpler approach: After writing the restored DB, trigger an app restart via `app_handle.restart()` (Tauri v2 API) to guarantee a clean state. This is safer and simpler.
6. If `app_handle.restart()` is not available, the simplest safe fix: extract to staging path, lock `DbState`, close old connection by replacing it with a new connection opened on the restored file. Wrap in a transaction-safe way.

---

## #87 -- Backup ZIP filename collision

### Problem description

In `create_backup` (backup.rs:84-86), files are added to the ZIP using only the basename:

```rust
let entry_name = format!("files/{}", path.file_name()...);
```

If two files in different directories share the same filename (e.g., `/designs/rose.pes` and `/archive/rose.pes`), the second silently overwrites the first in the ZIP. Only the last file with a given name survives.

### Affected components

- `src-tauri/src/commands/backup.rs` -- `create_backup`, file inclusion loop (lines 80-93)

### Root cause

The entry name uses only `file_name()` (basename) without incorporating the directory structure or a disambiguation suffix.

### Proposed approach

1. Use the file's database ID to guarantee uniqueness: `format!("files/{}_{}", file_id, basename)`.
2. This requires changing the query to also select the file ID: `SELECT id, filepath FROM embroidery_files WHERE ...`.
3. Alternative: use a relative path from the library root (similar to `export_library`), but this is more complex and requires the library_root setting.
4. The ID-prefix approach is simpler and guarantees uniqueness without depending on external config.
5. Update the query to: `SELECT id, filepath FROM embroidery_files WHERE filepath IS NOT NULL AND filepath != '' AND deleted_at IS NULL` (also fixes #85 for backup).

---

## #88 -- Trash dialog dangerous UX

### Problem description

The `toolbar:trash` handler (main.ts:376-401) uses a two-step `confirm()` flow:

```
First dialog:  OK = Restore all  |  Cancel = Purge trash
Second dialog: Confirm permanent delete?
```

The problem: clicking "Cancel" on the first dialog (which a user would naturally do to dismiss/abort) instead triggers the purge flow. This inverts the standard Cancel = abort convention.

### Affected components

- `src/main.ts` -- `toolbar:trash` event handler (lines 376-401)

### Root cause

Using `confirm()` as a branching mechanism (OK = action A, Cancel = action B) violates the standard UI pattern where Cancel means "do nothing."

### Proposed approach

Replace the `confirm()` flow with a proper three-button dialog. Since this is a vanilla TS project without a framework dialog component:

1. Create a new method or inline function that renders a custom dialog overlay with three explicit buttons:
   - "Wiederherstellen" (Restore all) -- primary action
   - "Papierkorb leeren" (Purge) -- destructive action, styled as danger
   - "Abbrechen" (Cancel) -- closes dialog, does nothing
2. The purge button should still show a second confirmation ("Papierkorb wirklich endgueltig leeren?") before proceeding.
3. Show the list of trashed files (count and names) in the dialog body.
4. Return a Promise that resolves to the user's choice.

---

## #89 -- DocumentViewer cleanup

### Problem description

Two issues in `DocumentViewer.ts`:

1. **Pan handler leak** (lines 341-349): Four mouse event handlers (`mousedown`, `mousemove`, `mouseup`, `mouseleave`) are registered on `canvasContainer` as inline arrow functions. In `close()` (lines 867-890), only `wheel` and `keydown` handlers are removed. The four mouse handlers are never removed. While the DOM element is removed (which detaches the handlers from the event loop), the closures retain references to `this` (the DocumentViewer instance), preventing garbage collection until the DOM node is collected.

2. **Missing `.catch()`** (lines 441-444): `updateNavUI` calls `this.pdfDoc.getPage(this.currentPage).then(...)` without a `.catch()`. If the viewer is closed while this promise is pending, `getPage` may reject (destroyed document), producing an unhandled promise rejection.

### Affected components

- `src/components/DocumentViewer.ts` -- `buildUI()` (pan handlers), `updateNavUI()` (zoom label), `close()` (cleanup)

### Root cause

1. The pan handlers were added as inline functions without storing references, unlike `wheelHandler` and `keyHandler` which are stored as class properties.
2. The `.then()` chain in `updateNavUI` simply omits error handling.

### Proposed approach

1. **Pan handlers**: Store the four mouse handlers as class properties (like `wheelHandler` and `keyHandler`). In `close()`, remove them from `canvasContainer` before nulling the reference:
   - Add properties: `mouseDownHandler`, `mouseMoveHandler`, `mouseUpHandler`, `mouseLeaveHandler`.
   - In `buildUI()`, assign these properties and register them.
   - In `close()`, remove all four before setting `canvasContainer = null`.

2. **Missing `.catch()`**: Add `.catch(() => {})` to the `getPage().then()` chain in `updateNavUI()`, or guard with `if (!this.pdfDoc) return` and wrap in try/catch. The simplest fix:
   ```typescript
   this.pdfDoc.getPage(this.currentPage).then((page) => {
     const scale = this.getEffectiveScale(page);
     zoomLabel.textContent = `${Math.round(scale * 100)}%`;
   }).catch(() => {});
   ```

---

## #90 -- import_library fragile folder lookup

### Problem description

In `import_library` (backup.rs:717-721):

```rust
let folder_id: i64 = conn.query_row(
    "SELECT id FROM folders LIMIT 1", [], |row| row.get(0),
).unwrap_or(1);
```

If no folder exists in the database (fresh install, or all folders deleted), the query returns no rows, and `unwrap_or(1)` assigns `folder_id = 1`. Since folder ID 1 does not exist, every subsequent `INSERT INTO embroidery_files` with `folder_id = 1` violates the foreign key constraint, and the entire import fails silently (or errors out).

### Affected components

- `src-tauri/src/commands/backup.rs` -- `import_library` (lines 716-735)

### Root cause

The `unwrap_or(1)` assumes folder ID 1 always exists, which is not guaranteed.

### Proposed approach

1. Before the import loop, check if any folder exists.
2. If no folder exists, create a default one:
   ```rust
   let folder_id: i64 = match conn.query_row(
       "SELECT id FROM folders LIMIT 1", [], |row| row.get::<_, i64>(0),
   ) {
       Ok(id) => id,
       Err(_) => {
           conn.execute(
               "INSERT INTO folders (name, path) VALUES ('Importiert', ?1)",
               rusqlite::params![new_library_root],
           )?;
           conn.last_insert_rowid()
       }
   };
   ```
3. This guarantees a valid folder_id for all imported records and uses the `new_library_root` path as the folder path, which is semantically correct.

---

## Summary of changes

| Issue | File(s) | Change type |
|-------|---------|-------------|
| #85 | `files.rs`, `projects.rs`, `backup.rs` | Add `deleted_at IS NULL` filters |
| #86 | `backup.rs` (+ potentially `lib.rs`) | Close/reopen DB connection on restore |
| #87 | `backup.rs` | Use file ID prefix in ZIP entry names |
| #88 | `main.ts` | Replace confirm() with custom 3-button dialog |
| #89 | `DocumentViewer.ts` | Store + remove pan handlers, add .catch() |
| #90 | `backup.rs` | Create default folder if none exists |
