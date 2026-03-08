# Sprint 4 Analysis: Datei-Import & Liste

**Date:** 2026-03-08
**Sprint:** 4
**Tickets:** S4-T1 through S4-T6

---

## 1. Problem Description / Requirements

The application currently has folder management (CRUD) but no way to discover, import, or display embroidery files. Sprint 4 delivers the core file-import pipeline and the center-panel UI for browsing imported files. Specifically:

- **Directory scanning:** Recursively walk a folder on disk, discover embroidery files (.pes, .dst, .jef, .vp3), emit progress events, and import them into the database.
- **File read queries:** Retrieve files with filtering (by folder, search text, format), plus detail queries for formats, colors, and tags.
- **Frontend services:** TypeScript wrappers around the Tauri commands and event listeners.
- **Search bar:** Debounced text input that drives `appState.searchQuery`.
- **Format filter chips:** One-click format filtering (Alle / PES / DST / JEF / VP3).
- **File list:** Reactive mini-card grid in the center panel that responds to folder selection, search, and format filter changes.

Together these tickets turn StitchManager from a folder browser into a file browser with import capability.

---

## 2. Affected Components

### Files to create

| Ticket | File | Purpose |
|--------|------|---------|
| S4-T1 | `src-tauri/src/commands/scanner.rs` | `scan_directory` and `import_files` commands |
| S4-T2 | `src-tauri/src/commands/files.rs` | `get_files`, `get_file`, `get_file_formats`, `get_file_colors`, `get_file_tags` commands |
| S4-T3 | `src/services/FileService.ts` | Frontend wrappers for file read commands |
| S4-T3 | `src/services/ScannerService.ts` | Frontend wrappers for scan/import commands + event listeners |
| S4-T4 | `src/components/SearchBar.ts` | Debounced search input component |
| S4-T5 | `src/components/FilterChips.ts` | Format filter chip bar component |
| S4-T6 | `src/components/FileList.ts` | File mini-card list component |

### Files to modify

| File | Change |
|------|--------|
| `src-tauri/src/commands/mod.rs` | Add `pub mod scanner;` and `pub mod files;` |
| `src-tauri/src/lib.rs` | Register new commands in `generate_handler![]` |
| `src-tauri/capabilities/default.json` | Add `"event:default"` permission if needed for Tauri event emission |
| `src/main.ts` | Import and mount SearchBar, FilterChips, FileList components; extend Tauri bridge with `scan:file-found` and `scan:complete` listeners |
| `src/styles/components.css` | Add styles for SearchBar, FilterChips, FileList |
| `src/main.ts` (initTauriBridge) | Add listeners for `scan:file-found` and `scan:complete` events |

### Files referenced (read-only context)

- `src-tauri/src/error.rs` — `AppError`, `lock_db()` helper
- `src-tauri/src/db/models.rs` — `EmbroideryFile`, `FileFormat`, `FileThreadColor`, `Tag` structs
- `src-tauri/src/db/migrations.rs` — table schemas (embroidery_files, file_formats, file_thread_colors, tags, file_tags)
- `src/types/index.ts` — `EmbroideryFile`, `FileFormat`, `ThreadColor`, `Tag`, `State` interfaces
- `src/state/AppState.ts` — `appState` singleton, `State` type already includes `searchQuery`, `formatFilter`, `selectedFolderId`, `files`
- `src/state/EventBus.ts` — `EventBus` for scan events
- `src/components/Component.ts` — base class with `render()`, `subscribe()`, `destroy()`
- `src/styles/aurora.css` — design tokens (accent colors, radius-pill, spacing, font sizes)

---

## 3. Rationale

The folder CRUD from Sprint 3 established the organizational structure, but without file import and listing, the application has no content to manage. Sprint 4 is the critical data-entry point:

- **Scanning** is preferred over manual file-by-file import because embroidery collections typically contain hundreds to thousands of files organized in deep directory trees.
- **Progress events** provide feedback for potentially long-running scans (large collections).
- **Filtering and search** are essential UX features because users need to locate specific designs quickly in large collections.
- **Reactive state** (AppState subscriptions) ensures the UI stays consistent when the user navigates between folders, changes search terms, or toggles format filters, without manual refresh.

