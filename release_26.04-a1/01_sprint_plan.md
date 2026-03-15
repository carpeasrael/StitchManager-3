# Sprint Plan: Release 26.04-a1 — Sewing Pattern Management

**Date:** 2026-03-15
**Release target:** 26.04-a1 (alpha 1)
**Base:** StitchManager 26.03 (schema v8)
**Scope:** UR-001 – UR-074 (Sewing Pattern Management Requirements)

---

## Sprint Overview

| Sprint | Name | Focus | Key URs | Est. Issues |
|--------|------|-------|---------|-------------|
| S1 | Data Model & Metadata | Schema extension, new fields, status tracking | UR-008, UR-014, UR-018 | 5 |
| S2 | PDF & Document Support | PDF import, attachment reclassification, drag-and-drop | UR-002, UR-003, UR-008, UR-009, UR-011 | 6 |
| S3 | Document Viewer | In-app PDF viewer, navigation, zoom/pan | UR-020–025, UR-031–035 | 7 |
| S4 | Print System | Direct printing, true-scale, print preview | UR-036–050 | 8 |
| S5 | Project Management | Project entries, project data, duplication | UR-017, UR-019, UR-052–054 | 5 |
| S6 | Data Safety & Portability | Recycle bin, backup/restore, migration, re-linking | UR-057, UR-058, UR-060–063 | 6 |
| S7 | Search & Filter Enhancement | Extended filters, sorting, usability polish | UR-028, UR-029, UR-065, UR-066 | 4 |
| S8 | Integration & Stabilization | End-to-end flows, acceptance criteria, polish | AE-001–008 | 4 |

**Total estimated issues: ~45**

---

## Sprint 1: Data Model & Metadata Extension

**Goal:** Extend the database schema and backend models to support sewing patterns alongside embroidery files. Add new metadata fields required by the sewing pattern domain.

### Issues

#### S1-01: Add `file_type` discriminator to data model
**URs:** UR-001, UR-004
- Add `file_type TEXT NOT NULL DEFAULT 'embroidery'` column to `embroidery_files` table
- Supported values: `embroidery`, `sewing_pattern`
- Update `db/models.rs`, `db/queries.rs`, `db/migrations.rs` (schema v9)
- Update `FileService` and `FolderService` to pass through file_type
- Update frontend `types/index.ts` with `FileType` enum
- **DoD:** Migration runs cleanly, existing data defaults to `embroidery`, new type selectable

#### S1-02: Add sewing pattern metadata fields
**URs:** UR-014
- Add columns to `embroidery_files`: `size_range`, `skill_level`, `language`, `format_type`, `file_source`, `purchase_link`
- `skill_level`: TEXT, values: `beginner`, `intermediate`, `advanced`, `expert` (nullable)
- `language`: TEXT, ISO 639-1 codes (nullable)
- `size_range`: TEXT, free-form e.g. "34-48" or "XS-XL" (nullable)
- `format_type`: TEXT, e.g. "PDF-A4", "PDF-A0", "projector" (nullable)
- `file_source`: TEXT, e.g. "Makerist", "Burda", "self-drafted" (nullable)
- `purchase_link`: TEXT, URL (nullable)
- Update Rust models, queries, row mappers
- Update FTS5 index to include new searchable fields (`language`, `file_source`)
- **DoD:** Fields stored and retrievable via existing file CRUD commands

#### S1-03: Add status tracking
**URs:** UR-018
- Add `status TEXT NOT NULL DEFAULT 'none'` to `embroidery_files`
- Values: `none`, `not_started`, `planned`, `in_progress`, `completed`, `archived`
- Add status update command in `commands/files.rs`
- Update models and queries
- **DoD:** Status can be set and queried per file

#### S1-04: Update MetadataPanel for new fields
**URs:** UR-013, UR-014, UR-018
- Add form sections for: size_range, skill_level (dropdown), language (dropdown), format_type, file_source, purchase_link (as clickable link), status (dropdown)
- Fields should show/hide based on `file_type` (some are sewing-specific, some shared)
- Add CSS for new form fields
- **DoD:** All new fields editable in MetadataPanel, dirty tracking works

#### S1-05: Update FTS5 and search index
**URs:** UR-027
- Rebuild FTS5 virtual table triggers to include new fields
- Add `language`, `file_source`, `size_range` to search index
- Update search result highlighting if applicable
- **DoD:** Free-text search finds patterns by new fields

