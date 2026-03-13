# Sprint 2 Analysis — Performance & Backend

**Date:** 2026-03-13
**Issues:** #36, #37, #39, #35

---

## Issue #36 — Database query optimizations

### Problem Description
Three database-level inefficiencies:
1. Missing indexes on `file_thread_colors(color_hex)` and `ai_analysis_results(accepted)` — columns used in search queries but not indexed
2. N+1 folder count query — `Sidebar.loadCounts()` calls `get_folder_file_count` once per folder (N Tauri IPC roundtrips)
3. Duplicate recursive CTE in `delete_folder` — identical folder tree CTE executed twice (thumbnail paths + file count)

### Affected Components
- `src-tauri/src/db/migrations.rs` — index definitions (currently: `idx_file_thread_colors_file_id` at line 137, `idx_ai_analysis_results_file_id` at line 171, but no index on `color_hex` or `accepted`)
- `src-tauri/src/commands/folders.rs` — `get_folder_file_count` (lines 165-179), `delete_folder` CTE duplication (lines 114-139)
- `src/components/Sidebar.ts` — `loadCounts()` N+1 pattern (lines 32-46)
- `src/services/FolderService.ts` — `getFileCount()` per-folder RPC (line 34-36)

### Root Cause
- Indexes: schema v1 indexed `file_id` FK columns but not columns used in WHERE/JOIN filters for color search and AI status queries
- N+1: `loadCounts` uses `Promise.all(folders.map(f => getFileCount(f.id)))` — each call is a separate Tauri invoke + mutex lock
- CTE: `delete_folder` runs the same recursive CTE twice because thumbnail paths and file count are queried separately

### Proposed Approach

**Step 1 — Add missing indexes (migration v4):**
Bump schema version to 4. Add:
```sql
CREATE INDEX IF NOT EXISTS idx_file_thread_colors_hex ON file_thread_colors(color_hex);
CREATE INDEX IF NOT EXISTS idx_ai_analysis_accepted ON ai_analysis_results(accepted);
```

**Step 2 — Batch folder counts:**
Add `get_all_folder_file_counts` command returning `HashMap<i64, i64>` via single query:
```sql
SELECT folder_id, COUNT(*) FROM embroidery_files GROUP BY folder_id
```
Update `FolderService.ts` to expose `getAllFileCounts()`. Update `Sidebar.loadCounts()` to call it once instead of N times. Keep `get_folder_file_count` for backward compatibility.

**Step 3 — Merge delete_folder CTEs:**
Combine into single query returning both thumbnail paths and count:
```sql
WITH RECURSIVE folder_tree(id) AS (...)
SELECT e.thumbnail_path, (SELECT COUNT(*) FROM embroidery_files e2
  JOIN folder_tree ft2 ON e2.folder_id = ft2.id) as total_count
FROM embroidery_files e JOIN folder_tree ft ON e.folder_id = ft.id
WHERE e.thumbnail_path IS NOT NULL AND e.thumbnail_path != ''
```
Or simpler: query thumbnail paths, derive count from result length + a separate count of files without thumbnails.

**Step 4 — Query params (skip):**
The `Vec<Box<dyn ToSql>>` pattern in `query_files_impl` is the idiomatic rusqlite approach for dynamic WHERE clauses. The overhead is negligible (one allocation per search query). Not worth refactoring.

---

## Issue #37 — Reduce mutex lock contention in batch operations

### Problem Description
`batch_rename`, `batch_organize`, and `batch_export_usb` acquire and release the DB mutex once per file in the loop. For 100 files = 200 lock/unlock cycles (read + update per file).

### Affected Components
- `src-tauri/src/commands/batch.rs`:
  - `batch_rename` (lines 110-236): lock per file for read (line 131), then re-lock for update (line 184)
  - `batch_organize` (lines 239-397): same pattern
  - `batch_export_usb` (lines 399-491): lock per file for read, no update needed

### Root Cause
Each iteration of the file loop independently locks the DB to read metadata, drops lock for filesystem I/O, then re-locks for the DB update. This was correct for safety but creates unnecessary contention.

### Proposed Approach

**Step 1 — Restructure batch_rename (3-phase):**
1. Phase 1 (single lock): Load all file metadata in one query (`WHERE id IN (...)`)
2. Phase 2 (no lock): Perform all filesystem renames, emit progress events, collect results
3. Phase 3 (single lock): Update all successful DB records in a single transaction
4. Handle partial failures: only update DB for files that were successfully renamed

**Step 2 — Restructure batch_organize (3-phase):**
Same pattern. Phase 2 creates directories and moves files. Phase 3 updates `filepath` for successful moves.

**Step 3 — Restructure batch_export_usb (2-phase):**
1. Phase 1 (single lock): Load all file paths
2. Phase 2 (no lock): Copy all files to USB destination, emit progress
No Phase 3 needed since export doesn't modify DB.

**Step 4 — Document TOCTOU window:**
Add comments explaining the larger TOCTOU window between Phase 1 and Phase 3. Acceptable for single-user desktop app. If a file is externally modified between phases, the DB update will still reference the old path — which was already the case with the per-file approach.

---

## Issue #39 — Optimize virtual scroll and render cycles

### Problem Description
Five frontend performance issues:
1. `FileList.renderVisible()` clears all DOM on every scroll (line 136: `innerHTML = ""`)
2. `MetadataPanel` does full re-render on tag changes (calls `this.render()`)
3. `MetadataPanel` has no same-file skip — re-renders even when clicking the same file
4. `SearchBar.renderPanel()` fully recreates filter panel on every filter change (line 157: `panelEl.remove()`)
5. `AppState.get()` uses shallow copy via spread operator (not `structuredClone` as originally thought)

