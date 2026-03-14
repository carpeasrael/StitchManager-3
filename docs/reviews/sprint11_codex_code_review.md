# Sprint 11 Codex Code Review — Version History + Machine Transfer

**Reviewer:** Codex CLI (code review)
**Date:** 2026-03-14
**Files reviewed:**
- `src-tauri/src/commands/versions.rs`
- `src-tauri/src/commands/transfer.rs`
- `src-tauri/src/commands/edit.rs` (auto-versioning hook)
- `src-tauri/src/commands/convert.rs` (auto-versioning hook)
- `src-tauri/src/db/migrations.rs` (apply_v8)

---

## Findings

### F1 — `export_version`: no path sanitization on user-supplied `path` (versions.rs:152-170)

**Severity:** High
**File:** `src-tauri/src/commands/versions.rs`, line 156

The `export_version` command accepts an arbitrary `path: String` from the frontend and writes blob data directly to it via `std::fs::write(&path, &file_data)`. There is no validation that the path is within a safe directory, no check for path traversal (`..`), and no check that it does not overwrite a critical file. The batch module in `batch.rs` applies `sanitize_path_component` / `sanitize_pattern_output` on user-supplied path fragments; `export_version` should apply equivalent guards or at minimum validate that the target directory exists and the path does not escape an expected scope.

**Recommendation:** Validate the export path: ensure the parent directory exists, the path does not contain `..` components, and optionally refuse to overwrite an existing file (similar to the overwrite guard in `edit.rs:save_transformed`).

---

### F2 — `transfer_files`: no path sanitization on `transfer_path` from DB (transfer.rs:115)

**Severity:** Medium
**File:** `src-tauri/src/commands/transfer.rs`, lines 115-118

The `transfer_path` is read from the database and used directly as a destination for `std::fs::copy` and `convert_and_copy`. While the value originates from `add_machine` (which also takes unsanitized user input), no traversal or validity check is performed on the stored path beyond `exists() && is_dir()`. An attacker who can write to the database (or a user who enters a malicious path) could cause files to be written to unexpected locations.

**Recommendation:** Sanitize `transfer_path` in `add_machine` at insertion time (reject paths containing `..` or other suspicious components). The `exists() && is_dir()` check in `transfer_files` is necessary but not sufficient.

---

### F3 — `transfer_files`: filename collision / overwrite without warning (transfer.rs:166-170)

**Severity:** Medium
**File:** `src-tauri/src/commands/transfer.rs`, lines 156, 168

When copying files to the transfer destination, if a file with the same name already exists at the destination, `std::fs::copy` silently overwrites it. The `save_transformed` command in `edit.rs` explicitly prevents this with an existence check. Transfer should either warn, skip, or rename to avoid silent data loss on the target device.

**Recommendation:** Check `dest_file.exists()` before writing and either return an error, auto-rename (e.g., append `_1`), or include a `force_overwrite` parameter.

---

### F4 — `create_version_snapshot`: TOCTOU race between reading file and inserting version (versions.rs:30-63)

**Severity:** Low
**File:** `src-tauri/src/commands/versions.rs`, lines 30-63

The function reads the filepath from the DB, then reads the file from disk, then inserts the version. Because the DB mutex is held by the caller but filesystem access is not synchronized, the file on disk could change between the DB read and the `std::fs::read`. In a desktop app this is unlikely to be exploited, but it is a correctness concern for concurrent file-watcher modifications.

**Recommendation:** Acceptable for current scope. Document the limitation or consider reading the file while still inside the same lock scope (which is already the case for the DB lock, so the risk is only from external filesystem modifications).

---

### F5 — `add_machine` returns empty `created_at` string (transfer.rs:63)

**Severity:** Low
**File:** `src-tauri/src/commands/transfer.rs`, line 63

After inserting a machine profile, the returned `MachineProfile` has `created_at: String::new()` rather than the actual timestamp set by the database default. The frontend will receive an empty string for `created_at` on the newly created profile. Other similar patterns in the codebase re-query the row after insert.

**Recommendation:** Re-query the inserted row (`SELECT ... WHERE id = ?1`) to return the actual `created_at` value, or use `datetime('now')` in Rust to match the DB default.

---

### F6 — `transfer_files` silently skips missing file IDs (transfer.rs:131)

**Severity:** Low
**File:** `src-tauri/src/commands/transfer.rs`, line 131

When loading file paths, if a `file_id` does not exist in the database, it is silently skipped (`Err(_) => continue`). The `total` field in the result reflects only found files, not the originally requested count. This can mask bugs where the frontend sends stale IDs.

**Recommendation:** Either include skipped IDs in the error list or set `total` to `file_ids.len()` (it currently is, which is correct) and increment `failed` for missing DB entries so the caller knows some files were not found.

---

### F7 — Version pruning query correctness (versions.rs:66-70)

**Severity:** Low
**File:** `src-tauri/src/commands/versions.rs`, lines 66-70

The pruning DELETE uses a subquery: `DELETE FROM file_versions WHERE file_id = ?1 AND id NOT IN (SELECT id FROM file_versions WHERE file_id = ?1 ORDER BY version_number DESC LIMIT ?2)`. This is correct in SQLite (subquery with LIMIT in NOT IN works), but note that if two versions have the same `version_number` (which is possible since there is no UNIQUE constraint on `(file_id, version_number)`), the pruning behavior becomes non-deterministic.

**Recommendation:** Add a UNIQUE constraint on `(file_id, version_number)` in the `file_versions` table, or change the pruning to use `ORDER BY id DESC` which is guaranteed unique.

---

### F8 — `restore_version` ignores snapshot failure (versions.rs:124)

**Severity:** Low
**File:** `src-tauri/src/commands/versions.rs`, line 124

Before restoring, a snapshot of the current state is created with `let _ = create_version_snapshot(...)`. If the snapshot fails (e.g., disk full), the restore proceeds anyway and the user loses the ability to undo. While the comment indicates this is intentional, it should at minimum log the failure.

**Recommendation:** Log the error if the pre-restore snapshot fails: `if let Err(e) = create_version_snapshot(...) { log::warn!("..."); }`.

---

## Summary

8 findings total:
- 1 High (F1 — export path unsanitized)
- 2 Medium (F2 — transfer path sanitization, F3 — overwrite on transfer)
- 5 Low (F4-F8 — race condition, empty timestamp, silent skip, pruning edge case, ignored error)

The high-severity finding (F1) and the medium findings (F2, F3) should be addressed before merging. The low-severity findings are acceptable to defer but recommended to fix.
