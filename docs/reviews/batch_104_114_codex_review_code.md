# Code Review — Issues #104-#114 (Codex)

## Scope

Reviewed uncommitted diff and committed changes across 6 commits (b3aa34b..b056aa8) for issues #104, #105/#111, #106, #108, #110, #114.

## Findings

No findings.

Detailed verification per change:

1. **capabilities/default.json (#104):** `sql:default` removed. Generated schema updated accordingly. No frontend code uses `@tauri-apps/plugin-sql` imports or Database class. The backend still registers the plugin for Rust-side use, which is intentional.

2. **tauri.conf.json (#105/#111):** CSP string extended with `form-action 'self'; frame-ancestors 'none'`. Proper semicolon separation. No syntax errors. These directives correctly prevent form submission to external origins and prevent the app from being embedded in iframes.

3. **ManufacturingDialog.ts & ProjectListDialog.ts (#106):**
   - Both import `trapFocus` from `../utils/focus-trap`.
   - Both declare `private releaseFocusTrap: (() => void) | null = null`.
   - Both call `trapFocus(dialog)` in `init()` after DOM insertion, with fallback to overlay if `.dialog` not found.
   - Both release the trap in `close()` before removing the overlay and null out the reference.
   - The `focus-trap.ts` utility correctly handles Tab cycling, initial focus, and restores previous focus on release.

4. **MetadataPanel.ts (#108):**
   - Guard placed correctly after the early-return for same-file check but before file loading.
   - Condition `this.dirty && this.currentFile && fileId !== this.currentFile.id` is precise.
   - On cancel, reverts selection to `this.currentFile.id`. The re-entrant call is caught by the `fileId === this.currentFile?.id` early return, preventing infinite recursion.
   - German-language confirm text is consistent with the app's UI language.

5. **files.rs (#110):**
   - SEC-002 block uses `dirs::data_dir()` (crate already in dependencies) with `XDG_DATA_HOME` env var fallback.
   - Canonicalizes both paths. Uses string containment check for app identifier.
   - Warn-only, non-blocking -- appropriate for logging without disrupting user workflow.
   - The `_canonical_app` variable is computed but unused; the code checks string containment instead of prefix comparison. This is intentional (comment explains files may come from user dialog choices outside app data dir).

6. **FileList.ts & main.ts (#114):**
   - `EventBus.on("filelist:scroll-to-index", ...)` subscription wrapped in `this.subscribe()` for lifecycle cleanup.
   - `scrollToIndex` uses `CARD_HEIGHT` constant consistently with the rest of the virtual scroll implementation.
   - Scrolls only when item is outside visible viewport (both above and below checks).
   - `navigateFile()` in `main.ts` emits the event after setting the new selection, which is the correct order.

Build validation: `npm run build` succeeds, `cargo check` succeeds, `cargo test` passes (0 failures).