---

## Sprint 2: PDF & Document File Support

**Goal:** Enable import, storage, and basic handling of PDF files and common image/document formats as first-class sewing pattern files. Enhance the attachment system for structured multi-file records.

### Issues

#### S2-01: PDF file format recognition and import
**URs:** UR-008
- Extend scanner to recognize `.pdf` files as sewing pattern candidates
- Add PDF metadata extraction (page count, paper size, title) using a Rust PDF library (e.g., `lopdf` or `pdf`)
- Create `PdfParser` implementing a new or adapted parser trait
- Store PDF-specific metadata: `page_count`, `paper_size` in file record or dedicated columns
- Generate thumbnail from first page (via `pdfium` or image extraction)
- **DoD:** PDF files importable via scan/import, metadata extracted, thumbnail generated

#### S2-02: Support additional file formats
**URs:** UR-008
- Extend scanner to recognize: `.png`, `.jpg`, `.jpeg`, `.svg`, `.bmp` as pattern/instruction images
- Basic metadata extraction (dimensions, file size)
- Thumbnail generation for image files (resize to standard thumbnail size)
- **DoD:** Image files importable alongside PDFs

#### S2-03: Enhance file_attachments with structured types
**URs:** UR-002, UR-003, UR-009
- Extend `file_attachments.attachment_type` to support: `pattern`, `instruction`, `cover_image`, `measurement_chart`, `fabric_requirements`, `notes`, `other`
- Add `display_name TEXT` and `sort_order INTEGER` to attachments
- Update attachment CRUD commands
- Update frontend attachment management UI in MetadataPanel
- Add attachment type selector when adding files
- **DoD:** Attachments can be classified by type, displayed by type in UI

#### S2-04: Drag-and-drop import
**URs:** UR-011
- Add `dragover`/`drop` event handlers to the main app area
- Support dropping files onto the file list area to trigger import
- Support dropping onto a folder in the sidebar to import into that folder
- Support dropping onto an existing file record to add as attachment
- Show drop zone overlay with visual feedback
- **DoD:** Files can be imported by dragging from OS file manager

#### S2-05: Multi-file pattern record creation
**URs:** UR-002, UR-009
- Add "Add Files" button/action to MetadataPanel for existing records
- Allow associating multiple files during initial import
- Show all associated files in MetadataPanel as a file list with type badges
- Allow reordering and removing attached files
- **DoD:** A single pattern record can hold pattern PDF + instruction PDF + cover image etc.

#### S2-06: Enhanced file watcher for new formats
**URs:** UR-007
- Extend `file_watcher.rs` to detect PDF and image file changes
- Auto-import new PDFs/images found in watched directory
- Maintain separate handling for embroidery vs. document files
- **DoD:** New PDFs appearing in library_root are auto-detected

---

## Sprint 3: In-App Document Viewer

**Goal:** Provide an in-app PDF viewer for reading sewing pattern documents and instructions without requiring an external application.

### Issues

#### S3-01: PDF viewer component (frontend)
**URs:** UR-021, UR-022, UR-031
- Integrate `pdf.js` (Mozilla) as the rendering engine
- Create `DocumentViewer` component extending `Component` base
- Render PDF pages to canvas elements
- Load PDF from local file path (via Tauri file access)
- Display in a dedicated panel or overlay dialog
- **DoD:** PDF files open and render correctly in-app

#### S3-02: Page navigation
**URs:** UR-023, UR-035
- Page-by-page navigation (prev/next, page number input)
- Multi-page overview mode (grid of page thumbnails)
- Toggle between single-page and overview
- Page count display in toolbar/header
- Keyboard shortcuts: Left/Right arrows, Home/End
- **DoD:** Users can navigate through PDF pages efficiently

#### S3-03: Zoom and pan controls
**URs:** UR-033
- Zoom in/out with buttons and keyboard (Ctrl++/Ctrl+-)
- Fit-to-width, fit-to-page modes
- Mouse wheel zoom
- Click-and-drag pan when zoomed
- Zoom level indicator
- **DoD:** Users can zoom and pan pattern documents freely

#### S3-04: Document properties display
**URs:** UR-034
- Show page count, paper size (A4/Letter/A0), document title
- Display in document viewer header or info panel
- Detect paper size from PDF page dimensions
- **DoD:** Document metadata visible during viewing

