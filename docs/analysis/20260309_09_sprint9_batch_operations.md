# Sprint 9: Batch-Operationen & USB-Export — Analysis

## Problem Description

StitchManager currently only supports single-file operations: one file selected, one AI analysis at a time, one metadata edit at a time. Users with large embroidery collections need batch operations: renaming multiple files at once using patterns, organizing files into folder structures, exporting selections to USB, and running AI analysis on multiple files sequentially. The FileList currently supports only single selection (`selectedFileId: number | null`).

Sprint 9 delivers: multi-select in FileList, Rust batch commands (rename, organize, USB export), a batch AI analysis command, a progress dialog, a frontend BatchService, and a Dateiverwaltung (file management) settings tab.

## Affected Components

### Backend (Rust / Tauri)
- `src-tauri/src/commands/batch.rs` (new) — 3 batch commands: `batch_rename`, `batch_organize`, `batch_export_usb`
- `src-tauri/src/commands/ai.rs` (extend) — new `ai_analyze_batch` command
- `src-tauri/src/commands/mod.rs` — register `batch` module
- `src-tauri/src/lib.rs` — register 4 new commands in `invoke_handler`
- `src-tauri/src/db/models.rs` — add `BatchResult` struct

### Frontend (TypeScript)
- `src/services/BatchService.ts` (new) — frontend service wrapping batch Tauri commands
- `src/services/AiService.ts` (extend) — add `analyzeBatch()` method
- `src/components/BatchDialog.ts` (new) — progress dialog with bar, log, cancel
- `src/components/SettingsDialog.ts` (extend) — add Dateiverwaltung tab
- `src/components/FileList.ts` (extend) — multi-select with Cmd/Ctrl+click and Shift+click
- `src/components/Toolbar.ts` (extend) — show batch actions when multiple files selected
- `src/state/AppState.ts` (extend) — add `selectedFileIds: number[]` to initial state
- `src/types/index.ts` (extend) — add `BatchResult` type, update `State` interface
- `src/main.ts` (extend) — add batch event handlers and `batch:complete` bridge
- `src/styles/components.css` (extend) — BatchDialog styles, multi-select highlight styles

### Database
- No schema changes needed. Settings `rename_pattern`, `organize_pattern`, `library_root`, `metadata_root` already seeded in v1 migration.

## Root Cause / Rationale

Single-file-only workflows are a bottleneck for users managing hundreds or thousands of embroidery files. Batch rename with pattern substitution (`{name}`, `{theme}`, `{format}`), batch organize into folder hierarchies, USB export for machine transfer, and batch AI analysis are core productivity features. The infrastructure is partially in place: `batch:progress` is already bridged in `main.ts`, settings keys exist in DB, and the AI analysis pipeline is complete.

## Proposed Approach

### Execution Order
1. S9-T6 (Multi-select — foundational for all batch operations)
2. S9-T1 (Rust batch commands)
3. S9-T2 (Frontend BatchService)
4. S9-T3 (BatchDialog with progress UI)
5. S9-T4 (SettingsDialog Dateiverwaltung tab)
6. S9-T5 (Batch AI analysis)

### S9-T6: Multi-Select in FileList

1. Add `selectedFileIds: number[]` to `State` interface in `types/index.ts` and to `initialState` in `AppState.ts`.

2. Modify `FileList.ts` click handler:
   - **Normal click**: Clear multi-select, set `selectedFileId` to clicked file (existing behavior).
   - **Cmd/Ctrl+click** (`e.metaKey || e.ctrlKey`): Toggle file in `selectedFileIds` array. If toggling to a single remaining file, also set `selectedFileId`.
   - **Shift+click**: Range select from last selected to clicked file. Set `selectedFileIds` to the range.
   - Visual: Add `selected` class to all cards whose file.id is in `selectedFileIds`.

3. Subscribe to `selectedFileIds` in FileList for re-render.

4. Modify `Toolbar.ts`: When `selectedFileIds.length > 1`, show batch action buttons (Batch Umbenennen, Batch Organisieren, USB-Export, Batch KI). When 0 or 1 selected, show single-file actions as before.

### S9-T1: commands/batch.rs

1. Create `src-tauri/src/commands/batch.rs` with:
   - `BatchResult` struct: `{ total: i64, success: i64, failed: i64, errors: Vec<String> }`
   - `BatchProgressPayload` struct for event emission: `{ current: i64, total: i64, filename: String, status: String }`

2. **`batch_rename`** command:
   - Parameters: `file_ids: Vec<i64>`, `pattern: String`, `app_handle: AppHandle`, `db: State<DbState>`
   - For each file: load metadata, apply pattern substitution (`{name}` → file.name, `{theme}` → file.theme, `{format}` → file extension), rename physical file via `std::fs::rename`, update `filename` in DB.
   - Emit `batch:progress` per file with current/total/filename/status.
   - On per-file error: log error, increment failed count, continue to next file.
   - Return `BatchResult`.

3. **`batch_organize`** command:
   - Parameters: `file_ids: Vec<i64>`, `pattern: String`, `app_handle: AppHandle`, `db: State<DbState>`
   - For each file: compute target path using pattern substitution, create directories as needed, move file, update `filepath` in DB.
   - Same progress events and error resilience pattern.

