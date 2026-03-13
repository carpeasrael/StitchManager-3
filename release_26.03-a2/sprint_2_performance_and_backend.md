# Sprint 2 — Performance & Backend

**Focus:** Optimize DB queries, batch operations, rendering, and reduce backend code duplication
**Issues:** #36, #37, #39, #35

---

## Issue #36 — Database query optimizations

**Type:** Performance
**Effort:** M

### Problem
Missing indexes, N+1 folder count query, duplicate recursive CTE, heap-allocated query params.

### Affected Files
- `src-tauri/src/db/migrations.rs` — add indexes
- `src-tauri/src/commands/folders.rs` — `get_folder_file_count` N+1 query
- `src-tauri/src/commands/folders.rs` — `delete_folder` duplicate CTE
- `src-tauri/src/commands/files.rs` — `query_files_impl` heap-allocated params

### Implementation Plan

#### Add missing indexes (Step 1)
1. Add a new migration (or extend existing) with:
   ```sql
   CREATE INDEX IF NOT EXISTS idx_file_thread_colors_hex ON file_thread_colors(color_hex);
   CREATE INDEX IF NOT EXISTS idx_ai_analysis_accepted ON ai_analysis_results(accepted);
   ```

#### Fix N+1 folder count (Step 2)
2. Replace per-folder `get_folder_file_count` with a single JOIN query:
   ```sql
   SELECT f.id, COUNT(e.id) as file_count
   FROM folders f LEFT JOIN embroidery_files e ON f.id = e.folder_id
   GROUP BY f.id
   ```
   Return a `HashMap<i64, i64>` or attach counts directly to folder structs.

#### Deduplicate recursive CTE in delete_folder (Step 3)
3. Combine the two recursive CTEs (thumbnail path collection + file count) into a single query that returns both thumbnail paths and file count.

#### Optimize query params (Step 4)
4. Refactor `query_files_impl` to use `rusqlite::params!` macro where possible, or a fixed-size array for common query patterns. For dynamic WHERE clauses, consider `rusqlite::params_from_iter`.

### Verification
- `cargo test` — all DB tests pass
- `EXPLAIN QUERY PLAN` on key queries shows index usage
- Benchmark sidebar load with 100+ folders (should be noticeably faster)

---

## Issue #37 — Reduce mutex lock contention in batch operations

**Type:** Performance
**Effort:** M

### Problem
`batch_rename`, `batch_organize`, `batch_export_usb` acquire and release the DB lock once per file. For 100 files = 100 lock/unlock cycles.

### Affected Files
- `src-tauri/src/commands/batch.rs` — all three batch functions

### Implementation Plan

#### Restructure batch_rename (Step 1)
1. Phase 1: Acquire lock once → load all file metadata in a single transaction
2. Phase 2: Release lock → perform all filesystem operations, collecting results
3. Phase 3: Acquire lock once → update all DB records in a single transaction
4. Handle partial failures: if some renames fail, only update DB for successful ones

#### Restructure batch_organize (Step 2)
5. Same 3-phase pattern as batch_rename
6. Create destination directories before moving files (outside lock)

#### Restructure batch_export_usb (Step 3)
7. Phase 1: Acquire lock once → load all file paths
8. Phase 2: Release lock → copy all files to USB
9. No Phase 3 needed (export doesn't modify DB)

#### TOCTOU documentation (Step 4)
10. Add code comments documenting the TOCTOU window and acceptable failure modes
11. The batch progress event emission should happen in Phase 2 (filesystem ops)

### Verification
- Batch rename 50 files — verify all renamed correctly
- Batch organize 50 files — verify all moved to correct directories
- USB export 50 files — verify all copied
- Measure lock time: should be ~2 acquisitions vs N

---

## Issue #39 — Optimize virtual scroll and render cycles

**Type:** Performance
**Effort:** L

### Problem
5 performance issues: full DOM clear on scroll, full re-render on tag change, no same-file skip, full panel recreation on filter change, expensive deep-copy on every `get()`.

### Affected Files
- `src/components/FileList.ts` — `renderVisible()` innerHTML clear
- `src/components/MetadataPanel.ts` — full re-render on tag change + no same-file skip
- `src/components/SearchBar.ts` — `renderPanel()` full recreation
- `src/state/AppState.ts` — `structuredClone` on every `get()`

### Implementation Plan

#### FileList incremental scroll (Step 1)
1. Track currently rendered card indices
2. On scroll, calculate new visible range
3. Remove cards that scrolled out of view
4. Add cards that scrolled into view
5. Reposition existing cards if needed (translate3d)

#### MetadataPanel same-file skip (Step 2)
6. Store `currentFileId` in MetadataPanel
7. In `onSelectionChanged()`, skip re-render if selected file ID matches `currentFileId`
8. Still update if the file's data has changed (compare a hash or timestamp)

#### MetadataPanel selective tag update (Step 3)
9. Extract tag rendering into a `renderTags()` method
10. On tag add/remove, call only `renderTags()` instead of full `render()`
11. Mark dirty state without full re-render

#### SearchBar selective filter update (Step 4)
12. Instead of removing and recreating the entire filter panel, update only the changed section
13. Preserve focus state across filter changes

#### AppState selective cloning (Step 5)
14. Add a `getRef()` method that returns a read-only reference (no clone)
15. Keep `get()` for cases where mutation is needed
16. Document that `getRef()` callers must not mutate the returned value
17. Alternatively, use `Object.freeze()` on returned references for safety

### Verification
- Scroll through 1000+ file list — verify no jank (60fps target)
- Select same file twice — verify no re-render (check via console log or breakpoint)
- Add/remove tags — verify only tag section updates
- Profile with Chrome DevTools Performance tab

---

## Issue #35 — Extract shared file import helpers

**Type:** Refactor
**Effort:** M

### Problem
Four functions duplicate ~200+ lines of file pre-parsing and metadata insertion logic.

### Affected Files
- `src-tauri/src/commands/scanner.rs` — `import_files`, `watcher_auto_import`, `mass_import`
- `src-tauri/src/commands/migration.rs` — `migrate_from_2stitch` (if exists, or similar location)

### Implementation Plan

#### Extract pre_parse_file (Step 1)
1. Create `PreParsedFile` struct with fields: filename, file_size, extension, parsed_info (optional)
2. Create `fn pre_parse_file(filepath: &str) -> Result<PreParsedFile, AppError>`
3. Consolidate the file path parsing, metadata reading, extension extraction, and parser invocation

#### Extract persist_parsed_metadata (Step 2)
4. Create `fn persist_parsed_metadata(tx: &Transaction, file_id: i64, parsed: &PreParsedFile) -> Result<(), AppError>`
5. Consolidate the UPDATE for stitch_count/color_count/width/height, INSERT for thread colors, INSERT for formats

#### Extract find_or_create_folder (Step 3)
6. Create `fn find_or_create_folder(tx: &Transaction, dir_path: &str, folders: &mut Vec<(i64, String)>) -> Result<i64, AppError>`
7. Consolidate the folder lookup/creation logic from mass_import and migrate_from_2stitch

#### Refactor callers (Step 4)
8. Update `import_files` to use the new helpers
9. Update `watcher_auto_import` to use the new helpers
10. Update `mass_import` to use the new helpers
11. Update `migrate_from_2stitch` to use the new helpers

### Verification
- `cargo test` — all existing tests pass
- Import files via each method — verify metadata is correctly stored
- Compare before/after behavior for all four code paths
