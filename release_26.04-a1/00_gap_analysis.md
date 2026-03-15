# Gap Analysis: Sewing Pattern Management Requirements vs. Current StitchManager

**Date:** 2026-03-15
**Base:** StitchManager release 26.03 (schema v8, 4 embroidery formats, 19 DB tables)
**Target:** Sewing Pattern Management requirements (UR-001 – UR-074)

---

## 1. Already Implemented (no work needed)

| UR | Requirement | Current Implementation |
|----|-------------|----------------------|
| UR-001 | Digital library | Embroidery file library with folders, metadata, tags |
| UR-005 | Large collections | Virtual-scrolled FileList (72px cards, 5-card buffer) |
| UR-006 | Simple UI | Three-panel layout, consistent component model |
| UR-007 | Import from local storage | ScannerService: scan_directory, import_files |
| UR-010 | Preserve originals | Files stored as-is; edits create version snapshots |
| UR-012 | Bulk import | Mass import with progress tracking and statistics |
| UR-015 | Custom metadata fields | custom_field_definitions + custom_field_values tables |
| UR-016 | Assign tags | Tags table with many-to-many file association |
| UR-026 | Searchable list | FTS5 full-text search across 11 fields |
| UR-027 | Search by metadata/tags/text | FTS5 + tag filter + advanced numeric filters |
| UR-030 | Thumbnail browsing | Embedded + synthetic PNG thumbnails |
| UR-051 | Favorite patterns | `is_favorite` boolean on embroidery_files |
| UR-055 | References intact | Foreign keys with ON DELETE CASCADE |
| UR-056 | Confirmation before delete | Dialog confirmation in frontend |
| UR-059 | Data loss prevention | WAL mode, file_versions (max 10 per file) |
| UR-064 | Non-technical users | German UI, clean layout, keyboard shortcuts |
| UR-067 | Responsive navigation | Virtual scrolling, debounced search |
| UR-068 | Dark/light mode | `hell`/`dunkel` theme with WCAG AA compliance |
| UR-069 | Acceptable open time | Lazy loading, virtual scroll |
| UR-070 | Stable with large collections | SQLite WAL, busy_timeout, virtual scroll |
| UR-071 | Bulk operations | Batch rename, organize, export, AI analysis |

**21 of 74 requirements already covered.**

---

## 2. Partially Implemented (enhancement needed)

| UR | Requirement | Current State | Gap |
|----|-------------|--------------|-----|
| UR-002 | Multiple files per entry | `file_attachments` table exists | Only "license" type; no structured multi-file model (pattern PDF, instruction PDF, cover image, etc.) |
| UR-003 | Link instructions to pattern | Attachments can store instructions | No explicit instruction type, no viewer integration |
| UR-009 | Multiple related files | Attachment CRUD exists | No typed classification (cover image, measurement chart, fabric requirements) |
| UR-013/014 | Metadata fields | 22+ fields exist | Missing: size_range, skill_level, language, format_type, file_source, purchase_link |
| UR-017 | Categories/collections | Folder hierarchy exists | No "collection" concept independent of folders |
| UR-028 | Filtering criteria | Format, tags, numeric ranges | Missing filters for: skill level, designer, language, status, garment type |
| UR-029 | Sorting | Basic sort exists | Missing sort by: designer, category, last modified |
| UR-060 | Export metadata | PDF report generation | No structured metadata export (JSON/CSV) |
| UR-062 | Export selected records | USB export for embroidery files | No metadata + file reference bundle export |

---

## 3. Not Implemented (new development required)

### 3.1 PDF & Document File Support (HIGH priority)
| UR | Requirement |
|----|-------------|
| UR-008 | Support PDF and standard image/document formats |
| UR-011 | Drag-and-drop import |

### 3.2 Instruction Management (HIGH priority)
| UR | Requirement |
|----|-------------|
| UR-004 | Unified interface for patterns + instructions |
| UR-020 | Attach instructions to pattern entry |
| UR-021 | Open and read instructions in-app |
| UR-022 | Readable format optimized for screen |
| UR-023 | Page-by-page or section navigation |
| UR-024 | Notes/comments linked to instructions |
| UR-025 | Bookmark instruction pages/sections |

