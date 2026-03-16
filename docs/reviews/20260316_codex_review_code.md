# Codex Code Review — 2026-03-16

**Reviewer:** Codex CLI reviewer 1
**Scope:** files.rs, projects.rs, backup.rs, main.ts, Toolbar.ts, DocumentViewer.ts
**Issues:** #85–#90

## Review Summary

All six files were read and analyzed for correctness, security, consistency, and adherence to project conventions.

## Findings

**None.**

All reviewed code is clean:

- **files.rs** — Query building with parameterized SQL is correct. FTS5 input sanitization strips all special characters. LIKE queries use proper escaping. Soft-delete exclusion (`deleted_at IS NULL`) is consistently applied. Pagination logic is sound. Thumbnail batch generation correctly persists paths after generation.

- **projects.rs** — CRUD operations follow established patterns. Status validation uses a whitelist (`VALID_STATUSES`). Empty name trimming is handled. Dynamic UPDATE building with parameterized queries is correct. `duplicate_project` copies both the project and its details. Collection operations validate existence before insert. All tests exercise the expected DB constraints including ON CONFLICT upsert and cascade deletes.

- **backup.rs** — `create_backup` safely uses `VACUUM INTO` for DB copy. ZIP entry names for files use ID prefixes to prevent collisions. `restore_backup` validates manifest presence, creates a safety backup of the current DB, and validates ZIP entry names against path traversal (`..`, leading `/` or `\`). `relink_file` and `relink_batch` call `validate_no_traversal`. `import_metadata_json` merges by `unique_id` using COALESCE to preserve existing values. `import_library` deduplicates by `unique_id`. Batch archive/unarchive correctly check status guards. Auto-purge uses configurable retention days from settings.

- **main.ts** — Event handler registration and teardown is thorough (HMR-safe). Reload generation counter prevents stale results. Soft-delete is used instead of hard delete with appropriate user feedback. Drag-and-drop handler validates folder selection before import. All async operations have proper error handling with user-facing toast messages. The `initGeneration` guard prevents race conditions during HMR.

- **Toolbar.ts** — Menu groups are well-organized. ARIA attributes (`role="menu"`, `role="menuitem"`, `aria-haspopup`, `aria-expanded`) are correctly applied. Outside-click handler uses `requestAnimationFrame` to avoid immediate close. `updateItemStates` correctly handles single-select vs. multi-select states. Cleanup in `destroy()` delegates to `closeMenu()` then `super.destroy()`.

- **DocumentViewer.ts** — PDF.js worker is configured via `import.meta.url`. Singleton pattern with proper cleanup. Render task cancellation prevents race conditions. Keyboard shortcuts correctly skip input/textarea elements. Zoom clamping (0.25–5.0) is consistent across wheel, button, and keyboard zoom. Pan handlers are stored and removed during cleanup. Overview mode batches rendering (6 at a time) with event loop yields. All event listeners registered in `buildUI` are removed in `close()`.

## Verdict

**PASS** — Zero findings.
