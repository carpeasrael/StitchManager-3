# Sprint 17 Code Review

**Reviewer:** Claude Opus 4.6 (1M context)
**Date:** 2026-03-14
**Scope:** Issues #67--#73 (7 low-priority fixes)
**Result:** Zero findings

---

## Review Summary

All seven changes were reviewed for correctness, consistency with project conventions, potential regressions, and edge-case safety. No issues found.

---

## Per-Issue Review

### #67 -- SettingsDialog.ts: close() cleanup for background image

**Files:** `src/components/SettingsDialog.ts`

The close() method (line 866) now handles two cancel scenarios:

1. **User selected a new image then cancelled:** `bgPathModified` is true, so `removeBackgroundImage()` is called to delete the orphaned copy, then the original `bg_image_path` is restored in the DB via `setSetting()`.
2. **User clicked "Bild entfernen" then cancelled:** `pendingBgRemove` is true but `bgPathModified` is false, so no file removal occurs and the CSS custom properties are simply reverted to originals.

The save path correctly clears `bgPathModified` after committing (line 182), and when `pendingBgRemove` is true on save, the actual `removeBackgroundImage()` call is executed (lines 170-178).

State management is sound: `pendingBgRemove` and `bgPathModified` are reset on dialog open (lines 57-58), toggled appropriately on user actions (lines 383-384 for image select, line 409 for remove), and the `close(saved)` parameter gates revert logic. The `.catch(() => {})` on fire-and-forget calls in the cancel path (lines 879, 881) is acceptable since these are best-effort cleanup operations and errors are non-critical.

No issues.

### #68 -- icon.ico: Regenerated with standard sizes

**Files:** `src-tauri/icons/icon.ico`

Binary asset -- cannot code-review content, but the file exists at the expected path. The sizes listed (16x16, 32x32, 64x64, 128x128, 256x256) are the standard Windows ICO sizes needed for taskbar, title bar, desktop, and high-DPI display.

No issues.

### #69 -- components.css: @media (prefers-reduced-motion: reduce)

**Files:** `src/styles/components.css` (lines 2573-2580)

The rule uses `animation-duration: 0.01ms !important`, `animation-iteration-count: 1 !important`, and `transition-duration: 0.01ms !important` on `*, *::before, *::after`. This is the standard WCAG-compliant reduced-motion pattern. Using 0.01ms instead of 0s avoids edge cases where some browsers treat 0s as "no animation was set" and fall back to default behavior. Placed at the end of the file so it overrides all prior animation/transition declarations. The `!important` is necessary and appropriate here.

No issues.

### #70 -- MetadataPanel.ts: JSON.stringify() for tag comparison

**Files:** `src/components/MetadataPanel.ts`

In `checkDirty()` (line 127):
```typescript
JSON.stringify(current.tags) !== JSON.stringify(this.snapshot.tags)
```

In `save()` (line 700):
```typescript
JSON.stringify(values.tags) !== JSON.stringify(this.snapshot.tags)
```

Both `current.tags` and `this.snapshot.tags` are sorted string arrays (sorted in `getCurrentFormValues()` at line 162 and `takeSnapshot()` at line 112), so `JSON.stringify()` produces a deterministic, order-stable comparison. This is correct and replaces the previous `tags.join(",")` approach which would have failed on tags containing commas (e.g., a tag named `"rot, blau"` would be indistinguishable from two separate tags `"rot"` and `"blau"`).

No issues.

### #71 -- Sidebar.ts: ToastContainer.show() replaces alert()

**Files:** `src/components/Sidebar.ts`

In `createFolder()` (line 173):
```typescript
ToastContainer.show("error", `Ordner konnte nicht erstellt werden: ${e}`);
```

The `ToastContainer` import is present at line 4. The error message includes the exception detail via template literal, which is consistent with how other components report errors (e.g., Toolbar.ts line 380). The `alert()` call was the only one remaining in Sidebar, so this aligns the component with the project-wide convention of using toast notifications instead of blocking browser dialogs.

No issues.

### #72 -- convert.rs: Merged two lock acquisitions into single scope

**Files:** `src-tauri/src/commands/convert.rs`

The `convert_file_inner()` function (lines 62-115) now performs both the version snapshot creation and the filepath query in a single `lock_db()` scope (lines 69-83). The lock is acquired once, `create_version_snapshot()` is called, then the filepath is queried, and the lock is released when the block exits. This eliminates a previously unnecessary second lock acquisition, reducing contention on the Mutex-wrapped DB connection.

The `let _ =` on `create_version_snapshot()` (line 72) intentionally ignores errors -- version snapshots are a best-effort operation and should not block conversion. This is consistent with the same pattern used in other commands.

The error mapping (lines 77-82) correctly distinguishes `QueryReturnedNoRows` from other SQLite errors, returning `NotFound` for missing files and `Database` for unexpected errors.

No issues.

### #73 -- Toolbar.ts: Error toast in scanFolder() catch block

**Files:** `src/components/Toolbar.ts`

In `scanFolder()` (line 380):
```typescript
ToastContainer.show("error", "Ordner konnte nicht gescannt werden");
```

The `ToastContainer` import is present at line 4. Previously the catch block only had `console.warn()`, meaning scan failures were silently swallowed from the user's perspective. The error toast now informs the user. The message is in German, consistent with the UI language convention.

No issues.

---

## Cross-Cutting Checks

- **Import consistency:** All files that use `ToastContainer` have the import. No unused imports introduced.
- **Convention adherence:** German UI strings, English code, error logging via `console.warn()` before user-facing toast.
- **No regressions:** No changes to public interfaces, state management, or event contracts.
- **Type safety:** All changes are type-compatible with existing signatures.
- **CSS specificity:** The reduced-motion media query uses `!important` which is the standard pattern and is placed last in the file, avoiding unintended cascade issues.

---

## Verdict

All seven Sprint 17 changes are correct, well-scoped, and consistent with project conventions. Zero findings.
