# Analysis: Issue #17 -- Detailed Search Function

**Date:** 2026-03-12
**Issue:** #17 -- "The application should have a detailed search function. It should be possible to search all parameters."

---

## 1. Problem Description

The current search implementation is a simple text box that performs a case-insensitive `LIKE` match against only two fields: `name` and `filename` on the `embroidery_files` table. The user issue requests that **all parameters** should be searchable. The app tracks a rich set of metadata per file -- over 20 fields across multiple tables -- yet search only covers 2 of them.

### What currently exists

- **Frontend (`SearchBar.ts`):** A single text input with 300ms debounce. The typed string is stored as `appState.searchQuery` (a plain string). No UI controls exist to select which fields to search, set numeric ranges, or filter by tags/colors.
- **Backend (`files.rs::get_files`):** The search parameter produces a single SQL clause: `(e.name LIKE ? OR e.filename LIKE ?)`. No JOINs to related tables (tags, colors, formats, custom fields) are performed for search.
- **State (`AppState.ts`):** Only `searchQuery: string` and `formatFilter: string | null` exist. There is no structured search/filter state.

### What is missing

Users cannot search or filter by:

| Category | Fields |
|----------|--------|
| Text metadata | `theme`, `description`, `license`, `design_name`, `category`, `author`, `keywords`, `comments` |
| Numeric metadata | `stitch_count`, `color_count`, `width_mm`, `height_mm`, `file_size_bytes`, `jump_count`, `trim_count`, `hoop_width_mm`, `hoop_height_mm` |
| Related entities | tags (via `file_tags`/`tags`), thread colors/brands (via `file_thread_colors`), format type/version (via `file_formats`) |
| Boolean / status | `ai_analyzed`, `ai_confirmed` |
| Custom fields | values from `custom_field_values` table |

---

## 2. Affected Components

### Backend (Rust)

| File | Change |
|------|--------|
| `src-tauri/src/commands/files.rs` | Expand `get_files` to accept a structured search object; build SQL with JOINs to tags, colors, custom fields; add numeric range filtering |
| `src-tauri/src/db/queries.rs` | Possibly add helper query fragments for multi-table search |
| `src-tauri/src/db/models.rs` | Add a `SearchParams` struct for deserialization |
| `src-tauri/src/commands/mod.rs` | Re-export if new command added |

### Frontend (TypeScript)

| File | Change |
|------|--------|
| `src/components/SearchBar.ts` | Replace simple text input with expandable advanced search UI; add field selector, operators, value inputs |
| `src/services/FileService.ts` | Update `getFiles()` signature to pass structured search params |
| `src/state/AppState.ts` | Add structured search state alongside or replacing the string `searchQuery` |
| `src/types/index.ts` | Add `SearchParams` / `SearchFilter` interfaces |
| `src/components/FileList.ts` | Adapt `loadFiles()` to pass new search params |
| `src/styles/components.css` | Styles for expanded search panel, filter pills, range inputs |
| `src/main.ts` | Possibly adapt `reloadFiles()` to use new search params |

---

## 3. Root Cause / Rationale

The search was built as a minimal MVP feature during sprint 4 (file import/list phase). Only `name` and `filename` were indexed and queried because the primary use case was "find a file by name." The data model was expanded significantly in later sprints (parsers adding stitch/color/dimension data in sprints 5-6, AI metadata in sprint 8, extended PES fields in v3 migration), but the search query was never updated to cover the new fields.

The `get_files` Rust command hardcodes a two-field `LIKE` clause. To search tags, colors, or custom fields would require JOINs to `file_tags`/`tags`, `file_thread_colors`, and `custom_field_values` -- none of which are present in the current query.

Additionally, numeric fields like `stitch_count` or `width_mm` require range-based filtering (e.g., "stitch count between 5000 and 20000"), which the current string-only `LIKE` approach cannot express.

---

## 4. Proposed Approach

### Phase A: Data model and backend search command

**Step A1 -- Define `SearchParams` struct (backend)**

Add a new `SearchParams` struct in `src-tauri/src/db/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    /// Free-text query: searches name, filename, theme, description,
    /// design_name, category, author, keywords, comments, license
    pub text: Option<String>,

    /// Tags: file must have ALL listed tags (AND logic)
    pub tags: Option<Vec<String>>,

    /// Numeric range filters
    pub stitch_count_min: Option<i32>,
    pub stitch_count_max: Option<i32>,
    pub color_count_min: Option<i32>,
    pub color_count_max: Option<i32>,
    pub width_mm_min: Option<f64>,
    pub width_mm_max: Option<f64>,
    pub height_mm_min: Option<f64>,
    pub height_mm_max: Option<f64>,
    pub file_size_min: Option<i64>,
    pub file_size_max: Option<i64>,

    /// Boolean filters
    pub ai_analyzed: Option<bool>,
    pub ai_confirmed: Option<bool>,

    /// Thread color name or brand search
    pub color_search: Option<String>,
}
```

**Step A2 -- Expand `get_files` command (backend)**

Modify `get_files` in `src-tauri/src/commands/files.rs`:

1. Accept `search_params: Option<SearchParams>` instead of (or in addition to) `search: Option<String>`.
2. Keep backward compatibility: if only `search` is provided (plain string), use it as the `text` field of `SearchParams`.
3. Build dynamic SQL:
   - **Text search:** Expand the `LIKE` clause to cover all text columns: `name`, `filename`, `theme`, `description`, `design_name`, `category`, `author`, `keywords`, `comments`, `license`. Use OR across all fields.
   - **Tag filter:** Add `EXISTS (SELECT 1 FROM file_tags ft JOIN tags t ON t.id = ft.tag_id WHERE ft.file_id = e.id AND t.name = ?)` for each tag. AND logic means one EXISTS per tag.
   - **Numeric ranges:** Add `e.stitch_count >= ? AND e.stitch_count <= ?` style clauses for each min/max pair.
   - **Boolean filters:** Add `e.ai_analyzed = ?` style clauses.
   - **Color search:** Add `EXISTS (SELECT 1 FROM file_thread_colors ftc WHERE ftc.file_id = e.id AND (ftc.color_name LIKE ? OR ftc.brand LIKE ?))`.
4. All conditions are AND-combined (narrowing search).

**Step A3 -- Add tests**

Add Rust unit tests for:
- Text search across multiple fields
- Tag-based filtering
- Numeric range queries
- Boolean filters
- Color/brand search
- Combined multi-filter queries

### Phase B: Frontend types and service layer

**Step B1 -- Add TypeScript `SearchParams` interface**

In `src/types/index.ts`:

```typescript
export interface SearchParams {
  text?: string;
  tags?: string[];
  stitchCountMin?: number;
  stitchCountMax?: number;
  colorCountMin?: number;
  colorCountMax?: number;
  widthMmMin?: number;
  widthMmMax?: number;
  heightMmMin?: number;
  heightMmMax?: number;
  fileSizeMin?: number;
  fileSizeMax?: number;
  aiAnalyzed?: boolean;
  aiConfirmed?: boolean;
  colorSearch?: string;
}
```

**Step B2 -- Update `FileService.getFiles()`**

Modify the service to pass `searchParams` to the backend invoke call. Maintain backward compatibility during transition.

**Step B3 -- Update `AppState`**

Add `searchParams: SearchParams` to the `State` interface. The existing `searchQuery: string` can be kept as the `text` field shortcut for the quick-search input. Add an `activeFilterCount: number` computed from non-empty search params so the UI can show a badge.

### Phase C: SearchBar UI expansion

**Step C1 -- Keep the quick-search input**

The existing text input stays as a fast "search everywhere" box. Typing here sets `searchParams.text`. This provides the same UX for simple searches.

**Step C2 -- Add an "Advanced filters" toggle**

Add a filter icon button next to the search input. Clicking it expands a dropdown/panel below the toolbar with:

- **Tag filter:** Multi-select chip input (reuse the tag autocomplete pattern from MetadataPanel).
- **Numeric ranges:** Paired min/max number inputs for stitch count, color count, dimensions, file size. Show only the most common ones by default, with an "Weitere" (More) toggle.
- **Status filters:** Checkboxes or toggle chips for "KI-analysiert" / "KI-bestaetigt".
- **Color/brand search:** Text input to find files by thread color name or brand.
- **Active filter indicator:** A badge on the filter button showing the count of active filters.
- **"Alle zuruecksetzen" (Reset all):** Button to clear all filters.

**Step C3 -- Filter chips summary row**

Display active filters as removable chips below the toolbar (or inline next to the format filter chips). Each chip shows the filter name and value; clicking X removes that filter.

### Phase D: Integration wiring

**Step D1 -- `FileList.loadFiles()` adaptation**

Update to pass the full `searchParams` object from AppState instead of just the `searchQuery` string.

**Step D2 -- `main.ts::reloadFiles()` adaptation**

Update to read and pass `searchParams`.

**Step D3 -- Shortcut integration**

The existing `Ctrl+F` shortcut focuses the text input. No change needed, but pressing `Ctrl+F` twice could toggle the advanced panel.

**Step D4 -- Format filter integration**

The existing `formatFilter` in AppState can remain separate or be folded into `searchParams`. Recommend keeping it separate for now since the FilterChips component manages it independently, but the backend should accept it alongside `searchParams`.

### Phase E: Validation and polish

- Verify all filters work individually and in combination.
- Verify empty/null filter params are ignored (no result narrowing).
- Verify performance with large file sets (use parameterized queries, avoid N+1).
- Verify the search panel respects the theme (light/dark).
- Verify keyboard accessibility (Tab through filter inputs, Enter to apply, Escape to close panel).
- Run `cargo test`, `npm run build`, `cargo check`.

---

## Summary of Changes by File

| File | Type | Change |
|------|------|--------|
| `src-tauri/src/db/models.rs` | Backend | Add `SearchParams` struct |
| `src-tauri/src/commands/files.rs` | Backend | Expand `get_files` to handle structured search with multi-table JOINs/EXISTS |
| `src-tauri/src/db/queries.rs` | Backend | Minor: possibly add aliased query helpers |
| `src/types/index.ts` | Frontend | Add `SearchParams` interface |
| `src/state/AppState.ts` | Frontend | Add `searchParams` to `State` |
| `src/services/FileService.ts` | Frontend | Update `getFiles()` to pass `SearchParams` |
| `src/components/SearchBar.ts` | Frontend | Add advanced filter panel UI, filter toggle, active filter badge |
| `src/components/FileList.ts` | Frontend | Pass `searchParams` to `loadFiles()` |
| `src/main.ts` | Frontend | Update `reloadFiles()` to use `searchParams` |
| `src/styles/components.css` | Frontend | Styles for filter panel, range inputs, filter chips |

**Estimated scope:** Medium-large. The backend SQL builder is the most complex part. The frontend is mostly additive UI without changing existing component contracts.
