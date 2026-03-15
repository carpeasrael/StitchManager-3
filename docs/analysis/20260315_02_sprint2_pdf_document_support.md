# Analysis: Sprint 2 — PDF & Document File Support

**Date:** 2026-03-15
**Sprint:** S2 (release 26.04-a1)
**Issues:** S2-01, S2-02, S2-03, S2-04, S2-05, S2-06
**Requirements:** UR-002, UR-003, UR-007, UR-008, UR-009, UR-011, UR-012

---

## Problem Description

StitchManager currently only imports embroidery files (PES/DST/JEF/VP3). The sewing pattern management requirements demand support for PDF files (the primary sewing pattern format) and common image formats (PNG, JPG, SVG, BMP) as cover images, instruction documents, and measurement charts. Additionally, the attachment system needs structured type classification, drag-and-drop import must be added, and the file watcher must detect new document formats.

---

## Affected Components

### Backend (Rust)
| File | Impact |
|------|--------|
| `src-tauri/src/commands/scanner.rs` | Extend `SUPPORTED_EXTENSIONS`, `is_embroidery_file()` → `is_supported_file()`, adapt `pre_parse_file()` for new formats, set `file_type` on import |
| `src-tauri/src/services/file_watcher.rs` | Extend `SUPPORTED_EXTENSIONS` for PDF/image detection |
| `src-tauri/src/parsers/mod.rs` | Register new parsers in `get_parser()` registry |
| `src-tauri/src/parsers/pdf.rs` | **New file:** PDF metadata extraction via `lopdf` crate |
| `src-tauri/src/parsers/image.rs` | **New file:** Image metadata extraction via existing `image` crate |
| `src-tauri/src/services/thumbnail.rs` | Handle image format thumbnails (resize from source) |
| `src-tauri/src/db/migrations.rs` | Migration v10: add `display_name`, `sort_order` to `file_attachments`; add `page_count`, `paper_size` to `embroidery_files` |
| `src-tauri/src/db/models.rs` | Extend `EmbroideryFile` (+2), `FileAttachment` (+2) |
| `src-tauri/src/db/queries.rs` | Update FILE_SELECT (39 cols), row_to_file |
| `src-tauri/src/commands/files.rs` | Update `attach_file` with type selector; extend `update_file` for new fields |
| `src-tauri/src/lib.rs` | No new commands needed |
| `src-tauri/Cargo.toml` | Add `lopdf` dependency |

### Frontend (TypeScript)
| File | Impact |
|------|--------|
| `src/types/index.ts` | Extend `EmbroideryFile` (+2), `FileAttachment` (+2) |
| `src/components/MetadataPanel.ts` | Attachment type selector, display_name editing, type badges, attachment list reordering |
| `src/components/FileList.ts` | Add drag-and-drop handlers, drop zone overlay |
| `src/components/Sidebar.ts` | Add folder drop targets |
| `src/main.ts` | Wire drop events to import pipeline |
| `src/styles/components.css` | Drop zone styles, attachment type badges |

---

## Root Cause / Rationale

1. **Extension gate:** `is_embroidery_file()` in both `scanner.rs` (line 13) and `file_watcher.rs` (line 9) hardcodes only `["pes", "dst", "jef", "vp3"]` — blocks all other formats.
2. **Parser registry:** `get_parser()` in `parsers/mod.rs` only matches 4 embroidery formats — new formats get `None` parser and `parsed=false`.
3. **No file_type assignment:** Scanner INSERT doesn't set `file_type` — defaults to `'embroidery'`. PDFs/images need `file_type='sewing_pattern'`.
4. **Attachment types:** `attach_file()` always uses hardcoded type `"other"` from frontend — no type selector exists.
5. **No drag-and-drop:** No `dragover`/`drop` event handlers anywhere in the frontend.

---

## Proposed Approach

### S2-01: PDF File Format Recognition and Import

**New dependency:** Add `lopdf = "0.34"` to Cargo.toml for PDF metadata extraction.

**New parser:** `src-tauri/src/parsers/pdf.rs`
- Implement `EmbroideryParser` trait (reuse same interface)
- `parse()`: Extract page count, paper size (from first page MediaBox), title (from PDF info dictionary)
- `extract_thumbnail()`: Return `None` (PDF page rendering requires heavy native deps like pdfium — defer to Sprint 3 when pdf.js is available)
- `extract_stitch_segments()`: Return empty vec (not applicable)
- Store `page_count` and `paper_size` in new columns on `embroidery_files`

**Scanner changes (`scanner.rs`):**
- Rename `SUPPORTED_EXTENSIONS` → split into `EMBROIDERY_EXTENSIONS` and `DOCUMENT_EXTENSIONS`
- New `is_supported_file()` replaces `is_embroidery_file()` — checks both lists
- In `pre_parse_file()` / `persist_parsed_metadata()`: detect extension category, set `file_type='sewing_pattern'` for PDF/image files
- In file INSERT: set `file_type` based on extension

**Schema migration v10:**
- `ALTER TABLE embroidery_files ADD COLUMN page_count INTEGER;`
- `ALTER TABLE embroidery_files ADD COLUMN paper_size TEXT;`

