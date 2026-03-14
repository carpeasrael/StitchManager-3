# Sprint 12 Analysis — Critical Data Safety & Integrity Fixes

**Date:** 2026-03-14
**Issues:** #46, #47, #48, #49
**Severity:** All critical

---

## Issue #46 — Duplicate toolbar:delete-folder handler causes double confirmation dialogs

### Problem description
Two separate `EventBus.on("toolbar:delete-folder", ...)` handlers are registered in `initEventHandlers()` in `src/main.ts`. When the event fires, both execute sequentially — the user sees two confirmation dialogs for a single delete operation.

### Affected components
- `src/main.ts` lines 403–441 (handler 1) and lines 742–765 (handler 2)

### Root cause
Merge artifact — two implementations of the same handler were both committed. The second handler is a simpler, earlier version.

### Proposed approach
1. **Remove the second handler** (lines 742–765) entirely
2. **Keep the first handler** (lines 403–441) — it is the more complete implementation:
   - Fetches file count via `FolderService.getFileCount()`
   - Detects subfolders and includes them in the warning
   - Uses proper UTF-8 encoding for German text ("löschen" vs "loeschen")
3. Verify no other duplicate handlers exist (analysis confirms none found)

---

## Issue #47 — restore_version SQL bug skips pre-restore snapshots after first restore

### Problem description
In `src-tauri/src/commands/versions.rs` lines 133–138, the SQL query intended to check if the *most recent* version is a restore operation actually checks if *any* restore operation has ever existed for the file. After the first restore, pre-restore snapshots are permanently disabled, risking data loss on subsequent restores.

### Affected components
- `src-tauri/src/commands/versions.rs` — `restore_version()` function, lines 111–153
- `src-tauri/src/db/migrations.rs` — `file_versions` table schema (v8)

### Root cause
The SQL query `SELECT COUNT(*) > 0 FROM file_versions WHERE file_id = ?1 AND operation = 'restore' ORDER BY version_number DESC LIMIT 1` is logically broken:

- `COUNT(*)` aggregates ALL matching rows into a single result row
- `ORDER BY ... LIMIT 1` operates on the aggregation result (always 1 row), not on the source rows
- Result: returns `true` if any restore ever existed, not just the most recent version

### Proposed approach
Replace the query with one that checks whether the most recent version entry is a restore:

```sql
SELECT operation FROM file_versions
WHERE file_id = ?1
ORDER BY version_number DESC
LIMIT 1
```

Then check if the result equals `'restore'`. If the most recent operation is already a restore, skip the pre-restore snapshot (to avoid duplicates). Otherwise, create one.

Updated Rust code:
```rust
let recent_restore: bool = conn.query_row(
    "SELECT operation FROM file_versions WHERE file_id = ?1 \
     ORDER BY version_number DESC LIMIT 1",
    [file_id],
    |row| {
        let op: String = row.get(0)?;
        Ok(op == "restore")
    },
).unwrap_or(false);
```

---

## Issue #48 — watcher_auto_import holds DB lock during thumbnail I/O — app freezes

### Problem description
`watcher_auto_import()` (line 604) and `import_files()` (line 211) acquire the DB mutex and never release it until the function returns. Thumbnail generation (file parsing, stitch rendering, PNG writing) happens while the lock is held, blocking all other DB operations and freezing the entire app.

### Affected components
- `src-tauri/src/commands/scanner.rs`:
  - `import_files()` — lock acquired line 211, thumbnails generated lines 265–279
  - `watcher_auto_import()` — lock acquired line 604, thumbnails generated lines 665–679
  - `mass_import()` — **correct pattern** at line 506: `drop(conn)` before thumbnails
- `src-tauri/src/services/thumbnail.rs` — expensive I/O (parse stitches, render image, write PNG)

### Root cause
`mass_import()` was fixed to drop the lock before thumbnails (`drop(conn)` at line 506, re-acquire per update), but `import_files()` and `watcher_auto_import()` were never updated to match.

