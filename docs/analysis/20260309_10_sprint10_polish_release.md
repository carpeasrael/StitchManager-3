# Sprint 10: Polish & Release — Analysis

## Problem Description

StitchManager has all core features implemented (Sprints 1–9): file import, parsing (PES/DST/JEF/VP3), metadata editing, AI analysis, batch operations, and multi-select. Sprint 10 delivers production readiness: completing the SettingsDialog with all 5 tabs, global keyboard shortcuts, filesystem watching for auto-detection of new files, draggable splitter handles for the 3-panel layout, virtual scrolling for large file lists, toast notifications for user feedback, production bundle configuration, and final QA.

## Affected Components

### Backend (Rust / Tauri)
- `src-tauri/src/services/file_watcher.rs` (new) — filesystem watcher using `notify` crate (already in Cargo.toml)
- `src-tauri/src/services/mod.rs` — register watcher module
- `src-tauri/src/lib.rs` — start watcher in setup, register watcher commands
- `src-tauri/tauri.conf.json` — bundle configuration (identifier, version, icons)

### Frontend (TypeScript)
- `src/components/SettingsDialog.ts` — add 3 new tabs: Allgemein, Erscheinungsbild, Benutzerdefiniert
- `src/components/Toast.ts` (new) — toast notification component
- `src/components/Splitter.ts` (new) — draggable panel splitter
- `src/components/FileList.ts` — virtual scrolling implementation
- `src/shortcuts.ts` (new) — keyboard shortcut handler module
- `src/main.ts` — wire shortcuts, toasts, watcher events
- `src/types/index.ts` — Toast type, extend State
- `src/state/AppState.ts` — add toasts array
- `src/styles/components.css` — toast styles, splitter styles
- `src/styles/layout.css` — splitter integration in grid layout

## Root Cause / Rationale

The app needs UX polish and production configuration to be release-ready. Users expect: keyboard shortcuts for efficient workflows, toast notifications for action feedback, resizable panels, smooth scrolling with large collections, automatic detection of externally added files, and a complete settings UI. The bundle must be properly configured for distribution.

## Proposed Approach

### Execution Order
1. S10-T6 (Toast — foundational for user feedback in other tickets)
2. S10-T1 (SettingsDialog — depends on toast for save confirmation)
3. S10-T2 (Keyboard Shortcuts)
4. S10-T4 (Splitter Handles)
5. S10-T5 (Virtual Scrolling)
6. S10-T3 (Filesystem Watcher)
7. S10-T7 (Bundle Configuration)
8. S10-T8 (QA — last, verifies everything)

### S10-T6: Toast-Benachrichtigungen

1. Add types to `src/types/index.ts`:
   ```typescript
   type ToastLevel = "success" | "error" | "info";
   interface Toast { id: string; level: ToastLevel; message: string; }
   ```
2. Add `toasts: Toast[]` to State interface and initialState.
3. Create `src/components/Toast.ts`:
   - `ToastContainer` class that renders a fixed-position container (top-right).
   - Subscribes to `appState` `toasts` changes and renders/removes toast elements.
   - Each toast auto-dismisses after 4 seconds.
   - CSS transitions: slide-in from right, fade-out on dismiss.
   - Static `Toast.show(level, message, duration?)` helper.
4. Add CSS styles in `src/styles/components.css`: `.toast-container`, `.toast`, `.toast-success`, `.toast-error`, `.toast-info`, slide/fade animations.
5. Initialize `ToastContainer` in `main.ts`.

### S10-T1: SettingsDialog komplett

Currently has 2 tabs (KI-Einstellungen, Dateiverwaltung). Need to add 3 more:

