Issue resolved. No findings.

## Verification Details

### Issue #2: batch_rename can silently overwrite files causing permanent data loss

All three problems described in the issue are fully resolved, and all eight findings from the first-round code review have been addressed.

#### Issue Requirement 1: Silent file overwrite when two files produce the same target filename
**Resolved.** The `dedup_path` function (lines 75-107) uses a `HashSet<PathBuf>` to track all claimed paths within a batch and appends `_1`, `_2`, etc. suffixes when collisions are detected. It checks both filesystem existence and the in-batch claimed set. Applied in `batch_rename` (line 165), `batch_organize` (line 320), and `batch_export_usb` (line 437).

#### Issue Requirement 2: No collision detection before renaming
**Resolved.** Each batch function initializes a `claimed` HashSet before the loop and passes it to `dedup_path` for every file. The dedup check runs before any filesystem rename occurs.

#### Issue Requirement 3: No atomicity between filesystem rename and DB update
**Resolved.** Both `batch_rename` (lines 181-191) and `batch_organize` (lines 337-347) now:
- Track whether a filesystem rename occurred (`did_rename` flag)
- Catch DB update errors with `if let Err(db_err) = conn.execute(...)`
- Roll back the filesystem rename on DB failure via `std::fs::rename(&new_path, old_path)`
- Remove the rolled-back path from the `claimed` set so it remains available for subsequent files

#### Issue Requirement 4: batch_organize has same fixes
**Resolved.** `batch_organize` has identical collision detection (line 320) and rollback logic (lines 337-347).

#### First-Round Code Review Findings — All Addressed

| # | Finding | Status |
|---|---------|--------|
| 1 | `dedup_path` counter overflow | Fixed: bounded loop `1..=100_000u32` |
| 2 | Non-canonical path comparison | Fixed: `canonicalize()` applied to `old_path` before comparison |
| 3 | `HashMap<PathBuf, ()>` -> `HashSet<PathBuf>` | Fixed: uses `HashSet` throughout |
| 4 | `create_dir_all` before path validation | Fixed: validation via `normalized.starts_with()` now precedes `create_dir_all` |
| 5 | `base_dir.canonicalize()` silent fallback | Fixed: returns `AppError::Validation` on failure |
| 6 | Rolled-back paths remain in `claimed` | Fixed: `claimed.remove(&new_path)` in rollback path |
| 7 | `to_string_lossy` for non-UTF-8 paths | Informational — existing codebase pattern, not introduced by this change |
| 8 | Test depends on fixed filesystem path | Fixed: uses `tempfile::tempdir()` |

#### Test Coverage
Four new unit tests cover the `dedup_path` function: no-collision, batch-collision with suffix progression, no-extension handling, and existing-file-on-disk detection.
