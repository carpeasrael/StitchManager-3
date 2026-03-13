# Sprint 8 — Scale & Performance

**Focus:** Support 50,000+ stitch files efficiently
**Issues:** #28
**Prerequisites:** DB indexes (#36), batch mutex (#37), render optimization (#39) — all completed in Sprint 2

---

## Issue #28 — 50k+ File Performance

**Type:** Performance
**Effort:** XL

### Problem
Application must manage 50,000+ stitch files efficiently. File storage, database queries, and UI rendering must be optimized to maintain responsiveness at scale.

### Affected Components
- `src-tauri/src/db/migrations.rs` — indexes, FTS5
- `src-tauri/src/db/queries.rs` — pagination
- `src-tauri/src/commands/files.rs` — paginated queries
- `src-tauri/src/commands/folders.rs` — cached folder counts
- `src-tauri/src/commands/scanner.rs` — bulk import optimization
- `src/components/FileList.ts` — infinite scroll, lazy loading
- `src/state/AppState.ts` — LRU cache for file data
- `src/services/FileService.ts` — paginated API

### Proposed Approach

#### Phase A: Database Layer (Steps 1–3)

**Step 1 — Covering indexes**
1. Add composite indexes for common query patterns:
   ```sql
   CREATE INDEX IF NOT EXISTS idx_files_folder_name ON embroidery_files(folder_id, filename);
   CREATE INDEX IF NOT EXISTS idx_files_format ON embroidery_files(id, file_format);
   CREATE INDEX IF NOT EXISTS idx_files_search ON embroidery_files(filename, description);
   ```
2. Run `ANALYZE` on database open to keep SQLite query planner informed

**Step 2 — FTS5 full-text search**
3. Create FTS5 virtual table for text search:
   ```sql
   CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
     filename, description, tags,
     content=embroidery_files, content_rowid=id
   );
   ```
4. Add triggers to keep FTS index in sync on INSERT/UPDATE/DELETE
5. Replace `LIKE '%term%'` queries with `FTS5 MATCH` for search

**Step 3 — Folder count caching**
6. Add `file_count INTEGER DEFAULT 0` column to `folders` table (or create a cache table)
7. Add triggers on `embroidery_files` INSERT/DELETE to maintain counts
8. Remove per-request `COUNT(*)` queries from folder listing

#### Phase B: Paginated Loading (Steps 4–5)

**Step 4 — Backend pagination**
9. Modify `query_files_impl` to accept `page` and `page_size` parameters
10. Return `(Vec<EmbroideryFile>, total_count: i64)` for frontend pagination state
11. Default page size: 100 files

**Step 5 — Frontend pagination**
12. Update `FileService.getFiles()` to pass `page` and `page_size`
13. `FileList` requests only visible page(s) of files
14. Implement infinite scroll: load next page when scrolled near bottom
15. Keep a sliding window of loaded pages in memory (current ± 2)
16. Evict distant pages to control memory usage

#### Phase C: State & Memory (Steps 6–7)

**Step 6 — AppState optimization**
17. Replace full file array with `Map<number, EmbroideryFile>` LRU cache
18. Store only current view's file IDs + metadata for visible files
19. Cap cache at ~2000 files, evict least-recently-used beyond that

**Step 7 — Lazy thumbnails**
20. Generate/load thumbnails only when files scroll into view
21. Add disk-based thumbnail cache with lookup index
22. Use background thread for thumbnail generation

#### Phase D: Bulk Import (Step 8)

**Step 8 — Import optimization**
23. Use `BEGIN IMMEDIATE` transactions for batch inserts
24. Batch 100 files per transaction
25. Disable `synchronous` during bulk import, re-enable after
26. Progress display with percentage and ETA

### Benchmarks (Target)

| Operation | Target (50k files) |
|-----------|-------------------|
| App startup | < 2s |
| Folder switch | < 200ms |
| Search | < 500ms |
| Scroll (continuous) | 60fps |
| File import (1000 files) | < 30s |
| Memory usage | < 500MB |

### Verification
- Create synthetic test dataset with 50,000 files
- Measure startup, search, scroll, and import performance against targets
- Monitor memory usage throughout a typical workflow
- Verify no UI freezes during any operation
- Stress test with rapid folder switching and search queries
