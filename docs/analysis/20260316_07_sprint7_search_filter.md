# Sprint 7 Analysis: Search & Filter Enhancement

**Date:** 2026-03-16
**Sprint:** 7 — Search & Filter Enhancement
**Requirements:** UR-028 (filtering), UR-029 (sorting), UR-065 (quick-access workflows), UR-066 (content-type distinction)

---

## S7-01: Extended Filter Panel

### Problem Description

UR-028 requires filtering by garment type, size, designer/brand, language, status, skill level, and tags. The current advanced filter panel in `SearchBar.ts` already has filters for tags, numeric ranges (stitches, colors, width, height, file size), AI status booleans, color/brand search, file type, status, skill level, language, and file source. However, several UR-028 criteria are missing:

- **Garment type** — no filter for `category` field (which maps to garment/project type)
- **Size range** — no filter for the `sizeRange` field (e.g., 34-46, S-XL)
- **Designer/Brand** — no filter for the `author` field (designer/brand)

The backend `SearchParams` struct also lacks these three filter fields, and `build_query_conditions` has no clauses for them.

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| Backend model | `src-tauri/src/db/models.rs` | Add `category`, `size_range`, `author` to `SearchParams` |
| Backend query | `src-tauri/src/commands/files.rs` | Add WHERE clauses in `build_query_conditions` for the three new fields |
| Frontend types | `src/types/index.ts` | Add `category`, `sizeRange`, `author` to `SearchParams` interface |
| Frontend UI | `src/components/SearchBar.ts` | Add three new filter inputs in `renderPanel`, update `activeFilterCount`, update `buildActiveChips` |

### Root Cause / Rationale

The existing filter panel was built around embroidery-specific attributes (stitches, colors, dimensions). The sewing-pattern metadata fields (`category`, `sizeRange`, `author`) were added to the data model but never wired into the filter panel or backend query conditions.

### Proposed Approach

1. **Backend `SearchParams`** — add three `Option<String>` fields: `category`, `size_range`, `author`.
2. **Backend `build_query_conditions`** — add LIKE-based clauses for `category` and `author` (partial match is more useful than exact match for free-text fields), and exact match for `size_range`.
3. **Frontend `SearchParams`** — add `category?: string`, `sizeRange?: string`, `author?: string`.
4. **`SearchBar.ts`** — add `buildTextFilter` calls for "Kategorie" (`category`, placeholder "z.B. Kleid, Rock..."), "Groesse" (`sizeRange`, placeholder "z.B. 34-46, S-XL..."), "Designer" (`author`, placeholder "z.B. Burda, Simplicity...").
5. **`activeFilterCount`** — count the three new fields.
6. **`buildActiveChips`** — add chip entries for the three new fields.

---

## S7-02: Enhanced Sorting

### Problem Description

UR-029 requires sorting by title, date added, designer, category, and last modified date. Currently, the backend hardcodes `ORDER BY e.filename` in both `query_files_impl` and `get_files_paginated`. There is no sort parameter in the API, no sort UI in the frontend, and no persistence of sort preferences.

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| Backend model | `src-tauri/src/db/models.rs` | Add `sort_field` and `sort_direction` to `SearchParams` |
| Backend query | `src-tauri/src/commands/files.rs` | Build dynamic `ORDER BY` from `SearchParams` sort fields (whitelist allowed columns) |
| Frontend types | `src/types/index.ts` | Add `sortField?: string`, `sortDirection?: 'asc' \| 'desc'` to `SearchParams` |
| Frontend UI | `src/components/SearchBar.ts` | Add sort controls (select + direction toggle) in the advanced filter panel |
| Frontend state | `src/state/AppState.ts` | No structural change; sort lives inside `searchParams` |
| Frontend persistence | `src/services/SettingsService.ts` | Persist sort preference via settings key `sort_field` and `sort_direction` |
| Frontend init | `src/main.ts` | Load persisted sort on startup into `searchParams` |

### Root Cause / Rationale

Sort was never parameterized. The hardcoded `ORDER BY e.filename` ignores all user requirements for flexible sorting. Users with large libraries need to sort by date added (find newest), by designer (group by brand), by category, or by last modified (find recently edited).