---

## 4. Proposed Approach

### S4-T1: `commands/scanner.rs` — Directory scanner

**Step-by-step:**

1. Create `src-tauri/src/commands/scanner.rs`.
2. Define a serializable `ScanResult` struct:
   ```rust
   #[derive(Debug, Clone, Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct ScanResult {
       pub found_files: Vec<String>,  // absolute file paths
       pub total_scanned: usize,
       pub duplicates_skipped: usize,
   }
   ```
3. Define serializable event payload structs for `scan:progress`, `scan:file-found`, `scan:complete`.
4. Implement `scan_directory(path: String, app_handle: tauri::AppHandle) -> Result<ScanResult, AppError>`:
   - Use `walkdir::WalkDir::new(&path)` for recursive traversal.
   - Filter entries by extension: `.pes`, `.dst`, `.jef`, `.vp3` (case-insensitive comparison via `.to_ascii_lowercase()`).
   - For each matching file, emit `scan:file-found` event via `app_handle.emit("scan:file-found", &payload)`.
   - Emit periodic `scan:progress` events (e.g., every 50 entries or every 500ms) with `{ scanned: usize, found: usize }`.
   - Collect found paths into a `Vec<String>`.
   - Emit `scan:complete` event at the end.
   - Return `ScanResult`.
5. Implement `import_files(db: State<DbState>, file_paths: Vec<String>, folder_id: i64) -> Result<Vec<EmbroideryFile>, AppError>`:
   - Lock the database via `lock_db()`.
   - For each file path:
     - Extract `filename` from path.
     - Get `file_size_bytes` via `std::fs::metadata`.
     - INSERT INTO `embroidery_files` with `INSERT OR IGNORE` to handle the UNIQUE filepath constraint (duplicate detection).
     - If the insert affected 0 rows, increment a skipped counter; otherwise, query back the inserted row.
   - Also insert a row into `file_formats` for the primary format (extract extension as format string).
   - Return the vector of successfully inserted `EmbroideryFile` records.
6. Register both commands in `mod.rs` and `lib.rs`.
7. Write tests using `init_database_in_memory()` and `tempdir` for filesystem scanning tests.

**Key decisions:**
- `scan_directory` does NOT write to the database; it only discovers files. `import_files` is the separate write step. This two-phase design lets the UI show discovered files first and let the user confirm import.
- The `app_handle` parameter is Tauri's mechanism for emitting global events from within a command. The `#[tauri::command]` macro supports `app_handle: tauri::AppHandle` as a special injectable parameter.
- Use `INSERT OR IGNORE` rather than checking existence first to avoid TOCTOU races and simplify the code. The UNIQUE constraint on `filepath` handles deduplication.

### S4-T2: `commands/files.rs` — File read operations

**Step-by-step:**

1. Create `src-tauri/src/commands/files.rs`.
2. Define a `row_to_embroidery_file` helper (same pattern as `row_to_folder` in `folders.rs`).
3. Define a base SELECT constant:
   ```rust
   const FILE_SELECT: &str = "SELECT id, folder_id, filename, filepath, name, theme, description, license, width_mm, height_mm, stitch_count, color_count, file_size_bytes, thumbnail_path, ai_analyzed, ai_confirmed, created_at, updated_at FROM embroidery_files";
   ```
4. Implement `get_files(db, folder_id: Option<i64>, search: Option<String>, format_filter: Option<String>) -> Result<Vec<EmbroideryFile>, AppError>`:
   - Build a dynamic WHERE clause from the optional parameters.
   - `folder_id` -> `WHERE folder_id = ?`
   - `search` -> `WHERE (name LIKE ? OR filename LIKE ?)` using `%search%` pattern
   - `format_filter` -> JOIN or subquery: `WHERE id IN (SELECT file_id FROM file_formats WHERE UPPER(format) = UPPER(?))`
   - Combine conditions with AND. Use a `Vec<Box<dyn rusqlite::types::ToSql>>` for dynamic parameter binding.
   - ORDER BY `filename ASC`.
