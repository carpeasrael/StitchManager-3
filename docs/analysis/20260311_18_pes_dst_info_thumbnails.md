# Analysis: PES/DST Info Extraction and Thumbnail Display

**Date:** 2026-03-11
**Counter:** 18
**Short name:** pes_dst_info_thumbnails

---

## Problem Description

The application needs to read information and stitch patterns from .pes and .dst files, display thumbnail previews, and show extracted metadata (colors, stitch counts, etc.) in the UI. While the parsers themselves are comprehensive and all 128 tests pass, several critical gaps exist in the pipeline between parsing and display:

1. **Thumbnails are never generated or stored during file import** -- the `ThumbnailGenerator` service exists and works, but is never called from any command or import flow.
2. **Parsed thread colors are never persisted** to `file_thread_colors` during import, so the MetadataPanel color swatches section always shows "Keine Farbinformationen".
3. **File format records are never created** in `file_formats` during import, so format version info is unavailable in the UI.
4. **FileList does not display thumbnails** -- it shows a text format label instead of a thumbnail image.
5. **PES embedded thumbnail extraction produces raw monochrome pixels** (not a usable PNG), and the `get_thumbnail` command only reads a DB path (never generates on-demand).

---

## Affected Components

### Backend (Rust)

| File | Role | Issue |
|------|------|-------|
| `src-tauri/src/commands/scanner.rs` | `import_files`, `watcher_auto_import` | Parse metadata but never: (a) generate thumbnails, (b) insert colors into `file_thread_colors`, (c) insert format into `file_formats` |
| `src-tauri/src/commands/files.rs` | `get_thumbnail` | Only reads `thumbnail_path` from DB (always NULL), never generates on-demand |
| `src-tauri/src/services/thumbnail.rs` | `ThumbnailGenerator` | Fully implemented and tested but never instantiated or called from any command |
| `src-tauri/src/lib.rs` | App setup | No `ThumbnailGenerator` in managed state, no thumbnail cache dir configured |
| `src-tauri/src/parsers/pes.rs` | PES parser | `extract_thumbnail()` returns raw 48x38 monochrome pixel array (not a PNG); used by `ThumbnailGenerator` but that service is never invoked |
| `src-tauri/src/parsers/dst.rs` | DST parser | `extract_thumbnail()` returns `None` (correct -- DST has no embedded thumbnails); stitch-based rendering via `ThumbnailGenerator` would work if wired up |

### Frontend (TypeScript)

| File | Role | Issue |
|------|------|-------|
| `src/components/FileList.ts` | File list cards | Shows format text label (`getFormatLabel`) in `.file-card-thumb` div, never loads actual thumbnails |
| `src/components/MetadataPanel.ts` | Detail panel | Checks `file.thumbnailPath` (always null) and falls back to placeholder; color swatches section empty because `file_thread_colors` is never populated |
| `src/services/FileService.ts` | `getThumbnail()` | Calls `get_thumbnail` command which returns empty string (path is always NULL in DB) |

### Database

| Table | Issue |
|-------|-------|
| `embroidery_files.thumbnail_path` | Always NULL -- never set during import |
| `file_thread_colors` | Never populated during import (only by AI analysis) |
| `file_formats` | Never populated during import |

---

## Root Cause / Rationale

The root cause is an **incomplete wiring between the parsing layer and the persistence/display layers**. Specifically:

### 1. ThumbnailGenerator is orphaned
The `ThumbnailGenerator` was fully implemented (with tests) but the comment at line 1 says "Wired into Tauri commands in Sprint 7 (get_thumbnail command)" -- however this wiring was never completed. The generator:
- Is never instantiated in `lib.rs` setup
- Is never stored in Tauri managed state
- Is never called from `import_files`, `watcher_auto_import`, or `get_thumbnail`

### 2. Color persistence gap
The `import_files` function calls `parser.parse()` which returns `ParsedFileInfo` including a `colors: Vec<ParsedColor>` array. This metadata is used to update `embroidery_files` columns (stitch_count, color_count, etc.) but the individual color records are **discarded** -- they are never inserted into `file_thread_colors`.

### 3. Format records gap
Similarly, `import_files` never creates a `file_formats` row for the imported file, even though the format, version, filepath, and size are all known at import time.

### 4. FileList never requests thumbnails
The `FileList` component creates a `.file-card-thumb` div with text content only. It never calls `FileService.getThumbnail()` or sets an `<img>` element. Even if thumbnails were generated, they would not be visible in the file list.

### 5. get_thumbnail is read-only
The `get_thumbnail` command reads `thumbnail_path` from the database and returns its base64-encoded contents. Since the path is never set, it always returns an empty string. There is no fallback to generate on-demand.

---

## Proposed Approach

### Phase A: Wire up ThumbnailGenerator (Backend)

1. **Add ThumbnailGenerator to Tauri managed state** in `lib.rs`:
   - Create a `ThumbnailState` wrapper (similar to `DbState`) holding a `ThumbnailGenerator`
   - Initialize with cache dir at `{app_data_dir}/thumbnails/`
   - Register via `app.manage()`

2. **Generate thumbnails during import** in `import_files` and `watcher_auto_import`:
   - After inserting the file row and parsing metadata, call `ThumbnailGenerator::generate(file_id, &data, ext)`
   - Update `embroidery_files.thumbnail_path` with the returned path
   - This generates synthetic stitch-rendered thumbnails for all formats, and uses the embedded PES thumbnail as a fast alternative when available

