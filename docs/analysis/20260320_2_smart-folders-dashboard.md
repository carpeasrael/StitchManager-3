# Analysis: Smart Folders & Dashboard (Issue #126, Phase 4)

Date: 2026-03-20
Issue: #126 — Proposals 7 & 8

---

## Problem Description

### Proposal 7: Smart / Virtual Folders

Users cannot save filtered views. Common queries such as "all unanalyzed files", "all 5-star patterns", or "all files tagged 'Weihnachten'" must be manually reconstructed each session. There is no mechanism to persist a filter configuration as a named, reusable entity.

The existing `SearchParams` interface (defined in `src/types/index.ts` lines 148-174 and `src-tauri/src/db/models.rs` lines 629-678) already supports rich filtering: text search, tags, numeric ranges (stitch count, color count, dimensions, file size), boolean flags (ai_analyzed, ai_confirmed), and categorical filters (file_type, status, skill_level, language, file_source, category, author, size_range). However, there is no `rating_min`/`rating_max` or `is_favorite` filter in the current `SearchParams`. These gaps must be closed for smart folders to support "5 Sterne" or "Favoriten" presets.

### Proposal 8: Folder Statistics & Dashboard Enhancement

A basic `Dashboard` component already exists (`src/components/Dashboard.ts`). It shows:
- Library stats via `get_library_stats` (total files, folders, stitches, format counts)
- Recently edited files (up to 12)
- Favorite files

However, it lacks deeper analytics:
- No breakdown by AI analysis status (none / pending / confirmed)
- No top-10 folders by file count
- No count of files missing metadata (no tags, no rating, no description)
- No storage usage per folder
- No recent import activity (last 7 days)
- No file type breakdown (embroidery vs. sewing_pattern vs. other)

The existing `get_library_stats` Rust command (`src-tauri/src/commands/files.rs` lines 538-563) returns only `total_files`, `total_folders`, `total_stitches`, and `format_counts`. It needs to be expanded or supplemented with a dedicated statistics command.

---

## Affected Components

### Proposal 7: Smart Folders

| Layer | File | Lines/Detail | Change |
|-------|------|-------------|--------|
| DB schema | `src-tauri/src/db/migrations.rs` | `CURRENT_VERSION = 25` (line 5), migration chain ends at v25 (lines 144-146) | Add v26: `smart_folders` table |
| DB models | `src-tauri/src/db/models.rs` | After line 678 | Add `SmartFolder` struct |
| DB models | `src-tauri/src/db/models.rs` | `SearchParams` struct (lines 629-678) | Add `rating_min`, `rating_max`, `is_favorite` fields |
| Backend commands | `src-tauri/src/commands/mod.rs` | 21 modules (line 1-21) | Add `pub mod smart_folders;` |
| Backend commands | New: `src-tauri/src/commands/smart_folders.rs` | — | CRUD commands for smart folders |
| Backend commands | `src-tauri/src/commands/files.rs` | `build_query_conditions()` lines 15-252 | Add rating_min/max and is_favorite filter handling |
| Backend registration | `src-tauri/src/lib.rs` | `invoke_handler` macro (lines 118-353) | Register smart folder commands |
| Frontend types | `src/types/index.ts` | `SearchParams` (lines 148-174), `State` (lines 691-705) | Add `SmartFolder` interface; extend `SearchParams` with `ratingMin`, `ratingMax`, `isFavorite`; add `smartFolders` + `selectedSmartFolderId` to `State` |
| Frontend service | New: `src/services/SmartFolderService.ts` | — | Tauri invoke wrappers for CRUD |
| Frontend state | `src/state/AppState.ts` | `initialState` (lines 5-19) | Add `smartFolders: []` and `selectedSmartFolderId: null` |
| Sidebar | `src/components/Sidebar.ts` | After `renderCollections()` call (line 282) | Add "Intelligente Ordner" section rendering |
| FileList | `src/components/FileList.ts` | `loadFiles()` method (lines 59-78) | React to `selectedSmartFolderId` state changes, apply smart folder's filter_json as SearchParams |
| Dialog | New: `src/components/SmartFolderDialog.ts` | — | Create/edit smart folder dialog |
| Entry point | `src/main.ts` | Lines 1066-1076 (center panel setup) | Wire smart folder selection events |

### Proposal 8: Dashboard Enhancement