### Proposed Approach

1. **Backend `SearchParams`** — add `sort_field: Option<String>` and `sort_direction: Option<String>`.
2. **Backend ORDER BY** — in both `query_files_impl` and `get_files_paginated`, build the ORDER BY clause from `SearchParams`:
   - Whitelist allowed sort columns: `filename` (title), `created_at` (date added), `author` (designer), `category`, `updated_at` (last modified), `name`.
   - Map `sortField` values to SQL column names: `title` -> `COALESCE(e.name, e.filename)`, `date_added` -> `e.created_at`, `author` -> `e.author`, `category` -> `e.category`, `last_modified` -> `e.updated_at`.
   - Validate `sortDirection` is `asc` or `desc`, default `asc`.
   - Default to `ORDER BY e.filename ASC` when no sort is specified.
3. **Frontend `SearchParams`** — add `sortField?: string` and `sortDirection?: 'asc' | 'desc'`.
4. **`SearchBar.ts`** — add a sort section at the top of the advanced filter panel:
   - A `<select>` with options: Dateiname (default), Titel, Hinzugefuegt, Designer, Kategorie, Zuletzt bearbeitet.
   - A toggle button for ascending/descending direction.
   - On change, update `searchParams.sortField` and `searchParams.sortDirection`.
5. **Persistence** — on sort change, save to settings via `SettingsService.setSetting`. On init, load from settings into `searchParams`.

---

## S7-03: Quick-Access Workflows

### Problem Description

UR-065 requires minimizing steps to find and print a pattern. The Dashboard already shows "Zuletzt bearbeitet" (recent files) and "Favoriten" (favorites), but lacks:

- **Quick print** — no one-click print button on dashboard file cards or file list cards
- **Recent searches** — no history of recent search queries for quick re-use
- **Last printed** — no tracking or display of last-printed patterns

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| Frontend UI | `src/components/Dashboard.ts` | Add quick-print button on file cards; add "Zuletzt gedruckt" section |
| Frontend UI | `src/components/SearchBar.ts` | Add recent searches dropdown below the search input |
| Frontend state | `src/state/AppState.ts` | Add `recentSearches: string[]` to `State` |
| Frontend types | `src/types/index.ts` | Add `recentSearches` to `State` |
| Backend | `src-tauri/src/commands/files.rs` | Add `get_recently_printed` command (query by a new `last_printed_at` column or by settings/history) |
| Database | `src-tauri/src/db/migrations.rs` | Add `last_printed_at` column to `embroidery_files` |
| Backend model | `src-tauri/src/db/models.rs` | Add `last_printed_at` to `EmbroideryFile` |
| Backend query | `src-tauri/src/db/queries.rs` | Add `last_printed_at` to `FILE_SELECT` and `row_to_file` |
| Frontend service | `src/services/FileService.ts` | Add `getRecentlyPrinted()`, `markAsPrinted(fileId)` wrappers |
| CSS | `src/styles/components.css` | Styles for quick-print button overlay, recent searches dropdown |

### Root Cause / Rationale

The app has print functionality (via `PrintPreviewDialog`) but no shortcut to print directly from the dashboard or file list. Recent searches are not tracked, requiring users to retype common queries. Last-printed tracking does not exist in the database.

### Proposed Approach

1. **Database migration** — add `last_printed_at DATETIME DEFAULT NULL` column to `embroidery_files` table.
2. **Backend** — add `mark_as_printed` command (sets `last_printed_at = CURRENT_TIMESTAMP`) and `get_recently_printed` command (query ordered by `last_printed_at DESC NULLS LAST`, limit 12).
3. **Backend model/queries** — include `last_printed_at` in `EmbroideryFile`, `FILE_SELECT`, `row_to_file`.
4. **Frontend `FileService`** — add `markAsPrinted(fileId)` and `getRecentlyPrinted(limit)`.
5. **Print integration** — after successful print in `PrintPreviewDialog`, call `markAsPrinted`.
6. **Dashboard** — add "Zuletzt gedruckt" section after favorites, using `getRecentlyPrinted(12)`. Add a small print icon button on each dashboard file card that triggers `toolbar:print` for that file.
7. **Recent searches** — store last 10 unique search queries in `AppState.recentSearches` (persisted via `SettingsService` as JSON). Show a dropdown below the search input when focused and empty, listing recent searches as clickable items.