#### S3-05: Instruction bookmarks and notes
**URs:** UR-024, UR-025
- Add `instruction_bookmarks` table: `id`, `file_id`, `page_number`, `label`, `created_at`
- Add `instruction_notes` table: `id`, `file_id`, `page_number`, `note_text`, `created_at`, `updated_at`
- Bookmark toggle per page in viewer
- Notes panel per page (add/edit/delete)
- Bookmark list in sidebar for quick navigation
- **DoD:** Users can bookmark pages and add notes to specific pages

#### S3-06: Image viewer for non-PDF attachments
**URs:** UR-032
- Image viewer component for PNG/JPG/SVG cover images
- Zoom and pan support
- Navigation between multiple images
- **DoD:** Image attachments viewable in-app

#### S3-07: Viewer integration with main UI
**URs:** UR-004, UR-066
- "Open" button on pattern records and attachments
- Context-aware: opens PDF viewer for PDFs, image viewer for images
- Return-to-library navigation
- Remember last viewed page per document
- **DoD:** Seamless transition between library and document viewing

---

## Sprint 4: Print System

**Goal:** Enable direct printing of sewing patterns from within the app, with true-scale control, print preview, and page selection.

### Issues

#### S4-01: Print service backend
**URs:** UR-036, UR-037
- Create `PrintService` (Rust) using OS print dialog via Tauri shell plugin or `lpr`/`print` commands
- Alternatively: generate print-ready PDF and open system print dialog
- Investigate Tauri v2 printing capabilities / plugin landscape
- Create `commands/print.rs` with `print_file` command
- **DoD:** Backend can send a PDF to the OS print system

#### S4-02: Print preview component
**URs:** UR-038, UR-048
- Create `PrintPreviewDialog` component
- Render PDF pages at print resolution in preview
- Show measurement calibration square if present in source
- Display paper size grid overlay
- Page selection checkboxes
- **DoD:** Users see accurate preview before printing

#### S4-03: True-scale printing enforcement
**URs:** UR-041, UR-042, UR-043, UR-049
- Default print settings: no scaling (100%)
- Disable fit-to-page by default
- Add scale warning banner when user changes settings that affect scale
- Validate PDF page dimensions match selected paper size
- Preserve vector quality / line clarity in print output
- **DoD:** Patterns print at true scale by default, warnings shown if scale may change

#### S4-04: Print settings dialog
**URs:** UR-040, UR-044
- Paper size selection: A4, US Letter, A3, custom
- Orientation: Portrait / Landscape / Auto-detect
- Page range: All / Selection / Custom range
- Printer selection (query OS for available printers)
- Copies count
- **DoD:** Users can configure all standard print parameters

#### S4-05: Page selection for printing
**URs:** UR-039
- Select individual pages via checkboxes in preview
- Select page range via text input (e.g., "1-3, 5, 8-10")
- Visual indication of selected pages
- Print only selected pages
- **DoD:** Users can print specific pages or ranges

#### S4-06: Tiled multi-page printing
**URs:** UR-045, UR-046
- Detect large-format patterns (A0, A1, A2)
- Tile large pages across multiple A4/Letter sheets
- Show tile grid in preview with overlap margins
- Assembly marks on tiles (crop marks, overlap indicators)
- **DoD:** Large patterns can be printed as tiled sheets

#### S4-07: Layered printing support
**URs:** UR-047
- Detect PDF layers (OCG — Optional Content Groups)
- Display layer list with checkboxes
- Toggle layer visibility in preview
- Print only selected layers
- **DoD:** Multi-size/layered patterns allow per-layer printing

#### S4-08: Print instructions
**URs:** UR-050
- Extend print flow to instruction attachments
- Same print settings apply
- Print from document viewer context
- **DoD:** Instructions can be printed directly from viewer

---

## Sprint 5: Project Management

**Goal:** Allow users to create project entries linked to sewing patterns, track project-specific data, and organize work-in-progress.

### Issues

#### S5-01: Projects data model
**URs:** UR-052
- New `projects` table: `id`, `name`, `pattern_file_id` (FK to embroidery_files), `status`, `created_at`, `updated_at`, `notes`
- New `project_details` table: `id`, `project_id`, `key`, `value` (for chosen_size, fabric_used, etc.)
- Migration to schema v10 (or whichever version after S1)
- Rust models and CRUD commands
- **DoD:** Projects can be created, read, updated, deleted

