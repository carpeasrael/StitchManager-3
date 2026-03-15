# TC03 — Batch Operations

## TC03-01: Batch rename — successful operation
- **Precondition:** Multiple files selected, rename pattern set
- **Steps:** Select files → Batch rename with pattern `{theme}_{name}`
- **Expected:** All files renamed on disk and in DB
- **Status:** PASS (covered by unit tests)

## TC03-02: Batch rename — DB failure after filesystem rename
- **Precondition:** Multiple files selected
- **Steps:** Trigger batch rename where DB transaction fails after filesystem renames
- **Expected:** Filesystem renames rolled back, user informed of failure
- **Severity:** CRITICAL — rollback failures silently ignored, files orphaned (see BE-C3)
- **Status:** FAIL — silent data loss on rollback failure

## TC03-03: Batch rename — partial failure user feedback
- **Precondition:** Multiple files selected, some will fail to rename
- **Steps:** Execute batch rename where some files fail
- **Expected:** User shown success/failure count via toast
- **Severity:** MAJOR — errors only logged to console (see FE-M3)
- **Status:** FAIL — no user-visible feedback for partial failures

## TC03-04: Batch organize — path traversal in pattern
- **Precondition:** Files selected, organize pattern set
- **Steps:** Use organize pattern containing `../../../etc`
- **Expected:** Path traversal blocked
- **Severity:** MAJOR — path check compares unresolved vs canonical paths (see BE-M4)
- **Status:** FAIL — symlink-based traversal possible

## TC03-05: Batch organize — correct folder structure
- **Precondition:** Files with theme metadata
- **Steps:** Organize with pattern `{format}/{theme}`
- **Expected:** Files moved to correct subdirectories
- **Status:** PASS (basic case works)

## TC03-06: Batch export USB — path validation
- **Precondition:** USB device detected, files selected
- **Steps:** Export files to USB target path
- **Expected:** Target path validated against traversal
- **Severity:** MINOR — no path traversal check on target_path (see BE-m10)
- **Status:** FAIL — arbitrary write location possible

## TC03-07: Batch dialog auto-close after completion
- **Precondition:** Batch operation in progress
- **Steps:** Wait for operation to complete
- **Expected:** Dialog auto-closes after brief delay showing 100%
- **Status:** PASS (progress tracking and auto-close work correctly)

## TC03-08: Batch dialog manual close during operation
- **Precondition:** Batch operation in progress
- **Steps:** Click close button before operation completes
- **Expected:** Dialog closes cleanly, no crash from late progress events
- **Status:** PASS (null guards prevent crash)