3. **Make `get_thumbnail` generate on-demand** as fallback:
   - If `thumbnail_path` is NULL or the file doesn't exist on disk, read the original embroidery file and generate a thumbnail via `ThumbnailGenerator`
   - Update `thumbnail_path` in the database
   - Return the base64 data URI

### Phase B: Persist parsed colors and formats (Backend)

4. **Insert parsed colors into `file_thread_colors`** during import:
   - After parsing, iterate over `info.colors` and insert each with `sort_order`, `color_hex`, `color_name`, `brand`, `brand_code`, `is_ai=0`

5. **Insert format record into `file_formats`** during import:
   - Create one `file_formats` row per imported file with `format`, `format_version`, `filepath`, `file_size_bytes`, `parsed=1`

### Phase C: Display thumbnails in FileList (Frontend)

6. **Add thumbnail images to FileList cards**:
   - Change `.file-card-thumb` from a text div to contain an `<img>` element
   - Call `FileService.getThumbnail(file.id)` lazily (when card becomes visible)
   - Use the returned data URI as `img.src`
   - Keep the format label as fallback if thumbnail loading fails

### Phase D: Fix MetadataPanel thumbnail display

7. **Use `getThumbnail()` API instead of `convertFileSrc`**:
   - The current approach uses `convertFileSrc(file.thumbnailPath)` which requires the Tauri asset protocol. The `get_thumbnail` command already returns a `data:image/png;base64,...` URI which works directly as `img.src`.
   - Change MetadataPanel to call `FileService.getThumbnail(fileId)` and use the returned data URI.

### Phase E: Quality improvements

8. **PES thumbnail quality**: The current `extract_thumbnail` returns raw monochrome 48x38 pixels which `ThumbnailGenerator` scales to 192x192 black-and-white. Consider preferring the stitch-segment rendering (which has colors) over the embedded monochrome thumbnail for better visual quality.

9. **Thumbnail line thickness**: The current Bresenham line drawing produces 1-pixel-wide lines at 192x192, which can look thin for complex designs. Consider drawing 2px lines or using anti-aliased rendering.

10. **DST color assignment**: DST files have no embedded color data. The thumbnail generator uses a 10-color default palette cycling. This is acceptable but could be improved by allowing user color assignment.

---

## Current State Summary

### What works well
- **PES parser**: Comprehensive extraction of metadata (design name, dimensions, stitch/color/jump/trim counts, hoop size, PES v4+ extended metadata: category, author, keywords, comments). Supports PES v1 through v6. PEC palette fallback for older versions. Stitch segment extraction with color assignment.
- **DST parser**: Header parsing (design name, stitch count, color changes, extents/dimensions). Balanced-ternary stitch decoding. Jump/trim detection (consecutive jumps = trim). Stitch segment extraction.
- **ThumbnailGenerator**: Correct stitch-to-image rendering with Bresenham lines. Proper bounding box, uniform scaling, padding. Hex color parsing. Cache with invalidation. Both PES (embedded + stitch) and DST (stitch-only) paths work in tests.
- **All 128 Rust tests pass**.

### What is broken / missing
1. Thumbnails never generated (ThumbnailGenerator orphaned)
2. `thumbnail_path` always NULL in database
3. Parsed colors never stored in `file_thread_colors`
4. Format records never created in `file_formats`
5. FileList shows text, not thumbnail images
6. MetadataPanel thumbnail section shows placeholder (thumbnailPath always null)
7. MetadataPanel color swatches always empty (no colors in DB)
8. `get_thumbnail` command is read-only, no on-demand generation

### Impact
- Users cannot see visual previews of their embroidery files
- Thread color information is parsed but discarded, invisible in the UI
- Format version info is parsed but not queryable or displayable

---

## Solution Summary

Implemented 2026-03-11. All 8 gaps resolved across 5 phases:

**Phase A (Backend wiring):**
- Added `ThumbnailState` to `lib.rs` wrapping `ThumbnailGenerator` initialized with `{app_data_dir}/thumbnails/`
- `import_files` and `watcher_auto_import` generate thumbnails in a post-commit pass (outside the DB transaction to avoid holding the lock during I/O)
- `get_thumbnail` generates on-demand when cached path is missing, with `log::warn!` on errors

**Phase B (Color/format persistence):**
- Both import functions insert parsed thread colors into `file_thread_colors` with sort order, hex, name, brand, brand_code
- Both insert format records into `file_formats` with format, version, filepath, size
- All DB insert/update errors logged via `log::warn!`

**Phase C (FileList thumbnails):**
- FileList cards load thumbnail images via `FileService.getThumbnail()` with format label fallback
- In-memory `thumbCache` (Map) avoids re-fetching on virtual scroll with LRU eviction at 200 entries
- `isConnected` check prevents DOM mutations on detached elements

**Phase D (MetadataPanel fix):**
- Removed `convertFileSrc` dependency; uses `getThumbnail()` API with async loading
- Guards against stale async responses via `this.currentFile?.id === thumbFileId`
- AI analyze button always visible (not gated on `thumbnailPath`)

**Phase E (Quality):**
- CSS `.file-card-thumb-img` with `object-fit: contain` for proper thumbnail display
- Thumbnail cache cleared on folder/search change

**Files changed (9):** lib.rs, scanner.rs, files.rs, thumbnail.rs, FileList.ts, MetadataPanel.ts, FileService.ts (unchanged), components.css, analysis doc

**All 128 Rust tests pass. TypeScript + Vite build clean. 4/4 reviewers passed with zero findings.**