5. Implement `get_file(db, file_id: i64) -> Result<EmbroideryFile, AppError>`:
   - Single row query. Map `QueryReturnedNoRows` to `AppError::NotFound`.
6. Implement `get_file_formats(db, file_id: i64) -> Result<Vec<FileFormat>, AppError>`:
   - Query `file_formats WHERE file_id = ?`, ordered by `format`.
7. Implement `get_file_colors(db, file_id: i64) -> Result<Vec<FileThreadColor>, AppError>`:
   - Query `file_thread_colors WHERE file_id = ?`, ordered by `sort_order`.
8. Implement `get_file_tags(db, file_id: i64) -> Result<Vec<Tag>, AppError>`:
   - JOIN `file_tags` with `tags`: `SELECT t.* FROM tags t JOIN file_tags ft ON ft.tag_id = t.id WHERE ft.file_id = ?`.
9. Register all five commands.
10. Write unit tests for each query, including edge cases (no results, NotFound).

**Key decisions for dynamic query building:**
- Use a `Vec<String>` for WHERE clauses and `rusqlite::params_from_iter()` for parameter binding. This avoids SQL injection while supporting optional filters.
- The `format_filter` uses a subquery on `file_formats` rather than a JOIN on the main query to avoid duplicating rows when a file has multiple format variants.

### S4-T3: `FileService.ts` and `ScannerService.ts`

**Step-by-step:**

1. Create `src/services/FileService.ts` following the `FolderService.ts` pattern:
   ```typescript
   import { invoke } from "@tauri-apps/api/core";
   import type { EmbroideryFile, FileFormat, ThreadColor, Tag } from "../types/index";

   export async function getFiles(folderId?: number | null, search?: string | null, formatFilter?: string | null): Promise<EmbroideryFile[]> { ... }
   export async function getFile(fileId: number): Promise<EmbroideryFile> { ... }
   export async function getFormats(fileId: number): Promise<FileFormat[]> { ... }
   export async function getColors(fileId: number): Promise<ThreadColor[]> { ... }
   export async function getTags(fileId: number): Promise<Tag[]> { ... }
   ```
2. Create `src/services/ScannerService.ts`:
   ```typescript
   import { invoke } from "@tauri-apps/api/core";
   import type { EmbroideryFile } from "../types/index";

   export interface ScanResult {
     foundFiles: string[];
     totalScanned: number;
     duplicatesSkipped: number;
   }

   export async function scanDirectory(path: string): Promise<ScanResult> { ... }
   export async function importFiles(filePaths: string[], folderId: number): Promise<EmbroideryFile[]> { ... }
   ```
3. The ScannerService does NOT listen to events itself; event bridging is already handled in `main.ts` (initTauriBridge) which forwards Tauri events to EventBus. Components that need scan progress subscribe to EventBus directly.

**Note:** The `scanDirectory` invoke call needs `appHandle` on the Rust side, but Tauri commands automatically receive the `AppHandle` when declared as a parameter -- the frontend just calls `invoke("scan_directory", { path })` normally.

### S4-T4: `SearchBar` component

**Step-by-step:**

1. Create `src/components/SearchBar.ts` extending `Component`.
2. In `render()`:
   - Create an input container `div.search-bar`.
   - Add a search icon (SVG or Unicode magnifying glass character `\u{1F50D}` -- prefer a simple SVG for consistency).
   - Create an `<input type="text" placeholder="Suchen..." class="search-input">`.
   - Create a clear button (`x`) that is visible only when the input has text.
3. Debounce logic:
   - Store a `private debounceTimer: ReturnType<typeof setTimeout> | null`.
   - On `input` event, clear existing timer and set a new 300ms timeout.
   - When the timer fires, call `appState.set('searchQuery', value)`.
   - On clear button click, immediately set value to empty string and `appState.set('searchQuery', '')`.
