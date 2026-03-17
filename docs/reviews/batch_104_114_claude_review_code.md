# Code Review — Issues #104-#114 (Claude)

## Scope

Reviewed the combined diff of commits b3aa34b through b056aa8 (6 commits), covering issues #104, #105/#111, #106, #108, #110, #114.

## Findings

No findings.

All changes are well-structured, consistent with project conventions, and correctly implemented:

- **#104 (sql:default removal):** `sql:default` cleanly removed from `capabilities/default.json` and the generated schema file. No frontend code imports or calls `@tauri-apps/plugin-sql` directly (the only reference is in informational text within `app-texts.ts`). The `tauri-plugin-sql` crate remains in `Cargo.toml` for backend use, which is correct.

- **#105/#111 (CSP directives):** `form-action 'self'; frame-ancestors 'none'` appended to the CSP string in `tauri.conf.json`. Syntax is correct and follows the existing semicolon-delimited pattern.

- **#106 (focus traps):** Both `ManufacturingDialog` and `ProjectListDialog` correctly import `trapFocus` from `../utils/focus-trap`, store the release function in a private field (`releaseFocusTrap`), call `trapFocus()` after DOM insertion in `init()`, and release the trap in `close()` before removing the overlay. The `trapFocus` utility itself is well-implemented with proper Tab/Shift+Tab cycling, initial focus, and focus restoration on cleanup.

- **#108 (unsaved-changes guard):** The guard in `MetadataPanel.onSelectionChanged` correctly checks `this.dirty && this.currentFile && fileId !== this.currentFile.id` before prompting. On cancel, it reverts the selection via `appState.set("selectedFileId", this.currentFile.id)`, which re-triggers `onSelectionChanged` but is caught by the early-return on line 80 (`fileId === this.currentFile?.id`), preventing an infinite loop.

- **#110 (path validation warning):** The `open_attachment` function logs a warning when the canonical path does not contain the app identifier strings. Uses `dirs::data_dir()` (already a dependency) with `XDG_DATA_HOME` fallback. The check is warn-only (not blocking), which is appropriate for a first-pass security measure.

- **#114 (scroll-to-index):** `FileList` subscribes to `filelist:scroll-to-index` via `EventBus.on()` wrapped in `this.subscribe()` for proper cleanup on destroy. The `scrollToIndex` method correctly calculates visibility bounds using `CARD_HEIGHT` and only scrolls if the target item is outside the visible viewport. `main.ts` emits the event in `navigateFile()` after setting the selection.

All changes compile (`cargo check`, `npm run build`) and tests pass (`cargo test`).