### Proposed approach
Apply the same pattern from `mass_import()` to both functions:

1. **`import_files()`**: After the transaction commits (line ~261), `drop(conn)`. In the thumbnail loop, re-acquire with `lock_db(&db)` for each UPDATE.
2. **`watcher_auto_import()`**: After the transaction commits (line ~661), `drop(conn)`. In the thumbnail loop, re-acquire with `lock_db(&db)` for each UPDATE. Also re-acquire for the post-thumbnail event emission queries.

Pattern to follow (from `mass_import` line 506–525):
```rust
drop(conn);  // Release lock before expensive I/O

for (id, filepath, ext) in &thumb_pending {
    if let Ok(data) = std::fs::read(Path::new(filepath)) {
        match thumb_state.0.generate(*id, &data, ext) {
            Ok(thumb_path) => {
                if let Ok(c) = lock_db(&db) {
                    let _ = c.execute(
                        "UPDATE embroidery_files SET thumbnail_path = ?2 WHERE id = ?1",
                        rusqlite::params![id, thumb_path.to_string_lossy().as_ref()],
                    );
                }
            }
            Err(e) => { ... }
        }
    }
}
```

---

## Issue #49 — Batch rename/organize silent rollback failure can orphan files

### Problem description
In `batch_rename()` (lines 253–261) and `batch_organize()` (lines 442–449), when the DB transaction fails after filesystem renames succeeded, the rollback uses `let _ = std::fs::rename(...)` — silently discarding any rollback errors. If rollback fails, files exist on disk under new names but the database still contains original paths, making them invisible to the application.

### Affected components
- `src-tauri/src/commands/batch.rs`:
  - `batch_rename()` — rollback at lines 253–261
  - `batch_organize()` — rollback at lines 442–449
- `src/main.ts` — batch event handlers (lines 333–363) only `console.warn` errors
- `src/types/index.ts` — `BatchResult` interface lacks rollback status

### Root cause
The 3-phase design (read DB → rename files → update DB) deliberately releases the DB lock during filesystem operations for performance. Rollback is best-effort with no success validation — failures are silently discarded via `let _ =`.

### Proposed approach

1. **Capture and log rollback failures** — replace `let _ =` with explicit error handling:
```rust
if let Err(e) = tx_result {
    let mut rollback_failures: Vec<String> = Vec::new();
    for op in &pending_updates {
        if op.did_rename {
            if let Err(rb_err) = std::fs::rename(&op.new_path, &op.old_path) {
                log::error!(
                    "Rollback failed for file {}: {} -> {}: {}",
                    op.file_id, op.new_path, op.old_path, rb_err
                );
                rollback_failures.push(format!(
                    "{}: {}", op.new_path, rb_err
                ));
            }
        }
    }
    if rollback_failures.is_empty() {
        return Err(AppError::Database(e));
    } else {
        return Err(AppError::Internal(format!(
            "DB-Transaktion fehlgeschlagen: {}. Rollback fehlgeschlagen für {} Dateien: {}",
            e, rollback_failures.len(), rollback_failures.join("; ")
        )));
    }
}
```

2. **Apply identical fix to both** `batch_rename()` and `batch_organize()`
3. The error message will reach the frontend via the existing error propagation path, and with issue #58 (Sprint 14) the toast will display it to the user

---

## Summary of Changes

| Issue | File | Change |
|-------|------|--------|
| #46 | `src/main.ts` | Remove duplicate handler at lines 742–765 |
| #47 | `src-tauri/src/commands/versions.rs` | Fix SQL query at lines 133–138 |
| #48 | `src-tauri/src/commands/scanner.rs` | Drop DB lock before thumbnail I/O in `import_files` and `watcher_auto_import` |
| #49 | `src-tauri/src/commands/batch.rs` | Capture and report rollback failures in `batch_rename` and `batch_organize` |

All fixes are isolated, non-overlapping, and follow existing patterns in the codebase.