4. Subscribe to `appState.on('searchQuery', ...)` so that if the state is changed externally, the input value stays synchronized.
5. In `destroy()`, clear any pending timer via the base class cleanup.

**Key decision:** 300ms debounce is a standard value that balances responsiveness with avoiding excessive backend calls. The debounce is purely on the frontend; the actual query fires via the FileList component reacting to `searchQuery` state changes.

### S4-T5: `FilterChips` component

**Step-by-step:**

1. Create `src/components/FilterChips.ts` extending `Component`.
2. Define chip data: `[{ label: 'Alle', value: null }, { label: 'PES', value: 'PES' }, { label: 'DST', value: 'DST' }, { label: 'JEF', value: 'JEF' }, { label: 'VP3', value: 'VP3' }]`.
3. In `render()`:
   - Create a `div.filter-chips` container.
   - For each chip, create a `<button class="filter-chip">` with the label text.
   - If the chip's `value` matches `appState.get('formatFilter')`, add the `.active` class.
   - On click, call `appState.set('formatFilter', chip.value)`.
4. Subscribe to `appState.on('formatFilter', () => this.render())` to re-render on state change.

**CSS styling:**
- `.filter-chip`: `border-radius: var(--radius-pill)`, `padding: var(--spacing-1) var(--spacing-3)`, `border: 1px solid var(--color-border)`, `background: var(--color-surface)`, `font-size: var(--font-size-caption)`, `cursor: pointer`.
- `.filter-chip.active`: `background: var(--color-accent)`, `color: white`, `border-color: var(--color-accent)`.

### S4-T6: `FileList` component

**Step-by-step:**

1. Create `src/components/FileList.ts` extending `Component`.
2. Subscribe to three state keys: `selectedFolderId`, `searchQuery`, `formatFilter`. On any change, call `loadFiles()`.
3. `loadFiles()`:
   - Read current values from `appState` for `selectedFolderId`, `searchQuery`, `formatFilter`.
   - Call `FileService.getFiles(folderId, search, formatFilter)`.
   - Store result in `appState.set('files', result)`.
   - Call `this.render()`.
4. `render()`:
   - Read `files` from `appState.get('files')` and `selectedFileId` from `appState.get('selectedFileId')`.
   - If files is empty, show an empty state: `div.file-list-empty` with text "Keine Dateien gefunden".
   - Otherwise, create a `div.file-list` grid container.
   - For each file, create a mini-card `div.file-card`:
     - Thumbnail placeholder: `div.file-thumbnail` with a generic icon or colored background.
     - Filename: `span.file-card-name` with `file.filename`, truncated with ellipsis.
     - Format chip: `span.file-card-format` showing the file extension (extracted from filename).
   - If `file.id === selectedFileId`, add `.selected` class.
   - On click, call `appState.set('selectedFileId', file.id)`.
5. Also subscribe to `appState.on('selectedFileId', () => this.render())` for highlight updates without reloading data.

**CSS styling:**
- `.file-list`: CSS Grid, `grid-template-columns: repeat(auto-fill, minmax(140px, 1fr))`, `gap: var(--spacing-3)`.
- `.file-card`: `background: var(--color-surface)`, `border: 1px solid var(--color-border-light)`, `border-radius: var(--radius-card)`, `padding: var(--spacing-2)`, `cursor: pointer`, `box-shadow: var(--shadow-xs)`.
- `.file-card.selected`: `border-color: var(--color-accent)`, `box-shadow: 0 0 0 2px var(--color-accent-20)`.
- `.file-thumbnail`: `aspect-ratio: 1`, `background: var(--color-bg)`, `border-radius: var(--radius-swatch)`, `display: flex`, `align-items: center`, `justify-content: center`, `color: var(--color-muted)`.

---

## 5. Key Design Decisions

### Event types and payloads for the scanner

The scanner emits three Tauri event types. Payloads must be `Serialize`:

| Event | Payload | When |
|-------|---------|------|
| `scan:progress` | `{ scanned: number, found: number }` | Every 50 directory entries or 500ms, whichever comes first |
| `scan:file-found` | `{ path: string, filename: string, extension: string }` | Each matching embroidery file |
| `scan:complete` | `{ totalScanned: number, totalFound: number, durationMs: number }` | End of scan |

These events are emitted via `app_handle.emit("scan:progress", &payload)` which is the Tauri v2 global event emission API. The `initTauriBridge` in `main.ts` already listens for `scan:progress` and forwards to EventBus. We need to add listeners for `scan:file-found` and `scan:complete`.

### Query building strategy for `get_files`

Dynamic WHERE clause construction in Rust using a builder pattern:

```rust
let mut conditions: Vec<String> = Vec::new();
let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

if let Some(fid) = folder_id {
    conditions.push("folder_id = ?".into());
    params.push(Box::new(fid));
}
if let Some(ref q) = search {
    let like = format!("%{q}%");
    conditions.push("(name LIKE ? OR filename LIKE ?)".into());
    params.push(Box::new(like.clone()));
    params.push(Box::new(like));
}
if let Some(ref fmt) = format_filter {
    conditions.push("id IN (SELECT file_id FROM file_formats WHERE UPPER(format) = UPPER(?))".into());
    params.push(Box::new(fmt.clone()));
}

let where_clause = if conditions.is_empty() {
    String::new()
} else {
    format!(" WHERE {}", conditions.join(" AND "))
};
```

This approach avoids string interpolation of user values (all values go through parameter binding) while supporting any combination of optional filters.

### Debounce strategy

- The 300ms debounce is implemented purely in the SearchBar component using `setTimeout`/`clearTimeout`.
- The debounce gates writes to `appState.searchQuery`. The FileList component reacts to `appState.on('searchQuery', ...)` and triggers a backend call.
- This means the backend only receives fully debounced queries, not intermediate keystrokes.
- The clear button bypasses the debounce (immediate clear) for snappy UX.

### Component mounting in `main.ts`

The SearchBar and FilterChips will be mounted into the `.app-toolbar` element (which currently just has placeholder text "Toolbar"). The FileList will be mounted into `.app-center`. This matches the layout grid areas defined in `layout.css`.

---

## 6. Risk Areas

### UNIQUE constraint handling on `embroidery_files.filepath`

- **Risk:** `INSERT OR IGNORE` silently skips duplicates, making it invisible to the user whether a file was skipped or if there was a different error.
- **Mitigation:** Use a two-step approach: first check `SELECT COUNT(*) FROM embroidery_files WHERE filepath = ?`, then insert. Alternatively, use `INSERT OR IGNORE` and check `changes()` to count actual inserts vs. skips. The latter is preferred for performance in batch imports. Report `duplicatesSkipped` count in the return value so the UI can display it.
- **Edge case:** Filepath normalization. On macOS, `/Users/foo/bar` and `/Users/foo/Bar` may or may not refer to the same file (APFS is case-insensitive by default). We should normalize paths using `std::fs::canonicalize()` before insert to ensure consistent UNIQUE constraint behavior.

### Async scanning and Tauri command threading

- **Risk:** `scan_directory` with `walkdir` is synchronous and could block the Tauri command thread for large directories.
- **Mitigation:** Tauri v2 runs `#[tauri::command]` handlers on a thread pool by default (not the main thread), so filesystem I/O is acceptable. However, if we want true async with cancellation support, we would need `tokio::task::spawn_blocking`. For MVP, the default thread pool behavior is sufficient.
- **Risk:** Emitting events from within a synchronous command -- `app_handle.emit()` is safe to call from any thread in Tauri v2.

### Event serialization

- **Risk:** Event payloads must implement `Serialize`. If a payload struct is missing `#[derive(Serialize)]` or has non-serializable fields, the event emission silently fails (no compile error, just a runtime log).
- **Mitigation:** Define dedicated payload structs (not reusing model structs) with only primitive fields. Write a unit test that serializes each payload to JSON to catch issues at test time.

