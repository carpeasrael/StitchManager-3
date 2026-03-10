Issue resolved. No findings.

## Verification Details

**Issue #4:** SQLite concurrent access: missing busy_timeout and mutex held during file I/O

### Bug 1: Two independent SQLite connections without busy_timeout

**Status: RESOLVED**

The issue requested `PRAGMA busy_timeout = 5000` on the Rust connection in `init_database`.

- `src-tauri/src/db/migrations.rs` line 9: `PRAGMA busy_timeout=5000` is now set alongside `journal_mode=WAL` and `foreign_keys=ON` in `init_database`.
- The in-memory test connection (`init_database_in_memory`, line 17) also sets `busy_timeout=5000`, ensuring test parity.

This ensures the Rust-side connection will retry for up to 5 seconds when encountering a lock from the frontend's `tauri_plugin_sql` connection, preventing immediate `SQLITE_BUSY` errors.

### Bug 2: File I/O while holding the database mutex

**Status: RESOLVED**

All three files identified in the issue have been fixed, plus a related fix in `batch.rs`:

1. **`src-tauri/src/commands/ai.rs`** (both `ai_analyze_file` and `ai_analyze_batch`):
   - DB queries for `thumbnail_path` and `config` are performed under the lock, then the lock is dropped via block scope.
   - `std::fs::read(path)` + base64 encoding happens outside the lock scope.
   - The lock is re-acquired later only for storing results in the DB.

2. **`src-tauri/src/commands/files.rs`** (`get_thumbnail`):
   - DB query for `thumbnail_path` is scoped in a block that drops the lock.
   - `std::fs::read(&path)` + base64 encoding happens after the lock is released.

3. **`src-tauri/src/commands/scanner.rs`** (`watcher_auto_import`):
   - `std::fs::metadata(path)` calls are collected into a `Vec<FileInfo>` before the lock is acquired.
   - The DB lock is only acquired after all filesystem metadata has been gathered.

4. **`src-tauri/src/commands/batch.rs`** (`batch_rename`, `batch_organize`, `batch_export_usb`):
   - All three batch commands now query the DB in a scoped block, drop the lock, perform filesystem I/O (`std::fs::rename`, `std::fs::copy`, `std::fs::create_dir_all`), then re-acquire the lock for any DB updates.
   - Filesystem rollback logic on DB failure is preserved.

### Correctness of the approach

- The pattern of "query DB -> drop lock -> file I/O -> re-acquire lock -> update DB" is correctly applied throughout.
- In `batch_rename` and `batch_organize`, the rollback logic (reverting filesystem changes if the DB update fails) remains intact.
- In `watcher_auto_import`, the metadata collection before lock acquisition is clean -- the `FileInfo` struct is local and well-scoped.
- No existing tests were broken by these changes (the refactoring is purely structural, not behavioral).

### All affected components from the issue are addressed

| Component | Issue requirement | Status |
|---|---|---|
| `src-tauri/src/db/migrations.rs` | Add `busy_timeout=5000` | Done |
| `src-tauri/src/commands/ai.rs` | Drop lock before thumbnail read | Done (both paths) |
| `src-tauri/src/commands/files.rs` | Drop lock before thumbnail read | Done |
| `src-tauri/src/commands/scanner.rs` | Drop lock before `fs::metadata` | Done |
| `src-tauri/src/lib.rs` | Listed as affected | No changes needed here; the fix is correctly in `migrations.rs` where the connection is created |

All items from the issue are fully addressed.