1. **Allgemein tab** — Already partially covered by Dateiverwaltung (library_root, metadata_root). Restructure: move library_root and metadata_root to Allgemein tab. Dateiverwaltung keeps rename/organize patterns only.
2. **Erscheinungsbild tab** — Theme toggle (hell/dunkel) bound to `theme` state + `ThemeMode`. Font size selector (small/medium/large) stored as setting `font_size`, applied via CSS custom property `--font-size-base`.
3. **Benutzerdefiniert tab** — List existing custom fields from `SettingsService.getCustomFields()`. Create form: name (text), type (select: text/number/select), options (text, comma-separated, shown only for type=select). Delete button per field with confirmation. Uses existing `custom_field_create`, `custom_field_delete` commands.
4. Reorder tabs: Allgemein, Erscheinungsbild, KI, Dateiverwaltung, Benutzerdefiniert.
5. Show toast on save success/failure.

### S10-T2: Keyboard-Shortcuts

1. Create `src/shortcuts.ts` module:
   - `initShortcuts()` function that adds a global `keydown` listener on `document`.
   - Guard: skip shortcuts when focus is in `<input>`, `<textarea>`, `<select>`.
   - Shortcut map:
     - `Cmd/Ctrl+S` → emit `shortcut:save` (save metadata)
     - `Cmd/Ctrl+F` → emit `shortcut:search` (focus search field)
     - `Cmd/Ctrl+,` → emit `shortcut:settings` (open settings)
     - `Delete/Backspace` → emit `shortcut:delete` (delete selected file with confirmation)
     - `ArrowUp` → emit `shortcut:prev-file`
     - `ArrowDown` → emit `shortcut:next-file`
     - `Escape` → emit `shortcut:escape` (close dialog / clear selection)
   - Each emits via `EventBus.emit()`.
   - `e.preventDefault()` for handled shortcuts to avoid browser conflicts.
2. Wire handlers in `main.ts`:
   - `shortcut:save` → trigger metadata save (same as save button click)
   - `shortcut:search` → focus the search input
   - `shortcut:settings` → open SettingsDialog
   - `shortcut:delete` → confirm dialog then delete file
   - `shortcut:prev-file` / `shortcut:next-file` → navigate file selection
   - `shortcut:escape` → close topmost dialog or clear selection
3. Call `initShortcuts()` from `main.ts` init.

### S10-T4: Splitter-Handles

1. Create `src/components/Splitter.ts`:
   - `Splitter` class: creates a thin (4px) vertical div with `col-resize` cursor.
   - On mousedown: begin tracking, add mousemove/mouseup listeners on document.
   - On mousemove: calculate delta, update CSS custom property (`--sidebar-width` or `--center-width`) on `document.documentElement`.
   - On mouseup: stop tracking, remove listeners.
   - Enforce min widths: sidebar min 180px max 400px, center min 300px max 800px.
2. Insert 2 splitter elements into the grid layout:
   - Between sidebar and center columns.
   - Between center and right columns.
3. Update `src/styles/layout.css`: add splitter columns to grid-template-columns, e.g. `var(--sidebar-width, 240px) 4px var(--center-width, 480px) 4px 1fr`.
4. Add `.splitter` CSS in `src/styles/components.css`: cursor, hover highlight, background.

### S10-T5: Virtual Scrolling für FileList

1. Modify `src/components/FileList.ts`:
   - Fixed card height: 72px (already consistent in CSS).
   - Track scroll position via `scroll` event on the file list container.
   - Calculate visible range: `startIndex = Math.floor(scrollTop / 72)`, `endIndex = startIndex + Math.ceil(containerHeight / 72) + buffer`.
   - Render only cards in `[startIndex - buffer, endIndex + buffer]` range (buffer = 5).
   - Use a spacer div with `height = totalFiles * 72px` to maintain scrollbar accuracy.
   - Cards positioned absolutely or via `paddingTop` on the container.
2. Selection must still work: click handlers reference file index correctly via `data-index` attribute.
3. On `files` state change: recalculate total height, re-render visible range.

### S10-T3: Dateisystem-Watcher