### Dynamic SQL query safety

- **Risk:** Building SQL strings dynamically could introduce injection vulnerabilities if values are concatenated.
- **Mitigation:** All user-provided values (search terms, format names, folder IDs) go through `rusqlite` parameter binding (`?` placeholders). Only structural SQL keywords (WHERE, AND, ORDER BY) are string-concatenated. The search `LIKE` pattern wrapping (`%value%`) is done in Rust before passing as a bound parameter.

### State synchronization between search/filter and file list

- **Risk:** Race condition where rapid state changes (typing fast, clicking chips) could cause out-of-order responses to `get_files` calls, showing stale results.
- **Mitigation:** In `FileList.loadFiles()`, use a generation counter or request ID. Increment on each call; when the response arrives, discard it if a newer request has been issued. Simple pattern:
  ```typescript
  private loadGeneration = 0;
  async loadFiles() {
    const gen = ++this.loadGeneration;
    const files = await FileService.getFiles(...);
    if (gen !== this.loadGeneration) return; // stale
    appState.set('files', files);
    this.render();
  }
  ```

### Toolbar layout for SearchBar + FilterChips

- **Risk:** The toolbar area is only 48px tall. Both components must fit in a single horizontal row.
- **Mitigation:** Use `display: flex` with `align-items: center` and `gap`. SearchBar takes `flex: 1` (fills available space), FilterChips uses fixed-width chip buttons. The filter chips can scroll horizontally if needed, but with only 5 options (Alle + 4 formats) this is unlikely to overflow.

---

## 7. Implementation Order

The recommended implementation order respects dependencies:

1. **S4-T1** (scanner.rs) -- no frontend dependency, can be tested in isolation
2. **S4-T2** (files.rs) -- no frontend dependency, can be tested in isolation
3. **S4-T3** (FileService.ts, ScannerService.ts) -- depends on T1 + T2 commands being registered
4. **S4-T4** (SearchBar) -- UI only, writes to appState
5. **S4-T5** (FilterChips) -- UI only, writes to appState
6. **S4-T6** (FileList) -- depends on T3 (FileService), T4 (searchQuery), T5 (formatFilter)

T1 and T2 can be implemented in parallel. T4 and T5 can be implemented in parallel. T3 and T6 are sequential dependencies.

---

## 8. Testing Strategy

### Rust (cargo test)

- **scanner.rs:** Create a temp directory with nested folders containing .pes, .dst, .txt files. Assert that `scan_directory` finds only embroidery files. Test `import_files` with `init_database_in_memory()`: verify DB rows, verify duplicate skipping on second import.
- **files.rs:** Use `init_database_in_memory()`. Insert test data directly via SQL. Test `get_files` with all filter combinations: no filters, folder_id only, search only, format only, all three combined. Test `get_file` with valid and invalid IDs. Test related data queries (formats, colors, tags).

### Frontend (manual / dev mode)

- Mount all components, create a test folder via the Sidebar, trigger scan on a known directory, verify file cards appear.
- Type in search bar, verify 300ms debounce and filtered results.
- Click format chips, verify file list updates.
- Click a file card, verify `selectedFileId` updates.

---

## 9. Files Summary

### New files (8)
- `src-tauri/src/commands/scanner.rs`
- `src-tauri/src/commands/files.rs`
- `src/services/FileService.ts`
- `src/services/ScannerService.ts`
- `src/components/SearchBar.ts`
- `src/components/FilterChips.ts`
- `src/components/FileList.ts`

### Modified files (4)
- `src-tauri/src/commands/mod.rs` -- add `pub mod scanner;` and `pub mod files;`
- `src-tauri/src/lib.rs` -- register 7 new commands in `generate_handler![]`
- `src/main.ts` -- mount SearchBar, FilterChips, FileList; extend Tauri bridge listeners
- `src/styles/components.css` -- add styles for search bar, filter chips, file list, file cards
