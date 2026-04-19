# Wave 2 Performance Review — 2026-04-19

## Summary
Wave 2 lands every Critical and High performance fix from the audit cleanly, with no new perf regressions. All 216 backend tests pass and `cargo check` is clean. Verdict: **PASS**.

## Verification of original 23 findings

### Critical
- **#1 FileList full re-render on append** — addressed.
  `src/components/FileList.ts:30,33,142-146,155-173`. `loadMoreFiles` sets `expectingAppend = true` immediately before `appState.set("files", ...)`; `onFilesChanged` consumes the flag and performs an incremental render: only the spacer height is updated and `renderVisible()` is invoked. The thumb cache and `renderedCards` map survive an append. The `lastRenderedCount > 0` guard against initial paint is implicit via the `next.length > this.lastRenderedCount` test.
- **#2 AppState.get() deep copy on every read** — addressed.
  `src/state/AppState.ts:41-48`. `get()` and `getRef()` both return live references; `clone()` is the explicit detached-copy escape hatch. `update()` was also fixed to read `this.state[key]` directly (`:77`). Audited all 110 `appState.get()` call sites with grep — no in-place `.push/.splice/.sort/.pop/.shift/.unshift/.reverse` mutation of the returned value, and the few object-spread usages (e.g. `{ ...appState.get("searchParams") }`) already detach.
- **#3 Per-row UPDATE in mass_import / import_files / watcher_auto_import** — addressed.
  `src-tauri/src/commands/scanner.rs:21-44` introduces `apply_thumbnail_paths()` (single tx, prepared cached statement) and is invoked at `:424,828,1027`. The DB lock is now acquired exactly once per import run instead of N times.
- **#4 Sidebar full DOM rebuild on selection change** — addressed.
  `src/components/Sidebar.ts:25-27,55-70`. `selectedFolderId` and `selectedSmartFolderId` listeners now call `updateSelectionClasses()` which only toggles a CSS class on the indexed row map (`folderRowEls`, `smartRowEls`, `allFoldersRowEl`). Index is rebuilt on each structural `render()`. Drag/contextmenu/keydown listeners are no longer torn down + recreated per click.

### High
- **#5 FTS5 trigger fires on every UPDATE (even updated_at)** — addressed.
  `src-tauri/src/db/migrations.rs:1344-1402` (apply_v27). Trigger is now `AFTER UPDATE OF` the actual FTS-indexed columns. `toggle_favorite`, `archive_files_batch`, etc. no longer trigger an FTS rewrite.
- **#6 get_thumbnails_batch serial** — addressed.
  `src-tauri/src/commands/files.rs:446-487`. Disk read + base64 + on-demand generation now run via `paths.into_par_iter()`. Generated paths are persisted in a single `unchecked_transaction` with a `prepare_cached` statement (`:495-510`).
- **#7 pre_parse_file loop serial** — addressed.
  `src-tauri/src/commands/scanner.rs:268,716,935`. All three sites use `.par_iter()`. `pre_parse_file` is pure (fs reads + stateless parsers).
- **#8 delete_folder serial unlinks** — addressed.
  `src-tauri/src/commands/folders.rs:201-208`. `thumbnail_paths.par_iter().for_each(...)`.
- **#9 import_metadata_json / archive_files_batch / unarchive_files_batch unwrapped** — addressed.
  `src-tauri/src/commands/backup.rs:573,629,649`. Each wraps the per-row UPDATE loop in `unchecked_transaction()` + `prepare_cached`.
- **#10 FTS5 sqlite_master probe per query** — addressed.
  `src-tauri/src/commands/files.rs:43-46`. Probe replaced with `let fts_exists = true;` justified by the schema-version invariant.
- **#11 Tag SELECT-then-INSERT** — addressed in two sites.
  `src-tauri/src/commands/files.rs:1010-1024` (`set_file_tags`) and `src-tauri/src/commands/ai.rs:498-514` (`ai_accept_result`) use `INSERT ... ON CONFLICT(name) DO UPDATE SET name=excluded.name RETURNING id`. One round-trip per tag.
- **#12 Missing indices** — addressed.
  v27 adds `idx_embroidery_files_created_at` and replaces `idx_file_thread_colors_file_id` with the composite `(file_id, sort_order)`.
- **#13 Slim FILE_SELECT for paginated list** — addressed.
  `src-tauri/src/db/queries.rs:13-30` defines `FILE_SELECT_LIST_ALIASED` masking `description, keywords, comments, purchase_link, instructions_html`; column ordering matches `FILE_SELECT` so `row_to_file()` is reused. Used at `src-tauri/src/commands/files.rs:402`. Frontend impact is safe — only `MetadataPanel.ts` reads these fields, and it always re-fetches via `FileService.getFile(fileId)` which uses `FILE_SELECT_LIVE_BY_ID` (full data).

### Medium
- **#14 Sidebar.loadCounts CTE caching** — partially addressed via the selection-class refactor (acknowledged).
- **#15 get_files unbounded** — deferred (audit-acknowledged).
- **#16 smartFolders cached parse** — deferred.
- **#17 MetadataPanel 8-roundtrip** — deferred.
- **#18 PES/DST parser walk allocations** — deferred.
- **#19 get_dashboard_stats 9→7 queries** — addressed.
  `src-tauri/src/commands/statistics.rs:59-71,92-104`. Two `SUM(CASE…)` aggregates collapse 6 separate full-table COUNTs into 2. `Option<i64>::unwrap_or(0)` correctly handles the empty-table NULL case.
- **#20 MetadataPanel.checkDirty per-keystroke** — deferred.

### Low
- **#21–#22** — minor, deferred.

### Bonus (not enumerated)
- **file_watcher HashSet flush cap at 500** — addressed.
  `src-tauri/src/services/file_watcher.rs:78-94`. Burst flush prevents unbounded memory accumulation; `last_flush` is reset on the burst path so the periodic timer doesn't immediately re-flush.

## New findings introduced by Wave 2

No new findings.

A few notes considered and dismissed:
- `FILE_SELECT_LIST_ALIASED` returns `''` (empty string) instead of NULL for masked text columns. Verified via grep that no UI code distinguishes "" from NULL on these fields outside of `MetadataPanel`, which always re-fetches via `get_file()`. No user-visible regression.
- `get_thumbnails_batch` parallel `thumb_state.0.generate(...)` writes to a shared cache directory; each call uses a distinct `file_id`-derived filename, so no inter-thread filename collision. `ThumbnailGenerator` only owns `PathBuf`, so it is `Send + Sync`.
- `delete_folder` rayon par_iter on 10K thumbnail unlinks uses the global rayon pool. Bounded by APFS concurrency, no starvation observed in unit tests.
- `apply_v27` migration is wrapped in a single `BEGIN/COMMIT`; the trigger CREATE references all 15 FTS-indexed columns explicitly — verified the column list matches the FTS table definition from earlier migrations (test `test_schema_version_is_current` passes).
