# Release Notes — StichMan 26.04-a1

**Release date:** 2026-03-16
**Type:** Alpha 1 — Sewing Pattern Management

## What's New

### Sewing Pattern Support (S1-S2)
- **File type discriminator** — files classified as `embroidery`, `sewing_pattern`, or `document`
- **PDF import** — scan, import, and parse PDF sewing patterns (page count, paper size, metadata)
- **Image import** — PNG, JPG, BMP files as reference images
- **Sewing metadata** — size range, skill level, language, format type, file source, purchase link
- **Status tracking** — not_started, planned, in_progress, completed, archived
- **Drag-and-drop** — drop files onto the app to import
- **Enhanced attachments** — structured types: pattern, instruction, cover image, measurement chart

### In-App Document Viewer (S3)
- **PDF viewer** — pdf.js rendering with page navigation, zoom/pan, fit modes
- **Page overview** — grid of page thumbnails for quick navigation
- **Bookmarks** — toggle, label, and navigate bookmarks per page
- **Notes** — add, edit, and delete per-page notes
- **Image viewer** — view PNG/JPG/SVG attachments in-app
- **Last-page memory** — resumes where you left off

### Print System (S4)
- **Direct printing** — print PDFs via OS print system (lpr/PowerShell)
- **Print preview** — full-page preview with page selection checkboxes
- **True-scale enforcement** — 100% scale by default, warning banner if changed
- **Page selection** — print specific pages or ranges
- **Tiling** — large-format detection, tile computation for A0/A1/A2 patterns
- **OCG layers** — detect and toggle PDF layers for multi-size patterns
- **Print from viewer** — Ctrl+P or toolbar button

### Project Management (S5)
- **Projects** — create, edit, duplicate, delete projects linked to patterns
- **Project details** — chosen size, fabric, modifications, sewing notes
- **Project duplication** — copy project without duplicating files
- **Collections** — group patterns into named collections (many-to-many)
- **Project list** — full-screen project browser with status filter and dashboard

### Data Safety & Portability (S6)
- **Soft delete / recycle bin** — deleted files recoverable, auto-purge after 30 days
- **Backup & restore** — ZIP backup of database + optional files
- **Library migration** — portable export with relative paths, import with path remapping
- **Re-link missing files** — detect and batch re-link moved files
- **Metadata export** — JSON and CSV export, JSON import with merge
- **Archive** — archive/unarchive (single + batch), hidden from default view

### Search & Filter Enhancement (S7)
- **Extended filters** — category, author, size range, plus existing skill/language/source/status
- **Sorting** — sort by name, date, author, category with direction toggle
- **Content-type badges** — color-coded file type indicators in file list
- **Print tracking** — mark_as_printed, recently printed queries

## Database

Schema upgraded from v8 to v13:
- v9: Sewing pattern metadata fields, FTS5 index update
- v10: PDF page_count/paper_size, enhanced attachments
- v11: Instruction bookmarks and notes
- v12: Projects, project details, collections
- v13: Soft delete (deleted_at column)

## Dependencies Added

- `pdfjs-dist` — PDF rendering in the frontend
- `lopdf` — PDF parsing in the backend
- `zip` — Backup archive creation/extraction
- `csv` — CSV metadata export

## Breaking Changes

None. Existing embroidery file data is preserved. All schema migrations are additive.