#### S5-02: Project-specific information
**URs:** UR-053, UR-019
- Structured fields: chosen_size, fabric_used, planned_modifications, cut_version
- Free-form sewing notes per project
- Separate from pattern metadata
- Frontend: ProjectPanel component or expandable section in MetadataPanel
- **DoD:** Users can record project-specific data

#### S5-03: Project duplication
**URs:** UR-054
- "New Project from Pattern" action
- Copies project structure but references same source files (no file duplication)
- Pre-fills with previous project data as template (optional)
- **DoD:** Multiple projects reference same pattern without file duplication

#### S5-04: Collections / pattern grouping
**URs:** UR-017
- New `collections` table: `id`, `name`, `description`, `created_at`
- New `collection_items` table: `collection_id`, `file_id`
- A file can belong to multiple collections
- Sidebar section for collections (below folders)
- **DoD:** Patterns can be grouped into named collections

#### S5-05: Project list and navigation
**URs:** UR-052
- Project list view (list of all projects, filterable by status)
- Quick access from pattern detail: "Show Projects"
- Project dashboard showing status overview
- **DoD:** Users can browse and manage projects

---

## Sprint 6: Data Safety & Portability

**Goal:** Add recycle bin, backup/restore, library migration, and file re-linking to protect user data and enable portability.

### Issues

#### S6-01: Soft delete / recycle bin
**URs:** UR-057
- Add `deleted_at DATETIME NULL` to `embroidery_files` and `projects`
- "Delete" moves to trash (sets deleted_at), does not remove data
- Recycle bin view: list deleted items with restore/permanent-delete actions
- Auto-purge after configurable retention period (default: 30 days)
- Update all queries to exclude soft-deleted records by default
- **DoD:** Deleted items recoverable from recycle bin

#### S6-02: Backup & restore
**URs:** UR-058
- Backup: export SQLite DB + file references as ZIP archive
- Include: database file, attachment paths manifest, settings
- Optional: include actual files (pattern PDFs, thumbnails) for full backup
- Restore: import ZIP, rebuild database, re-link files
- Backend commands: `create_backup`, `restore_backup`
- Settings UI for backup schedule / manual trigger
- **DoD:** Users can create and restore full library backups

#### S6-03: Library migration
**URs:** UR-061
- Export library as portable package (metadata + relative file paths)
- Import on new device with path remapping
- Detect and report missing files during import
- **DoD:** Library transferable between devices

#### S6-04: Re-link missing files
**URs:** UR-063
- Detect missing files on startup or manual check
- Show "missing files" indicator in UI
- "Re-link" dialog: browse for new location, batch re-link by folder
- Automatic re-linking when library_root changes
- **DoD:** Broken file references can be repaired without losing metadata

#### S6-05: Structured metadata export
**URs:** UR-060, UR-062
- Export selected records as JSON or CSV
- Include: all metadata, tags, attachment references, project data
- Import from exported JSON (merge or replace)
- **DoD:** Metadata portable in standard formats

#### S6-06: Archive function
**URs:** UR-057
- "Archive" status separate from delete
- Archived items hidden from default view but searchable
- Bulk archive/unarchive
- Archive filter in search
- **DoD:** Items can be archived and recovered

---

## Sprint 7: Search & Filter Enhancement

**Goal:** Extend search and filter capabilities to cover all new metadata fields and improve sorting options.

### Issues

#### S7-01: Extended filter panel
**URs:** UR-028
- Add filter options for: skill_level, language, status, file_source, file_type
- Filter by garment type (via tags or category)
- Filter by size range (text match or structured parse)
- Collapsible advanced filter panel below SearchBar
- **DoD:** All requirement-specified filter criteria available

#### S7-02: Enhanced sorting
**URs:** UR-029
- Add sort options: title, date_added, author/designer, category, last_modified
- Sort direction toggle (asc/desc)
- Sort selector in toolbar or file list header
- Persist sort preference in settings
- **DoD:** All specified sort options functional

#### S7-03: Quick-access workflows
**URs:** UR-065
- "Quick Print" action: select pattern → print preview in 2 clicks
- "Recent Patterns" section in dashboard
- "Last Printed" tracking
- Search suggestions / recent searches
- **DoD:** Common workflows require minimal clicks

#### S7-04: Clear content-type distinction in UI
**URs:** UR-066
- Visual badges/icons distinguishing: pattern files, instructions, project notes, print settings
- Color-coded attachment types in MetadataPanel
- File type icons in FileList cards
- Legend or tooltip explaining distinctions
- **DoD:** Users can instantly identify content types visually

