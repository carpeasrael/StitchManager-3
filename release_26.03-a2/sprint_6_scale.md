# Sprint 6 — Scale

**Focus:** Support 50,000+ stitch files efficiently
**Issues:** #28
**Benefits from:** #36 (DB indexes), #37 (batch mutex), #39 (render optimization)

---

## Issue #28 — 50k+ File Performance

**Type:** Performance
**Effort:** XL

### Problem
Application must manage 50,000+ stitch files efficiently. File storage and database must be optimized.

### Affected Files
- `src-tauri/src/db/migrations.rs` — indexing, query optimization
- `src-tauri/src/db/queries.rs` — pagination support
- `src-tauri/src/commands/files.rs` — paginated queries
- `src-tauri/src/commands/folders.rs` — folder counts optimization
- `src-tauri/src/commands/scanner.rs` — bulk import optimization
- `src/components/FileList.ts` — virtual scroll at scale
- `src/state/AppState.ts` — state management for large datasets
- `src/services/FileService.ts` — paginated loading

### Implementation Plan

#### Database optimization (Step 1)
_Prerequisites: #36 indexes should be in place_

1. Add covering indexes for common queries:
   ```sql
   CREATE INDEX idx_files_folder_name ON embroidery_files(folder_id, filename);
   CREATE INDEX idx_files_format ON embroidery_files(id, file_format);
   CREATE INDEX idx_files_search ON embroidery_files(filename, description);
   ```
2. Implement `ANALYZE` on database open to update SQLite statistics
3. Consider FTS5 virtual table for full-text search on filename + description + tags

#### Paginated file loading (Step 2)
4. Modify `query_files_impl` to support LIMIT/OFFSET pagination:
   ```rust
   fn query_files_impl(db: &Connection, params: QueryParams) -> Result<(Vec<EmbroideryFile>, i64), AppError>
   ```
   Returns files + total count.
5. Add `page` and `page_size` parameters to the frontend `FileService.getFiles()` call
6. Default page size: 100 files

#### Lazy loading in FileList (Step 3)
7. FileList requests only visible page(s) of files
8. On scroll near bottom, load next page (infinite scroll pattern)
9. Keep a window of loaded pages in memory (e.g., current ± 2 pages)
10. Evict distant pages to control memory usage

#### AppState optimization (Step 4)
_Prerequisite: #39 AppState selective cloning_

11. Don't store all 50k files in AppState at once
12. Store only the current view's file IDs + metadata for visible files
13. Use a `Map<number, EmbroideryFile>` cache with LRU eviction

#### Bulk import optimization (Step 5)
14. Use SQLite `BEGIN IMMEDIATE` transactions for batch inserts
15. Batch INSERT statements (insert 100 files per transaction)
16. Disable synchronous during bulk import, re-enable after
17. Show progress with percentage and ETA

#### Thumbnail optimization (Step 6)
18. Lazy-generate thumbnails only when files scroll into view
19. Cache thumbnails on disk with a lookup index
20. Use a worker thread for thumbnail generation to avoid blocking UI

#### Search optimization (Step 7)
21. Implement FTS5 for text search:
    ```sql
    CREATE VIRTUAL TABLE files_fts USING fts5(filename, description, tags, content=embroidery_files, content_rowid=id);
    ```
22. Use FTS5 MATCH for search queries instead of LIKE
23. Keep FTS index in sync via triggers

#### Folder count caching (Step 8)
24. Cache folder file counts in a separate table or computed column
25. Update counts via triggers on INSERT/DELETE of embroidery_files
26. Eliminates per-request COUNT queries

### Benchmarks (Target)
| Operation | Target (50k files) |
|-----------|-------------------|
| App startup | < 2 seconds |
| Folder switch | < 200ms |
| Search | < 500ms |
| Scroll (continuous) | 60fps |
| File import (1000 files) | < 30 seconds |

### Verification
- Load test with 50,000 synthetic files
- Measure startup time, search time, scroll performance
- Memory usage should stay under 500MB
- No UI freezes during any operation
