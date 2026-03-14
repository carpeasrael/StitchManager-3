# Sprint 11 — Claude Code Review

**Scope:** `versions.rs`, `transfer.rs`, `edit.rs`, `convert.rs`
**Focus:** restore_version infinite loop, transfer path traversal, blob memory, prune query, lock management

---

## Finding 1 — `restore_version` can create an infinite version loop (HIGH)

**File:** `src-tauri/src/commands/versions.rs`, line 124

`restore_version` calls `create_version_snapshot` before writing the restored data to disk. If a user repeatedly restores the same version, every restore creates a new snapshot of the current state, which itself was a restore. This is not technically an infinite loop at runtime (each call is a single snapshot + restore), but it generates unbounded version churn: restoring version N creates version N+1, and restoring N again creates N+2, and so on. Combined with the MAX_VERSIONS_PER_FILE=10 prune limit, rapid restore-undo-restore cycles will evict meaningful pre-edit versions from the history, replacing them with restore-of-restore snapshots that all contain the same data.

**Recommendation:** Either skip snapshotting when the operation is "restore" (since the version being restored is already preserved in the table), or deduplicate by comparing file content hashes before inserting a new version row.

---

## Finding 2 — `export_version` has no path sanitization (MEDIUM)

**File:** `src-tauri/src/commands/versions.rs`, line 152-171

The `path` parameter in `export_version` is a user-supplied string passed directly to `std::fs::write` with no sanitization. Unlike `batch.rs`, `scanner.rs`, and `files.rs` which all reject paths containing `..`, this command does not check for path traversal. A malicious or buggy frontend call could write arbitrary file data to any writable location on the filesystem.

**Recommendation:** Add path traversal validation consistent with the rest of the codebase (reject paths containing `..`).

---

## Finding 3 — `transfer_files` has no path traversal check on `transfer_path` (MEDIUM)

**File:** `src-tauri/src/commands/transfer.rs`, lines 98-189

The `transfer_path` stored in `machine_profiles` is used directly as the destination directory. While `add_machine` stores it from user input and it is read back from the database, there is no validation that the path does not contain traversal sequences. The `exists() && is_dir()` check on line 116 verifies the directory exists but does not prevent writing to sensitive locations (e.g., `/etc/` or `~/`). Additionally, the filename from the source file is joined to the destination path without sanitization -- a source file named `../../etc/cron.d/evil` would resolve outside the transfer directory.

**Recommendation:** Canonicalize the destination path and verify the final resolved output path is still within the intended transfer directory. Also sanitize the source filename component before joining.

---

## Finding 4 — Blob storage loads entire file into memory without size limit (MEDIUM)

**File:** `src-tauri/src/commands/versions.rs`, lines 41-46 and 110-121

`create_version_snapshot` reads the entire file into memory via `std::fs::read` and stores it as a BLOB in SQLite. `restore_version` reads the entire BLOB back into a `Vec<u8>`. There is no size limit check. Embroidery files are typically small, but a corrupt or very large file could cause excessive memory usage. With up to 10 versions per file stored as BLOBs, the database can grow significantly (10 files x 10 versions x file_size).

**Recommendation:** Add a maximum file size check before reading (e.g., reject files over 50 MB) to prevent accidental memory exhaustion.

---

## Finding 5 — `convert_file_inner` acquires and releases the DB lock twice (LOW)

**File:** `src-tauri/src/commands/convert.rs`, lines 68-87

The function acquires the mutex lock once for `create_version_snapshot` (line 70), drops it, then acquires it again to query the filepath (line 76). Since `create_version_snapshot` already queries the filepath internally (versions.rs line 30), this results in a redundant database query. The two lock acquisitions are not atomic, so in theory the file could be deleted between them (TOCTOU), though this is unlikely in a single-user desktop app.

**Recommendation:** Consider refactoring so the filepath is returned from `create_version_snapshot` or queried once under a single lock scope. This would also eliminate the duplicate query.

---

## Finding 6 — `transfer_files` acquires lock three times in sequence (LOW)

**File:** `src-tauri/src/commands/transfer.rs`, lines 103-186

The function acquires the DB lock three separate times: once for machine profile lookup (line 104), once for file path queries (line 122), and once for updating `last_used` (line 181). While each acquisition is individually correct, the repeated lock/unlock pattern increases contention risk and means the operation is not atomic. A concurrent delete of the machine profile between the first and third lock could cause silent failure of the `last_used` update (which is already handled by ignoring the error).

**Recommendation:** This is acceptable for a single-user desktop app, but documenting the non-atomic nature would be helpful. Consider combining the first two lock scopes since they are sequential with no I/O between them.

---

## Finding 7 — Prune query correctness verified (INFO)

**File:** `src-tauri/src/commands/versions.rs`, lines 66-70

The prune query `DELETE FROM file_versions WHERE file_id = ?1 AND id NOT IN (SELECT id FROM file_versions WHERE file_id = ?1 ORDER BY version_number DESC LIMIT ?2)` is correct. It deletes rows for the given `file_id` whose `id` is not among the top N by `version_number`. The subquery correctly scopes to the same `file_id` and orders descending to keep the newest. No issue found.

---

## Summary

| # | Severity | Description |
|---|----------|-------------|
| 1 | HIGH | `restore_version` creates churn snapshots that evict meaningful history |
| 2 | MEDIUM | `export_version` lacks path traversal validation |
| 3 | MEDIUM | `transfer_files` destination path not sanitized against traversal |
| 4 | MEDIUM | No file size limit on blob storage in version snapshots |
| 5 | LOW | `convert_file_inner` double-locks and double-queries filepath |
| 6 | LOW | `transfer_files` triple lock acquisition, non-atomic |

**6 findings total.** Review does not pass.
