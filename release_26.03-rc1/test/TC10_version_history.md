# TC10 — Version History

## TC10-01: Version snapshot created on convert
- **Precondition:** File exists, conversion triggered
- **Steps:** Convert file to different format
- **Expected:** Pre-conversion snapshot stored in `file_versions` table
- **Status:** PASS (snapshot creation works)

## TC10-02: Restore version — pre-restore snapshot
- **Precondition:** File with version history, no prior restore
- **Steps:** Restore to previous version
- **Expected:** Current state snapshotted before restore, then version applied
- **Status:** PASS (first restore creates snapshot)

## TC10-03: Restore version — repeated restore
- **Precondition:** File has been restored once before
- **Steps:** Restore to a different version
- **Expected:** Current state snapshotted again before new restore
- **Severity:** CRITICAL — SQL bug causes pre-restore snapshot to be skipped forever after first restore (see BE-C1)
- **Status:** FAIL — safety snapshot never created again after first restore

## TC10-04: Version history UI access
- **Precondition:** File with versions
- **Steps:** Attempt to view, restore, or manage versions from UI
- **Expected:** UI controls for version management
- **Severity:** MEDIUM — no UI exposes version history features (see INT-6.5)
- **Status:** FAIL — feature exists in backend but unreachable from UI

## TC10-05: Export version — path traversal
- **Precondition:** Version exists
- **Steps:** Export version with path containing `..`
- **Expected:** Path traversal rejected
- **Severity:** MINOR — string-level check only (see BE-m6)
- **Status:** PASS (basic check exists, limited bypass risk)