---

## S7-04: Content-Type Distinction

### Problem Description

UR-066 requires clear visual distinction between original pattern files, instructions, project notes, and printed output settings. Currently:

- File cards in `FileList.ts` show only format label (PES/DST/etc.), file size, and AI badge.
- Attachments show a generic paperclip icon with count but no type differentiation.
- The `file_type` field (`embroidery` or `sewing_pattern`) is stored but not visually indicated.
- Attachment types (`pattern`, `instruction`, `cover`, `measurement`, `fabric`, `other`) have no visual coding.

### Affected Components

| Layer | File | Change |
|-------|------|--------|
| Frontend UI | `src/components/FileList.ts` | Add file-type badge (visual icon/label), color-coded attachment indicators |
| Frontend UI | `src/components/MetadataPanel.ts` | Show attachment type icons in attachment list |
| Frontend types | `src/types/index.ts` | Add attachment type icon/color mapping constants |
| Frontend utils | `src/utils/format.ts` | Add `getFileTypeIcon()`, `getFileTypeBadge()`, `getAttachmentTypeIcon()` helper functions |
| CSS | `src/styles/components.css` | Badge styles, color tokens for file types and attachment types |

### Root Cause / Rationale

The data model already distinguishes file types and attachment types, but the UI renders everything uniformly. Users cannot visually scan a list and instantly tell whether an entry is an embroidery file vs. a sewing pattern, or whether attachments are instructions vs. cover images.

### Proposed Approach

1. **File type badges** — in `FileList.ts` `createCard()`, add a badge next to the filename:
   - `embroidery` -> icon stitch symbol, accent color (e.g., blue badge "Stickdatei")
   - `sewing_pattern` -> icon scissors symbol, different accent (e.g., purple badge "Schnittmuster")
   - Render as a small colored tag, similar to the existing AI badge.

2. **Attachment type indicators** — replace the generic paperclip with color-coded icons per attachment type:
   - `pattern` -> document icon, primary color
   - `instruction` -> book icon, green
   - `cover` -> image icon, orange
   - `measurement` -> ruler icon, teal
   - `fabric` -> fabric swatch icon, pink
   - `other` -> paperclip, gray
   Show these as small colored dots or mini-icons next to the attachment count.

3. **Utility functions** — add to `src/utils/format.ts`:
   - `getFileTypeLabel(fileType: string): string` — returns German label
   - `getFileTypeClass(fileType: string): string` — returns CSS class
   - `getAttachmentTypeIcon(type: string): string` — returns emoji/unicode icon
   - `getAttachmentTypeClass(type: string): string` — returns CSS class

4. **MetadataPanel** — in the attachments section, show the type-specific icon before each attachment filename.

5. **CSS** — define badge styles:
   - `.file-type-badge` base + `.file-type-badge--embroidery`, `.file-type-badge--sewing-pattern`
   - `.attachment-type-icon` with color variants per type
   - Ensure WCAG AA contrast in both light and dark themes.

6. **FileList attachment loading** — extend `getAttachmentCounts` or add `getAttachmentTypeSummary(fileIds)` that returns per-file attachment type breakdown for rendering color-coded indicators. Alternatively, load attachment types only for visible cards (lazy approach matching existing pattern).

---

## Implementation Order

1. **S7-01** (Extended filters) — backend + frontend filter additions, lowest risk
2. **S7-02** (Enhanced sorting) — backend ORDER BY parameterization + frontend sort UI
3. **S7-04** (Content-type distinction) — visual badges and icons, CSS-heavy
4. **S7-03** (Quick-access workflows) — requires DB migration, most cross-cutting

## Estimated Scope

- **Backend changes:** ~150 lines (models, query conditions, sort logic, new commands)
- **Frontend changes:** ~300 lines (SearchBar filters/sort, FileList badges, Dashboard sections, format utils)
- **CSS changes:** ~80 lines (badge styles, sort controls, recent searches dropdown)
- **DB migration:** 1 new column (`last_printed_at`)