| Layer | File | Lines/Detail | Change |
|-------|------|-------------|--------|
| Backend commands | New: `src-tauri/src/commands/statistics.rs` | — | Aggregate SQL queries for dashboard stats |
| Backend commands | `src-tauri/src/commands/mod.rs` | Line 1-21 | Add `pub mod statistics;` |
| Backend registration | `src-tauri/src/lib.rs` | Lines 118-353 | Register statistics commands |
| Frontend service | New: `src/services/StatisticsService.ts` | — | Invoke wrappers for statistics |
| Dashboard | `src/components/Dashboard.ts` | `renderDashboard()` (lines 53-129) | Add new sections: AI status breakdown, top folders, missing metadata, storage, imports |
| Frontend types | `src/types/index.ts` | After `LibraryStats` (lines 254-259) | Add `DashboardStats` interface |

---

## Root Cause / Rationale

### Why Smart Folders Are Needed

The current architecture requires users to manually set `searchParams` in the AppState each time they want to filter files. The `SearchBar` and `FilterChips` components offer limited pre-built filters (text search and format type). For power users managing large libraries, repeatedly configuring the same filter criteria is tedious and error-prone. Smart folders persist filter configurations in the database, making them instantly accessible from the sidebar.

The `SearchParams` interface already covers most filter dimensions, so smart folders can simply serialize a `SearchParams` object as `filter_json`. The only missing filter fields are:
1. **`rating_min` / `rating_max`** — the `rating` column exists on `embroidery_files` (added in migration v4, range 1-5) but `SearchParams` has no filter for it
2. **`is_favorite`** — the `is_favorite` column exists but is not filterable via `SearchParams`

### Why Dashboard Enhancement Is Needed

The existing `Dashboard` component (`src/components/Dashboard.ts`) provides basic stats but no actionable analytics. Users cannot see at a glance which files need attention (missing metadata, unanalyzed), which folders consume the most storage, or recent import activity. These analytics help users maintain library quality and understand their collection's health.

The existing `get_library_stats` command uses simple SQL aggregates. The proposed statistics queries follow the same pattern and can share the `lock_db` connection approach.

---

## Proposed Approach

### Phase 1: Extend SearchParams (prerequisite for both proposals)

**Step 1.1** — Add `rating_min`, `rating_max`, and `is_favorite` to `SearchParams`:

- In `src-tauri/src/db/models.rs`, add to `SearchParams` struct (after line 670):
  ```rust
  pub rating_min: Option<i32>,
  pub rating_max: Option<i32>,
  pub is_favorite: Option<bool>,
  ```

- In `src/types/index.ts`, add to `SearchParams` interface (after line 173):
  ```typescript
  ratingMin?: number;
  ratingMax?: number;
  isFavorite?: boolean;
  ```

**Step 1.2** — Handle new filters in `build_query_conditions()`:

- In `src-tauri/src/commands/files.rs`, inside `build_query_conditions()` (after the `size_range` block ending at line 251), add:
  - `rating_min`: `e.rating >= ?N`
  - `rating_max`: `e.rating <= ?N`
  - `is_favorite`: `e.is_favorite = ?N` (converting bool to 0/1)

### Phase 2: Smart Folders Backend

**Step 2.1** — Database migration v26:

- In `src-tauri/src/db/migrations.rs`:
  - Change `CURRENT_VERSION` from 25 to 26 (line 5)
  - Add `if current < 26 { apply_v26(conn)?; }` in `run_migrations()` (after line 146)
  - Add `apply_v26()` function creating the `smart_folders` table:
    ```sql
    CREATE TABLE IF NOT EXISTS smart_folders (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        name        TEXT NOT NULL,
        icon        TEXT NOT NULL DEFAULT '🔍',
        filter_json TEXT NOT NULL,
        sort_order  INTEGER NOT NULL DEFAULT 0,
        created_at  TEXT NOT NULL DEFAULT (datetime('now'))
    );
    ```
  - Insert default smart folders:
    - "Nicht analysiert" — `{"aiAnalyzed": false}`
    - "5 Sterne" — `{"ratingMin": 5}`
    - "Kuerzlich importiert" — (special: will use `created_at` date range, encoded as `{"sortField": "created_at", "sortDirection": "desc"}` with a date-range filter)
  - Update test assertions: add `"smart_folders"` to the expected tables list (line 1352-1404) and update schema version assertion from 25 to 26 (line 1420)