### 3.3 Preview & Viewing (HIGH priority)
| UR | Requirement |
|----|-------------|
| UR-031 | Preview sewing patterns before printing |
| UR-032 | Preview instructions before opening/printing |
| UR-033 | Zoom and pan for pattern preview |
| UR-034 | Display page count, paper size, document properties |
| UR-035 | Single-page / multi-page overview switching |

### 3.4 Direct Printing (MUST HAVE)
| UR | Requirement |
|----|-------------|
| UR-036 | Print directly from app |
| UR-037 | No external application required |
| UR-038 | Print preview |
| UR-039 | Print full pattern or selected pages |
| UR-040 | Paper size, orientation, page range, printer selection |
| UR-041 | True-scale printing |
| UR-042 | Prevent unintended scaling by default |
| UR-043 | Warning if settings may alter scale |
| UR-044 | Support A4 and US Letter |
| UR-045 | Tiled multi-page printing |
| UR-046 | Large-format printing support |
| UR-047 | Layer/view selection for layered patterns |
| UR-048 | Measurement calibration element display |
| UR-049 | Preserve line clarity when printing |
| UR-050 | Print instructions from app |

### 3.5 Enhanced Metadata & Status (MEDIUM priority)
| UR | Requirement |
|----|-------------|
| UR-018 | Status tracking (not started / planned / in progress / completed / archived) |
| UR-019 | Project-specific notes separate from pattern metadata |

### 3.6 Project Management (SHOULD HAVE)
| UR | Requirement |
|----|-------------|
| UR-052 | Project folders / project entries linked to pattern |
| UR-053 | Project-specific info (size, fabric, modifications, notes) |
| UR-054 | Duplicate project without duplicating source files |

### 3.7 Data Safety & Portability (SHOULD HAVE)
| UR | Requirement |
|----|-------------|
| UR-057 | Recycle bin / archive / recovery |
| UR-058 | Backup and restore |
| UR-061 | Migrate collection to another device |
| UR-063 | Re-link missing files after path change |

### 3.8 Usability Enhancements (SHOULD HAVE)
| UR | Requirement |
|----|-------------|
| UR-065 | Minimize steps to find and print |
| UR-066 | Clear distinction between files, instructions, notes, print settings |

### 3.9 Optional / Future (COULD HAVE)
| UR | Requirement |
|----|-------------|
| UR-072 | Multi-user profiles |
| UR-073 | Role-based permissions |
| UR-074 | Cloud/shared storage conflict handling |

---

## 4. Effort Summary

| Category | Requirements | Effort |
|----------|-------------|--------|
| Already done | 21 | - |
| Enhancement | 9 | Small–Medium |
| New: PDF/Document support | 2 | Large |
| New: Instruction management | 7 | Large |
| New: Preview & viewing | 5 | Large |
| New: Printing | 15 | Very Large |
| New: Metadata & status | 2 | Small |
| New: Project management | 3 | Medium |
| New: Data safety & portability | 4 | Medium |
| New: Usability | 2 | Small |
| New: Optional | 3 | Future |
| **Total** | **74** | |

---

## 5. Key Technical Decisions Required

1. **PDF rendering engine** — Options: `pdf.js` (frontend WASM), `pdfium` (Rust binding), or Tauri webview with built-in PDF support
2. **Print pipeline** — Options: OS print dialog via Tauri shell, `printpdf` crate for PDF generation, or browser `window.print()` with custom CSS
3. **True-scale printing** — Requires precise DPI control; likely needs direct PDF-to-printer pipeline without browser scaling
4. **Data model evolution** — Extend `embroidery_files` or create parallel `sewing_patterns` table? Recommendation: extend existing model with a `file_type` discriminator
5. **Attachment reclassification** — Enhance `file_attachments` with structured types or create new tables for instructions, cover images, etc.