---

## Sprint 8: Integration & Stabilization

**Goal:** End-to-end validation against acceptance criteria, cross-feature integration, performance optimization, and UX polish.

### Issues

#### S8-01: Acceptance criteria validation
**URs:** AE-001 – AE-008
- Validate each acceptance expectation as an integration test scenario:
  - AE-001: Import pattern + instructions into one record
  - AE-002: Search/retrieve by title, tag, category, metadata
  - AE-003: Open instructions from same record
  - AE-004: Preview pattern before printing
  - AE-005: Print directly without external viewer
  - AE-006: Printed output at correct scale
  - AE-007: Print selected pages only
  - AE-008: Manage growing library without losing overview
- Document test results
- **DoD:** All 8 acceptance criteria pass

#### S8-02: Cross-feature integration testing
- Test workflows spanning multiple sprints: import → view → annotate → print → archive
- Test data model integrity across file types (embroidery + sewing pattern)
- Test backup/restore preserves all new data
- Performance test with 1000+ pattern library
- **DoD:** No integration regressions

#### S8-03: UI/UX polish
- Consistent German translations for all new UI elements
- Keyboard shortcuts for new features (print: Ctrl+P, viewer navigation)
- Loading states and progress indicators for PDF operations
- Error handling for corrupt/unsupported PDFs
- **DoD:** Polished, consistent UX across all features

#### S8-04: Documentation and release preparation
- Update CLAUDE.md with new architecture details
- Update settings documentation for new settings
- Create user-facing release notes
- Update supported formats documentation
- **DoD:** Documentation current, release notes written

---

## Dependency Graph

```
S1 (Data Model) ──→ S2 (PDF Support) ──→ S3 (Document Viewer) ──→ S4 (Print System)
       │                    │
       │                    └──→ S7 (Search & Filter)
       │
       └──→ S5 (Project Management)
       │
       └──→ S6 (Data Safety)

S1–S7 ──→ S8 (Integration & Stabilization)
```

- **S1** is the foundation — all other sprints depend on it
- **S2** depends on S1 (new file types need the extended model)
- **S3** depends on S2 (viewer needs PDF import working)
- **S4** depends on S3 (print builds on the viewer)
- **S5** can run in parallel with S3/S4 (independent feature)
- **S6** can run in parallel with S3/S4 (independent feature)
- **S7** depends on S1/S2 (new filter fields)
- **S8** depends on all previous sprints

---

## Technical Decisions (to resolve in Sprint 1)

| Decision | Options | Recommendation |
|----------|---------|---------------|
| PDF rendering | pdf.js (WASM), pdfium (native), Tauri webview | **pdf.js** — mature, well-documented, works in Tauri webview |
| Print pipeline | OS dialog via shell, `printpdf` crate, browser `window.print()` | **OS print dialog** — Tauri shell plugin for cross-platform; generate print-ready PDF first |
| True-scale control | CSS `@media print`, PDF manipulation, direct printer commands | **PDF manipulation** — embed scale metadata, disable fit-to-page in generated print PDF |
| Data model approach | Extend `embroidery_files`, new `sewing_patterns` table, polymorphic | **Extend existing** — add `file_type` discriminator, avoid table proliferation |
| Thumbnail generation for PDFs | pdfium, poppler, pdf.js server-side, ImageMagick | **pdfium-render** crate — fast native PDF-to-image for thumbnails |

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|-----------|
| PDF.js bundle size increases app significantly | Medium | Lazy-load pdf.js only when opening PDF viewer |
| True-scale printing varies across OS/printer drivers | High | Calibration test page, user-adjustable scale factor, clear warnings |
| Large PDF files (A0 patterns) cause memory issues | Medium | Stream pages on demand, limit concurrent rendered pages |
| Cross-platform print dialog inconsistency | Medium | Abstract OS differences in Rust print service, test on Linux/macOS/Windows |
| Schema migration breaks existing data | High | Thorough migration testing, backup before migration, rollback support |
| Feature scope creep from 74 requirements | Medium | Strict sprint boundaries, defer COULD HAVE items to post-release |

---

## Out of Scope for 26.04-a1

The following requirements are deferred to a future release:

- **UR-072/073/074** — Multi-user mode, role-based permissions, cloud sync
- Advanced project tracking beyond basic status/notes
- Cloud synchronization
- Full SVG pattern editing
- Pattern marketplace integration