4. **`batch_export_usb`** command:
   - Parameters: `file_ids: Vec<i64>`, `target_path: String`, `app_handle: AppHandle`, `db: State<DbState>`
   - For each file: copy (not move) file to target_path, preserving filename.
   - Same progress and error patterns.

5. Helper `fn apply_pattern(pattern: &str, file: &EmbroideryFile) -> String` for substitution logic.

6. Unit tests: pattern substitution test, organize path construction test.

### S9-T2: BatchService (Frontend)

1. Create `src/services/BatchService.ts`:
   - `rename(fileIds: number[], pattern: string): Promise<BatchResult>`
   - `organize(fileIds: number[], pattern: string): Promise<BatchResult>`
   - `exportUsb(fileIds: number[], targetPath: string): Promise<BatchResult>`

### S9-T3: BatchDialog

1. Create `src/components/BatchDialog.ts` as a modal dialog (480x400):
   - Progress bar: `<div class="batch-progress-bar">` with inner fill div, text "X von N".
   - Log view: `<div class="batch-log">` scrollable container, each entry shows filename + success/error icon.
   - Step indicator: Current operation name.
   - Cancel button: Sets a cancelled flag; the dialog listens for `batch:progress` events and updates UI.
   - Auto-close: On completion (all progress events received), show summary for 2 seconds then close.

2. Static `open(operation: string, total: number): BatchDialog` method that returns instance.
3. `addLogEntry(filename: string, status: "success" | "error", message?: string)` method.
4. `setProgress(current: number, total: number)` method.

### S9-T4: SettingsDialog — Dateiverwaltung Tab

1. Extend `SettingsDialog.ts`:
   - Add a second tab "Dateiverwaltung" alongside existing "KI-Einstellungen".
   - Tab switching logic (show/hide tab content).
   - Fields:
     - Umbennungsmuster: `<input>` bound to `rename_pattern`, default `{name}_{theme}`
     - Organisationsmuster: `<input>` bound to `organize_pattern`, default `{theme}/{name}`
     - Bibliotheks-Stammverzeichnis: `<input>` + browse button bound to `library_root`
     - Metadaten-Verzeichnis: `<input>` bound to `metadata_root`
   - Platzhalter-Legende: Small info section explaining `{name}`, `{theme}`, `{format}` placeholders.
   - Save persists all via `SettingsService.setSetting()`.

### S9-T5: Batch AI Analysis

1. Extend `src-tauri/src/commands/ai.rs`:
   - New `ai_analyze_batch` async command: `file_ids: Vec<i64>`, `app_handle: AppHandle`, `db: State<DbState>`
   - Sequentially (not parallel) processes each file:
     1. Build prompt via existing `ai_build_prompt` logic
     2. Load thumbnail, create AI client, call analyze
     3. Store result in DB
     4. Emit `batch:progress` event per file
   - On per-file error: record error, emit error progress, continue
   - Return `Vec<AiAnalysisResult>` (successful results only)

2. Extend `src/services/AiService.ts`:
   - Add `analyzeBatch(fileIds: number[]): Promise<AiAnalysisResult[]>`

3. Wire up in `main.ts`: `toolbar:batch-ai` event handler opens BatchDialog then calls `analyzeBatch`.

### Integration Points

1. **main.ts**: Add event handlers for `toolbar:batch-rename`, `toolbar:batch-organize`, `toolbar:batch-export`, `toolbar:batch-ai`. Each opens appropriate dialog (pattern input or folder picker) then BatchDialog.
2. **Event bridge**: `batch:progress` bridged from Tauri to EventBus.
3. **Toolbar**: Batch buttons emit toolbar events; batch button visibility tied to `selectedFileIds.length > 1`.

---

## Closure Summary

**Commit:** `c98a13f` — Implement Sprint 9: Batch operations, multi-select, and USB export

**Solution:** All 6 tickets (S9-T1 through S9-T6) implemented as planned:

- **S9-T1:** `commands/batch.rs` with `batch_rename`, `batch_organize`, `batch_export_usb`, pattern substitution (`{name}`, `{theme}`, `{format}`), path traversal sanitization (`sanitize_path_component`, `sanitize_pattern_output`), canonicalize check in organize, USB filename collision dedup, 7 unit tests.
- **S9-T2:** `BatchService.ts` with `rename()`, `organize()`, `exportUsb()`.
- **S9-T3:** `BatchDialog.ts` with progress bar, log view, close button, auto-close after 2s.
- **S9-T4:** `SettingsDialog.ts` Dateiverwaltung tab with rename/organize patterns, library_root, metadata_root, placeholder legend.
- **S9-T5:** `ai_analyze_batch` command with shared `build_prompt_for_file()` helper, sequential processing, error resilience.
- **S9-T6:** Multi-select in FileList (Cmd/Ctrl+click toggle, Shift+click range), `selectedFileIds` state, batch buttons in Toolbar.

**Review:** 4 review agents passed with 0 findings (Round 2). Round 1 identified 10 issues (path traversal, duplicate struct, non-functional cancel, missing CSS, double extension, filename collisions, duplicated prompt logic, dead events, non-atomic ops, folder_id) — all resolved.