### Affected Components
- `src/components/FileList.ts` — `renderVisible()` (lines 129-240)
- `src/components/MetadataPanel.ts` — `render()` full panel rebuild, no file ID comparison
- `src/components/SearchBar.ts` — `renderPanel()` (lines 155-221)
- `src/state/AppState.ts` — `get()` spread copy (lines 23-34)

### Root Cause
- FileList: virtual scrolling calculates correct visible range but clears entire list before re-rendering
- MetadataPanel: `onSelectionChanged()` always calls `render()` without checking if the file ID changed
- SearchBar: filter panel is rebuilt from scratch on every state change
- AppState: shallow copy per `get()` call; not deep clone, so overhead is moderate

### Proposed Approach

**Step 1 — MetadataPanel same-file skip:**
In `onSelectionChanged()`, compare `selectedFileId` against `this.currentFile?.id`. If identical and file data unchanged, skip `render()`. This is the highest-impact, lowest-effort fix.

**Step 2 — FileList incremental scroll:**
Track currently rendered card range (`renderedStart`, `renderedEnd`). On scroll:
- Calculate new visible range
- Remove cards that left the viewport
- Add cards that entered the viewport
- Reuse existing cards that remain visible

**Step 3 — AppState `getRef()` method:**
Add `getRef<K>(key: K): Readonly<State[K]>` that returns the raw reference without copying. Use for read-only access patterns (render, display). Keep `get()` for cases where mutation is needed. Apply `Object.freeze()` in dev builds for safety.

**Step 4 — SearchBar selective update (defer):**
The filter panel has ~15-20 DOM elements and is rebuilt only on explicit user interactions (not scroll). The impact is minimal. Defer to a future sprint if profiling reveals it's a bottleneck.

**Step 5 — MetadataPanel selective tag update (defer):**
Tag changes already work correctly with dirty tracking. Extracting a `renderTags()` method is a moderate refactor with limited benefit since tags are a small DOM subtree. Defer.

---

## Issue #35 — Extract shared file import helpers

### Problem Description
Four functions duplicate ~200+ lines of file pre-parsing and metadata insertion:
- `import_files` (scanner.rs:92-275)
- `mass_import` (scanner.rs:306-614)
- `watcher_auto_import` (scanner.rs:666-842)
- `migrate_from_2stitch` (migration.rs:237-515)

### Affected Components
- `src-tauri/src/commands/scanner.rs` — three import functions with duplicated pre-parse (~30 lines × 3), metadata update SQL (~20 lines × 3), thread color insertion (~15 lines × 3), format insertion (~10 lines × 3)
- `src-tauri/src/commands/migration.rs` — fourth copy with additional 2stitch-specific logic

### Root Cause
Each import path was developed independently with copy-paste. The core logic (parse file → extract metadata → insert into DB) is identical but embedded in different control flow (single import, batch import, watcher import, migration import).

### Proposed Approach

**Step 1 — Define shared structs:**
```rust
pub struct PreParsedFile {
    pub filepath: String,
    pub filename: String,
    pub file_size: Option<i64>,
    pub ext: Option<String>,
    pub parsed: Option<ParsedFileInfo>,
}
```

**Step 2 — Extract `pre_parse_file()`:**
```rust
pub fn pre_parse_file(filepath: &str) -> PreParsedFile
```
Consolidates: path parsing, `fs::metadata` read, extension extraction, parser invocation.

**Step 3 — Extract `persist_file_metadata()`:**
```rust
pub fn persist_file_metadata(
    tx: &rusqlite::Transaction,
    file_id: i64,
    info: &ParsedFileInfo,
    thumb_gen: &ThumbnailGenerator,
    file_data: &[u8],
    ext: &str,
) -> Result<(), AppError>
```
Consolidates: UPDATE for stitch_count/color_count/dimensions, INSERT thread colors, INSERT file_formats, thumbnail generation.

**Step 4 — Refactor callers:**
Update `import_files`, `mass_import`, `watcher_auto_import` to use the shared helpers. For `migrate_from_2stitch`, use `pre_parse_file` but keep custom metadata merge logic (2stitch brand colors, notes, tags) as a post-processing step after calling `persist_file_metadata`.

**Step 5 — Place helpers:**
Add helpers as `pub fn` in `scanner.rs` (or a new `src-tauri/src/commands/import_helpers.rs` module). Since migration.rs already imports from scanner, placing them in scanner.rs avoids circular dependencies.

---

## Implementation Order

1. **#35** (Extract helpers) — Do first since it touches scanner.rs and migration.rs. Reduces risk of merge conflicts with other changes.
2. **#36** (DB optimizations) — Independent of other issues. Add indexes, batch folder counts, merge CTEs.
3. **#37** (Batch mutex) — Refactor batch.rs. May benefit from #35's cleaner helpers if batch operations reuse import logic (they don't directly, but establishes the pattern).
4. **#39** (Render optimizations) — Frontend-only changes. Independent of backend issues. Same-file skip and incremental scroll.

---

## Deferred Items

- `query_files_impl` heap-allocated params (#36 Step 4): idiomatic rusqlite, negligible overhead
- SearchBar selective panel update (#39 Step 4): low-impact, ~15 DOM elements
- MetadataPanel selective tag render (#39 Step 5): low-impact, small DOM subtree
