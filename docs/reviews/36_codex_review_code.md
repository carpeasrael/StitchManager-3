# Codex Code Review — Sprint 17

**Reviewer:** Codex CLI
**Date:** 2026-03-14
**Scope:** Sprint 17 changes only (#67, #69, #70, #71, #72, #73)

---

## Summary

All six Sprint 17 changes have been reviewed against correctness, consistency, and adherence to project conventions. No blocking findings were identified. The implementation quality is solid across all items.

---

## Per-Issue Review

### #67 — SettingsDialog.ts: background image cleanup on cancel

**Files:** `src/components/SettingsDialog.ts`

**Review:**

- The `close(saved = false)` method correctly reverts CSS custom properties (`--bg-image`, `--bg-opacity`, `--bg-blur`) to their original values when `saved` is false.
- When `bgPathModified` is true on cancel, the code calls `SettingsService.removeBackgroundImage()` to delete the orphaned copied file, then restores the original `bg_image_path` setting. This properly prevents orphaned files in the app data directory.
- The `pendingBgRemove` flag correctly defers actual file deletion to save-time, allowing cancel to revert cleanly.
- The `bgPathModified` flag is reset to `false` on successful save (lines 173 and 182), preventing the cancel-revert logic from firing after a committed save.
- Both `.catch(() => {})` calls on the fire-and-forget cleanup operations are acceptable since the dialog is closing and there is no user-actionable recovery path.
- The original values (`originalBgImage`, `originalBgOpacity`, `originalBgBlur`, `originalBgImagePath`) are captured correctly in `show()` before any mutations.

**Findings:** None.

---

### #69 — components.css: prefers-reduced-motion

**Files:** `src/styles/components.css`

**Review:**

- The `@media (prefers-reduced-motion: reduce)` block at the end of the file applies to all elements via the universal selector `*, *::before, *::after`.
- `animation-duration: 0.01ms !important` and `transition-duration: 0.01ms !important` effectively disable animations/transitions while still allowing `animationend`/`transitionend` events to fire (which is the recommended approach over `0s`).
- `animation-iteration-count: 1 !important` prevents infinite animations from looping.
- This satisfies WCAG 2.1 SC 2.3.3 (Animation from Interactions) requirements.
- Placement at the end of the file is correct to ensure it overrides all prior animation/transition rules.

**Findings:** None.

---

### #70 — MetadataPanel.ts: JSON.stringify tag comparison

**Files:** `src/components/MetadataPanel.ts`

**Review:**

- `checkDirty()` (line 127) uses `JSON.stringify(current.tags) !== JSON.stringify(this.snapshot.tags)` for tag array comparison.
- Both sides are sorted arrays of strings (tags are sorted in `getCurrentFormValues()` at line 162 and `takeSnapshot()` at line 112), so `JSON.stringify` comparison is deterministic and correct.
- The same pattern is used in `save()` (line 700) to detect whether tags actually changed before calling `setTags()`.
- This replaces what was presumably a reference equality or shallow comparison, which would have always reported dirty state for tags. The fix is correct.

**Findings:** None.

---

### #71 — Sidebar.ts: alert -> toast

**Files:** `src/components/Sidebar.ts`

**Review:**

- `ToastContainer` is properly imported at line 4.
- `loadFolders()` (line 28) shows an error toast instead of using `alert()`.
- `createFolder()` (line 172) shows an error toast on failure with the error detail appended.
- No remaining `alert()` calls exist anywhere in `src/` (confirmed via search).
- This is consistent with the project convention of using `ToastContainer.show()` for all user notifications.

**Findings:** None.

---

### #72 — convert.rs: single lock acquisition

**Files:** `src-tauri/src/commands/convert.rs`

**Review:**

- `convert_file_inner()` now performs both the version snapshot and the filepath query within a single `lock_db()` scope (lines 69-83).
- The lock is released at the closing brace of the block before file I/O operations (parsing, writing), which is correct — file I/O should not hold the DB lock.
- `create_version_snapshot()` receives a `&Connection` rather than acquiring its own lock, so there is no risk of deadlock from nested lock acquisitions.
- The `let _ =` on the version snapshot call (line 72) intentionally ignores errors, which is consistent with the non-fatal versioning policy documented in `versions.rs` (line 54: "Non-fatal: skip versioning if file can't be read").
- In `convert_files_batch()`, each iteration calls `convert_file_inner()` which acquires and releases the lock per file. This is acceptable — holding a single lock across all files would block the DB for the duration of potentially many file I/O operations.
- The duplicate filepath query (once in `create_version_snapshot` and once in `convert_file_inner`) is a minor redundancy but acceptable given the separation of concerns. The version snapshot needs the filepath independently to read the file data for storage.

**Findings:** None.

---

### #73 — Toolbar.ts: scan error toast + ToastContainer import

**Files:** `src/components/Toolbar.ts`

**Review:**

- `ToastContainer` is properly imported at line 4.
- `scanFolder()` (line 380) shows an error toast on scan failure.
- The `addFolder()` method (line 348) does not show a toast on failure — it only logs via `console.warn`. This is pre-existing behavior outside Sprint 17 scope.
- The `ToastContainer` import is correctly placed with other component imports at the top of the file.

**Findings:** None.

---

## Cross-Cutting Observations

1. **No remaining `alert()` calls** — Confirmed via codebase-wide search. The project consistently uses `ToastContainer.show()` for notifications.
2. **CSS accessibility** — The `prefers-reduced-motion` media query is the only motion-related accessibility measure needed given the project's minimal animation footprint.
3. **Lock discipline** — The single-lock pattern in `convert.rs` is consistent with how other commands in the codebase handle DB access. The lock is held for the minimum necessary duration.
4. **Tag comparison** — Both dirty-check and save-path use the same `JSON.stringify` pattern on pre-sorted arrays, ensuring consistency.

---

## Verdict

**Zero findings.** All Sprint 17 changes are correctly implemented, consistent with project conventions, and ready to ship.
