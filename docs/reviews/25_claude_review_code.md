# Code Review — Sprint 5 (Issues #27, #34, #30) — Round 4

Reviewer: Claude Opus 4.6
Date: 2026-03-13
Scope: All 17 modified and new files in sprint 5

## Verification of Round 3 Fixes

All 8 findings from Round 3 have been properly addressed:

1. **Background image transactional flow** -- FIXED. `SettingsDialog.ts` now tracks `bgPathModified` and `pendingBgRemove` flags. Image removal is deferred to save via `pendingBgRemove`. On cancel, `close(saved=false)` reverts CSS properties and restores the original `bg_image_path` DB setting when `bgPathModified` is true (line 877-878).

2. **`remove_background_image` DB mutex during FS I/O** -- FIXED. The path is now read in a scoped block that drops the lock (lines 231-239), filesystem deletion happens outside the lock, then the lock is re-acquired for the DB update (lines 246-250).

3. **`addAttachment` hardcoded "license" type** -- FIXED. Now uses `"other"` (line 1105).

4. **StatusBar missing "folders" subscription** -- FIXED. Line 14 subscribes to `appState.on("folders", ...)`.

5. **`applyFontSize` duplication** -- FIXED. Extracted to `src/utils/theme.ts` and imported by both `main.ts` (line 29) and `SettingsDialog.ts` (line 9).

6. **`searchParams` replacement instead of merge** -- FIXED. Line 1010 uses `appState.update("searchParams", (sp) => ({ ...sp, colorSearch: match.hex }))`.

7. **USB monitor duplicate polling** -- Accepted as low-impact with frontend deduplication in place. No change needed.

8. **German typo "Unscharfe"** -- FIXED. Now reads "Unschaerfe (px)" (line 456).

## New Review

After thorough review of all 17 files for code quality, correctness, security, error handling, performance, edge cases, and type/memory safety:

Code review passed. No findings.