1. Create `src-tauri/src/services/file_watcher.rs`:
   - Use `notify` crate (v6, already in Cargo.toml) with `RecommendedWatcher`.
   - `start_watcher(library_root: String, app_handle: AppHandle)` function:
     - Create watcher watching `library_root` recursively.
     - On `Create` event for embroidery files (.pes/.dst/.jef/.vp3): emit `fs:new-files` Tauri event with file paths.
     - On `Remove` event: emit `fs:files-removed` with file paths.
     - Filter events by extension using existing `is_embroidery_file` logic.
     - Debounce: collect events over 500ms window before emitting (avoid rapid-fire on bulk copies).
   - `stop_watcher()` to clean up.
2. Register module in `src-tauri/src/services/mod.rs`.
3. In `src-tauri/src/lib.rs` setup closure: read `library_root` setting from DB, if set, start watcher.
4. Add Tauri commands: `watcher_start(path)`, `watcher_stop()` for manual control.
5. Frontend in `main.ts`: bridge `fs:new-files` and `fs:files-removed` events. On new files: show info toast, trigger re-scan. On removed files: show info toast, reload file list.

### S10-T7: Bundle-Konfiguration

1. Update `src-tauri/tauri.conf.json`:
   - `identifier`: `"de.carpeasrael.stichman"`
   - `version`: `"2.0.0"`
   - Verify icon paths exist (32x32, 128x128, 128x128@2x, .icns, .ico).
2. Verify `npm run tauri build` succeeds on macOS.
3. Check bundle output size.

### S10-T8: Abschluss-QA und Bugfixes

This is a verification ticket, not an implementation ticket. After all other S10 tickets are implemented:
1. Run `cargo test` — all tests must pass.
2. Run `npm run build` — TypeScript + Vite build must succeed.
3. Run `cargo check` — no warnings.
4. Manual checklist verification (documented in sprint plan).
5. Fix any bugs found during QA.

---

## Phase 4: Closure

**Commit:** `c9d058a` — Implement Sprint 10: Polish & Release

### Summary

All 8 Sprint 10 tickets implemented and verified:

- **S10-T1 (SettingsDialog):** 5-tab dialog (Allgemein, Erscheinungsbild, KI-Einstellungen, Dateiverwaltung, Benutzerdefiniert) with live theme/font preview, cancel revert, custom fields CRUD with date type, double-open guard, partial save keeps dialog open.
- **S10-T2 (Keyboard shortcuts):** Cmd+S (save, works in inputs), Cmd+F (search), Cmd+comma (settings), Delete/Backspace, ArrowUp/Down, Escape — input-focus guards, conditional preventDefault.
- **S10-T3 (Filesystem watcher):** `notify` v6 crate, recursive watch, Create/Modify/Remove events, 500ms debounce with HashSet dedup, embroidery file filter, auto-start from library_root setting, toast + auto-reload.
- **S10-T4 (Splitter handles):** Two draggable splitters controlling `--sidebar-width` (180–400px) and `--center-width` (300–800px) via CSS custom properties, proper mouse event cleanup.
- **S10-T5 (Virtual scrolling):** Fixed 72px card height, 5-card buffer, absolute positioning, rAF-throttled scroll handler, selection-only DOM updates via `updateSelection()`.
- **S10-T6 (Toast notifications):** Max 5 concurrent, 4s auto-dismiss, slide-in/fade-out animations, exit animation race prevention, success/error/info levels.
- **S10-T7 (Bundle config):** productName "StichMan", version "2.0.0", identifier "de.carpeasrael.stichman", icons configured.
- **S10-T8 (QA):** cargo check clean, 99/99 tests pass, npm run build clean.

### Review Rounds

- **Round 1:** Codex #1 (8 findings), Claude #1 (10 findings) → all fixed
- **Round 2:** Codex #1 (3 findings), Claude #1 (4 findings) → all fixed
- **Round 3:** Claude #1 (1 finding: double-open guard) → fixed
- **Round 4:** Claude #1 (2 valid findings: Cmd+S in inputs, Escape preventDefault) → fixed
- **Round 5:** All 4 agents: ZERO FINDINGS ✓

### Shared Utilities

Extracted `formatSize()` and `getFormatLabel()` to `src/utils/format.ts`, imported by FileList and MetadataPanel.