### S2-02: Support Additional Image Formats

**New parser:** `src-tauri/src/parsers/image_parser.rs`
- Handle `.png`, `.jpg`, `.jpeg`, `.bmp`, `.svg`
- `parse()`: Use existing `image` crate to get dimensions → `width_mm` and `height_mm` (at 96 DPI default), file size
- `extract_thumbnail()`: Resize source image to thumbnail size (192x192) using `image` crate
- `extract_stitch_segments()`: Return empty vec
- SVG: basic dimension detection from viewBox attribute (width/height)

**Thumbnail generation:** The existing `thumbnail.rs` fallback path already handles images when `extract_thumbnail()` returns data. Image files can return the resized image directly.

### S2-03: Enhance file_attachments with Structured Types

**Schema migration v10 (combined with S2-01):**
- `ALTER TABLE file_attachments ADD COLUMN display_name TEXT;`
- `ALTER TABLE file_attachments ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;`

**Allowed attachment types:** `pattern`, `instruction`, `cover_image`, `measurement_chart`, `fabric_requirements`, `notes`, `license`, `other`

**Backend changes:**
- Extend `FileAttachment` model with `display_name` and `sort_order`
- Update `get_attachments()` to ORDER BY `sort_order, created_at`
- Update `attach_file()` to accept optional `display_name` parameter

**Frontend changes (MetadataPanel):**
- Attachment type selector dropdown when adding attachments
- Type badge display next to each attachment (color-coded)
- Display name shown instead of raw filename when set
- Reorder via sort_order (drag or up/down buttons)

### S2-04: Drag-and-Drop Import

**Implementation in `src/main.ts`:**
- Add `dragover`/`drop`/`dragenter`/`dragleave` listeners on document body
- On dragover: show full-screen drop zone overlay with visual feedback
- On drop: extract file paths from DataTransfer
- Determine target: if dropped on folder in sidebar → import to that folder; if dropped on file card → add as attachment; otherwise → import to selected folder

**Drop zone overlay:**
- Semi-transparent backdrop with centered icon/text
- CSS class `.drop-zone-active` toggled on drag enter/leave
- Different states: "Import in Ordner" vs "Als Anhang hinzufügen"

**Tauri consideration:** In Tauri v2 webview, `DataTransfer.files` on drop events provides `File` objects. File paths can be extracted via `webkitRelativePath` or Tauri's `onDragDropEvent` from `@tauri-apps/api/webviewWindow`. Use the Tauri drag-drop event API which provides absolute file paths.

### S2-05: Multi-File Pattern Record Creation

**MetadataPanel enhancements:**
- Show all attachments with type badges in a list
- "Add Files" button opens type-aware file dialog
- Attachment list shows: type icon, display name (or filename), file size, delete button
- Reorder via sort_order

**No new backend commands needed** — existing `attach_file()` with the type parameter extension is sufficient.

### S2-06: Enhanced File Watcher

**file_watcher.rs changes:**
- Extend `SUPPORTED_EXTENSIONS` to include `["pdf", "png", "jpg", "jpeg", "bmp", "svg"]`
- The rest of the watcher pipeline (debounce, event emission, auto-import) works unchanged
- Scanner's `watcher_auto_import` will handle new formats through the extended `is_supported_file()` check

---

## Implementation Order

1. **Migration v10** + Rust models + queries (S2-01 + S2-03 schema)
2. **PDF parser** + image parser + register in mod.rs (S2-01 + S2-02)
3. **Scanner extension** — new extension lists, file_type assignment (S2-01 + S2-02)
4. **File watcher extension** (S2-06)
5. **Attachment enhancement** — backend type/display_name/sort_order (S2-03)
6. **Frontend types** + MetadataPanel attachment UI (S2-03 + S2-05)
7. **Drag-and-drop** — main.ts + CSS (S2-04)
8. **Thumbnail generation** for images (S2-02)

---

## Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| PDF parsing library | `lopdf` | Pure Rust, lightweight, metadata extraction (page count, MediaBox, info dict). No native deps. |
| PDF thumbnail generation | Deferred to S3 | Requires pdf.js (frontend) or pdfium (heavy native dep). Placeholder icon for now. |
| Image thumbnail | `image` crate (existing) | Already in Cargo.toml, handles PNG/JPG/BMP resize natively. |
| SVG handling | Basic dimension parsing | Full SVG rendering is complex; extract viewBox dimensions, use placeholder thumbnail. |
| Drag-and-drop API | Tauri `onDragDropEvent` | Provides absolute file paths directly, works reliably in Tauri v2 webview. |
| Attachment type enforcement | Soft enum (no DB constraint) | Frontend dropdown provides valid values; backend accepts any string for extensibility. |

---

## Verification Plan

1. `cargo check` — compile with new `lopdf` dependency
2. `cargo test` — migration tests, parser tests for PDF/image
3. `npm run build` — TypeScript type checking
4. Manual: import a PDF file → verify metadata extracted, file_type='sewing_pattern', page_count populated
5. Manual: import PNG/JPG → verify thumbnail generated, dimensions extracted
6. Manual: drag files from file manager → verify drop zone appears, files import
7. Manual: add attachment with type selector → verify type badge displays
