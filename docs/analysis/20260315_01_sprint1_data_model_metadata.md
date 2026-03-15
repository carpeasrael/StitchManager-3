# Analysis: Sprint 1 — Data Model & Metadata Extension

**Date:** 2026-03-15
**Sprint:** S1 (release 26.04-a1)
**Issues:** S1-01, S1-02, S1-03, S1-04, S1-05
**Requirements:** UR-001, UR-004, UR-008, UR-013, UR-014, UR-018, UR-027, UR-028

---

## Problem Description

StitchManager currently supports only embroidery files (PES/DST/JEF/VP3). The sewing pattern management requirements (UR-001–UR-074) demand the app also manage sewing pattern files (primarily PDFs). Before any file format support can be added, the data model must be extended to:

1. **Distinguish file types** — embroidery vs. sewing pattern records need a discriminator
2. **Store sewing-specific metadata** — size range, skill level, language, format type, file source, purchase link
3. **Track project status** — patterns progress through states (not started → planned → in progress → completed → archived)
4. **Enable search/filter** by new fields
5. **Expose new fields** in the MetadataPanel UI

This is the foundational sprint; all subsequent sprints (PDF import, document viewer, printing, projects) depend on these schema changes.

---

## Affected Components

### Backend (Rust)
| File | Impact |
|------|--------|
| `src-tauri/src/db/migrations.rs` | New migration v9: 8 ALTER TABLE columns, 2 indexes, FTS5 rebuild |
| `src-tauri/src/db/models.rs` | +8 fields EmbroideryFile, +7 FileUpdate, +5 SearchParams |
| `src-tauri/src/db/queries.rs` | FILE_SELECT (29→37 cols), FILE_SELECT_ALIASED, row_to_file index shift |
| `src-tauri/src/commands/files.rs` | Extend update_file validation/SET clauses, build_query_conditions, add update_file_status |
| `src-tauri/src/lib.rs` | Register update_file_status command |

### Frontend (TypeScript)
| File | Impact |
|------|--------|
| `src/types/index.ts` | Extend EmbroideryFile, FileUpdate, SearchParams interfaces |
| `src/services/FileService.ts` | Add updateFileStatus() |
| `src/components/MetadataPanel.ts` | FormSnapshot, new form fields, helpers (addSelectField, addLinkField), save flow |
| `src/components/SearchBar.ts` | activeFilterCount, filter panel controls, active chips |

---

## Root Cause / Rationale

The current `embroidery_files` table is designed exclusively for embroidery files. It lacks:
- A `file_type` column to distinguish record types
- Metadata fields specific to sewing patterns (size_range, skill_level, language, etc.)
- A `status` field for project lifecycle tracking
- FTS5 indexing of the new searchable fields
- Frontend support for viewing/editing the new fields

Without these extensions, the app cannot store, search, or display sewing pattern records.

---

## Proposed Approach

### Single migration strategy
All schema changes are bundled into one migration (v9) to avoid multiple FTS5 rebuilds:

1. **Migration v9** adds 8 columns to `embroidery_files`:
   - `file_type TEXT NOT NULL DEFAULT 'embroidery'` + index
   - `size_range TEXT`, `skill_level TEXT`, `language TEXT`, `format_type TEXT`, `file_source TEXT`, `purchase_link TEXT`
   - `status TEXT NOT NULL DEFAULT 'none'` + index
   - Drops and recreates FTS5 table with 3 additional columns (language, file_source, size_range)
   - Recreates all 3 FTS5 triggers

2. **Rust models** — EmbroideryFile, FileUpdate, SearchParams structs extended

3. **Query layer** — FILE_SELECT grows to 37 columns; row_to_file shifts indices 25–28 to 33–36

4. **Commands** — update_file handles 7 new optional fields; build_query_conditions adds 5 equality filters; new update_file_status standalone command

5. **Frontend types** — EmbroideryFile, FileUpdate, SearchParams interfaces extended

6. **MetadataPanel** — Conditional sewing pattern section (visible when fileType='sewing_pattern'), always-visible status dropdown, new addSelectField/addLinkField helpers

7. **SearchBar** — 5 new filter controls in advanced panel, active filter chips

### Backward compatibility
- All new columns have safe defaults (`file_type='embroidery'`, `status='none'`, rest nullable)
- FileUpdate uses optional fields — existing code unchanged
- Scanner INSERT statements don't specify new columns — defaults apply

### Validation
- Status values validated server-side: `none`, `not_started`, `planned`, `in_progress`, `completed`, `archived`
- Skill level values: `beginner`, `easy`, `intermediate`, `advanced`, `expert`
- Purchase link: URL input type in frontend, no server-side URL validation (free text)

### Verification plan
1. `cargo check` — Rust compilation
2. `cargo test` — Migration tests, command tests
3. `npm run build` — TypeScript type checking
4. Manual: open existing file, verify defaults, test new fields, test search filters