**Step 2.2** — Rust model:

- In `src-tauri/src/db/models.rs`, add `SmartFolder` struct:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct SmartFolder {
      pub id: i64,
      pub name: String,
      pub icon: String,
      pub filter_json: String,
      pub sort_order: i32,
      pub created_at: String,
  }
  ```

**Step 2.3** — Command module `src-tauri/src/commands/smart_folders.rs`:

Commands to implement:
- `get_smart_folders()` — returns all smart folders ordered by `sort_order`
- `create_smart_folder(name, icon, filter_json)` — validate JSON parsability, insert, return new entity
- `update_smart_folder(id, name, icon, filter_json)` — update, return updated entity
- `delete_smart_folder(id)` — delete by id
- `update_smart_folder_sort_orders(orders: Vec<(i64, i32)>)` — batch update sort orders

All commands follow the existing pattern: take `State<'_, DbState>`, call `lock_db(&db)?`, execute SQL, return `Result<T, AppError>`.

**Step 2.4** — Register in `src-tauri/src/commands/mod.rs` and `src-tauri/src/lib.rs`:

- Add `pub mod smart_folders;` to `mod.rs`
- Add all 5 commands to the `invoke_handler!` macro in `lib.rs`

### Phase 3: Smart Folders Frontend

**Step 3.1** — TypeScript types:

- In `src/types/index.ts`, add `SmartFolder` interface:
  ```typescript
  export interface SmartFolder {
    id: number;
    name: string;
    icon: string;
    filterJson: string;
    sortOrder: number;
    createdAt: string;
  }
  ```
- Add to `State` interface:
  ```typescript
  smartFolders: SmartFolder[];
  selectedSmartFolderId: number | null;
  ```

**Step 3.2** — `src/state/AppState.ts`:

- Add to `initialState`: `smartFolders: []` and `selectedSmartFolderId: null`

**Step 3.3** — `src/services/SmartFolderService.ts`:

Create with invoke wrappers matching the 5 Rust commands.

**Step 3.4** — `src/components/Sidebar.ts`:

After the Collections section (`renderCollections()` at line 282), add a new `renderSmartFolders()` method:
- Header: "Intelligente Ordner" with an "+" button to create new smart folders
- List items with icon, name, and delete button
- Click handler: set `selectedSmartFolderId` in AppState and clear `selectedFolderId`
- Subscribe to `appState.on("smartFolders", ...)` and `appState.on("selectedSmartFolderId", ...)` for re-renders
- Load smart folders on initialization via `SmartFolderService.getAll()`

**Step 3.5** — `src/components/FileList.ts`:

- Add subscription to `appState.on("selectedSmartFolderId", () => this.loadFiles())` (after line 38)
- In `loadFiles()` (lines 59-78): when `selectedSmartFolderId` is set, load the smart folder's `filterJson`, parse it as `SearchParams`, and pass it to `getFilesPaginated()` with `folderId = null`

**Step 3.6** — `src/components/SmartFolderDialog.ts`:

New dialog component for creating/editing smart folders:
- Name input field
- Icon selector (emoji picker or text input)
- Filter configuration form (leveraging existing SearchParams fields):
  - Tags multi-select
  - AI status (not analyzed / analyzed / confirmed)
  - Rating range (1-5 stars)
  - File type dropdown (embroidery / sewing_pattern)
  - Favorite toggle
  - Free text search
- "Aus aktuellem Filter erstellen" (create from current filter) button that captures current AppState searchParams
- Save/Cancel buttons

**Step 3.7** — `src/main.ts`:

- Import and wire `SmartFolderDialog` if used as a modal
- Handle smart folder selection: when `selectedSmartFolderId` changes, clear `selectedFolderId` and vice versa (mutual exclusion)

### Phase 4: Statistics Backend

**Step 4.1** — New command module `src-tauri/src/commands/statistics.rs`:

Single command `get_dashboard_stats()` returning a `DashboardStats` struct with:
- `files_by_type`: `HashMap<String, i64>` — count grouped by `file_type` (embroidery, sewing_pattern, etc.)
- `ai_status`: `{ none: i64, analyzed: i64, confirmed: i64 }` — based on `ai_analyzed` and `ai_confirmed` columns
- `top_folders`: `Vec<{ folder_name: String, file_count: i64 }>` — top 10 folders by file count
- `missing_metadata`: `{ no_tags: i64, no_rating: i64, no_description: i64 }` — files lacking key metadata
- `storage_by_folder`: `Vec<{ folder_name: String, total_bytes: i64 }>` — sum of file_size_bytes per folder
- `recent_imports`: `i64` — files created in the last 7 days

SQL queries (all filtered by `deleted_at IS NULL`):
1. Files by type: `SELECT file_type, COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL GROUP BY file_type`
2. AI status: Three separate COUNT queries with different WHERE conditions on `ai_analyzed` and `ai_confirmed`
3. Top folders: `SELECT f.name, COUNT(e.id) FROM folders f LEFT JOIN embroidery_files e ON e.folder_id = f.id AND e.deleted_at IS NULL GROUP BY f.id ORDER BY COUNT(e.id) DESC LIMIT 10`
4. Missing metadata:
   - No tags: `SELECT COUNT(*) FROM embroidery_files e WHERE deleted_at IS NULL AND NOT EXISTS (SELECT 1 FROM file_tags WHERE file_id = e.id)`
   - No rating: `SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL AND rating IS NULL`
   - No description: `SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL AND (description IS NULL OR description = '')`
5. Storage by folder: `SELECT f.name, COALESCE(SUM(e.file_size_bytes), 0) FROM folders f LEFT JOIN embroidery_files e ON e.folder_id = f.id AND e.deleted_at IS NULL GROUP BY f.id ORDER BY SUM(e.file_size_bytes) DESC`
6. Recent imports: `SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL AND created_at >= datetime('now', '-7 days')`

**Step 4.2** — Register in `mod.rs` and `lib.rs`:

- Add `pub mod statistics;` to `src-tauri/src/commands/mod.rs`
- Add `commands::statistics::get_dashboard_stats` to `invoke_handler!`

### Phase 5: Dashboard Frontend Enhancement

**Step 5.1** — Types:

- In `src/types/index.ts`, add `DashboardStats` interface matching the Rust struct.

**Step 5.2** — `src/services/StatisticsService.ts`:

New service with `getDashboardStats()` invoke wrapper.

**Step 5.3** — `src/components/Dashboard.ts`:

Extend `load()` method (line 38) to also call `StatisticsService.getDashboardStats()`.

Extend `renderDashboard()` to add new sections:
1. **File Type Breakdown** — bar-style display showing embroidery vs. sewing pattern vs. other counts (using CSS bars, no chart library needed)
2. **AI Analysis Status** — three stat cards: "Nicht analysiert", "Analysiert", "Bestaetigt"
3. **Top 10 Ordner** — ranked list with file counts
4. **Fehlende Metadaten** — three stat cards showing files without tags, rating, or description
5. **Speicherverbrauch** — folder storage list with formatted sizes
6. **Kuerzlich importiert** — single stat card with last-7-days import count

**Step 5.4** — Toolbar button (optional):

The Dashboard is already shown when no folder is selected (line 22-35 of `Dashboard.ts`). Optionally add a "Bibliothek-Uebersicht" button to the System menu group in `src/components/Toolbar.ts` (after line 182) that emits `toolbar:show-dashboard`. The handler in `main.ts` would set `selectedFolderId` to null, which triggers the Dashboard to appear.

### Phase 6: Testing & Validation

1. Add `"smart_folders"` to the expected tables in the migration test (`migrations.rs` line 1352)
2. Update schema version assertions from 25 to 26
3. Run `cargo test` to verify migration + idempotency
4. Run `cargo check` for compile validation
5. Run `npm run build` for TypeScript type checking
6. Verify smart folder CRUD operations work end-to-end
7. Verify dashboard loads new statistics sections
8. Verify smart folder selection applies filter and FileList updates

### Implementation Order

The recommended implementation order is:
1. SearchParams extensions (Step 1) — prerequisite for smart folders
2. Smart folders backend (Steps 2.1-2.4)
3. Statistics backend (Steps 4.1-4.2)
4. Smart folders frontend (Steps 3.1-3.7)
5. Dashboard frontend (Steps 5.1-5.4)
6. Testing (Step 6)

This order ensures backend APIs are available before frontend integration, and that the SearchParams extensions (needed by both proposals) are in place first.
