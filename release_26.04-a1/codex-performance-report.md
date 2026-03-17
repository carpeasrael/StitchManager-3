# Performance Test Report — Codex Reviewer Agent
**Date:** 2026-03-17
**Release:** 26.04-a1

## Summary
- Tests executed: 15
- Passed: 12
- Findings: 3 (Critical: 0, High: 0, Medium: 2, Low: 1)

## Test Results

### PT-01 Virtual scroll: DOM node count
- **Status:** PASS
- **File(s):** `src/components/FileList.ts:120-131`
- **Description:** `calculateVisibleRange` computes visible cards as `Math.ceil(containerHeight / CARD_HEIGHT)` plus `BUFFER=5` on each side. For 10,000 files, the spacer is a single div and only `visibleCount + 2*BUFFER` card elements exist in the DOM.

### PT-02 Virtual scroll: Scroll performance
- **Status:** PASS
- **File(s):** `src/components/FileList.ts:105-118`
- **Description:** Scroll handler uses `requestAnimationFrame` with a `scrollRafPending` guard to prevent stacking. Only re-renders when visible range changes. Efficient card recycling via `renderedCards` Map with index-based add/remove.

### PT-03 DB query: FTS5 search performance
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs:43-81`
- **Description:** FTS5 MATCH query with `rowid IN (SELECT rowid FROM files_fts ...)` is O(log n). Index on `embroidery_files(folder_id)` and `embroidery_files(name)` support common query patterns. `ANALYZE` run after migrations.

### PT-04 DB query: Advanced search with multiple filters
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs:96-295`
- **Description:** Parameterized conditions assembled into a single query. Correlated subqueries for tags and format use EXISTS with indexed joins. No N+1 patterns.

### PT-05 Batch rename: 1000 files
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/batch.rs:110-284`
- **Description:** Phase 1 loads all metadata in single DB lock. Phase 2 performs FS renames without lock. Phase 3 commits all DB updates in single transaction. No per-file lock acquisition.

### PT-06 Batch organize: 1000 files
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/batch.rs:287-491`
- **Description:** Same three-phase design as batch rename. Single transaction for all DB updates.

### PT-07 File import: scan_directory with 500 files
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/scanner.rs:379-616`
- **Description:** `mass_import` pre-parses all files outside DB lock, then uses single transaction for all DB inserts. Thumbnail generation happens after DB lock is dropped. Progress events throttled to every 10 files.

### PT-08 Thumbnail generation: Stitch rendering
- **Status:** PASS
- **File(s):** `src-tauri/src/services/thumbnail.rs`
- **Description:** 192x192 target size limits memory. Bresenham line drawing is O(n) per segment. Bounding box computed in single pass. Cache check (`get_cached`) avoids re-generation.

### PT-09 Memory: App idle after loading files
- **Status:** FINDING
- **Severity:** Medium
- **File(s):** `src-tauri/src/services/thumbnail.rs`, `src-tauri/src/commands/scanner.rs`
- **Description:** The thumbnail cache (`ThumbnailGenerator`) uses filesystem caching with no eviction policy. For a library with 10,000+ files, the thumbnail directory grows unboundedly. While the actual PNG files are on disk (not in memory), there is no cache size limit or LRU eviction as documented in the test plan (PT-14). Additionally, `import_files` and `mass_import` read entire file contents into memory (`std::fs::read`) for parsing and thumbnail generation. For files up to 100MB each, this could cause significant memory spikes during batch import.
- **Evidence:** `ThumbnailGenerator::get_cached` at `thumbnail.rs:106` only checks file existence — no size tracking or eviction. `pre_parse_file` at `scanner.rs:55` reads the entire file into memory: `std::fs::read(path).ok()`. Import loops hold the entire `Vec<PreParsedFile>` in memory simultaneously.
- **Proposed Fix:** (1) Add a maximum cache size setting with LRU eviction for the thumbnail directory. (2) Consider streaming or chunked reading for large embroidery files during import rather than reading the entire file into memory at once.

### PT-10 Memory: Subscription cleanup on HMR
- **Status:** PASS
- **File(s):** `src/main.ts`, `src/components/Component.ts`
- **Description:** `Component` base class tracks subscriptions and cleans them up in `destroy()`. `main.ts` teardown logic cleans all global subscriptions on HMR. Components properly override `destroy()` to call `super.destroy()`.

### PT-11 DB lock contention: Concurrent read + write
- **Status:** PASS
- **File(s):** `src-tauri/src/error.rs:30-32`, `src-tauri/src/db/migrations.rs:9`
- **Description:** WAL mode enabled (`PRAGMA journal_mode=WAL`). `busy_timeout=5000ms`. Single Mutex-wrapped connection accessed via `lock_db()`. Batch operations drop DB lock before I/O-heavy thumbnail generation, then re-acquire briefly for updates. No deadlock risk (single lock).

### PT-12 Search debounce: Rapid typing
- **Status:** PASS
- **File(s):** `src/components/SearchBar.ts:113`
- **Description:** `this.debounceTimer = setTimeout(() => { ... }, 300)` with `clearTimeout` on each keystroke. Only one backend call per 300ms window.

### PT-13 File watcher: Rapid event coalescing
- **Status:** PASS
- **File(s):** `src-tauri/src/services/file_watcher.rs:52-123`
- **Description:** `HashSet<String>` accumulates events. `drain()` on flush. `recv_timeout(DEBOUNCE_MS)` ensures coalescing within 500ms windows.

### PT-14 Thumbnail cache: LRU eviction
- **Status:** FINDING
- **Severity:** Medium
- **File(s):** `src-tauri/src/services/thumbnail.rs`
- **Description:** No cache eviction mechanism exists. The test plan specifies LRU eviction at `THUMB_CACHE_MAX`, but no such constant or eviction logic was found. The cache grows indefinitely.
- **Evidence:** `ThumbnailGenerator` struct has only a `cache_dir: PathBuf` field. No `HashMap`, no size counter, no eviction code. `get_cached` checks file existence only.
- **Proposed Fix:** Implement a configurable cache size limit. On `generate()`, check total cache size and evict least-recently-accessed entries when the limit is exceeded.

### PT-15 Startup: App cold start
- **Status:** FINDING
- **Severity:** Low
- **File(s):** `src-tauri/src/lib.rs:18-110`
- **Description:** On startup, the app runs all 21 migrations (each guarded by version check), executes `ANALYZE`, and attempts to auto-start the file watcher plus USB monitor. For a fresh database this is fast, but for an existing database with 10,000+ files, the `ANALYZE` command at `migrations.rs:133` could add several hundred milliseconds to startup time.
- **Evidence:** `let _ = conn.execute_batch("ANALYZE;")` at migrations.rs:133 runs on every startup regardless of whether any migration was applied. For large tables, ANALYZE can be slow.
- **Proposed Fix:** Only run `ANALYZE` when a migration was actually applied (i.e., when `current < CURRENT_VERSION`), rather than unconditionally on every startup.
