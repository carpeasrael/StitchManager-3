# Sprint 5 Analysis: Project Management

**Date:** 2026-03-16
**Sprint:** S5 — Project Management
**Requirements:** UR-017, UR-019, UR-052, UR-053, UR-054
**Schema baseline:** v11

---

## Overview

Sprint 5 introduces a project management layer on top of the existing pattern library. Users can create projects linked to patterns, store project-specific information (chosen size, fabric, notes), duplicate projects without duplicating files, and organize patterns into named collections. This requires new database tables, Rust commands, frontend services, components, and state management.

---

## S5-01: Projects Data Model

### Problem Description

There is no concept of a "project" in the current data model. The `embroidery_files` table stores pattern metadata, but users cannot track project-specific work (e.g., "I am sewing a size M version of this dress pattern using linen"). UR-052 requires project entries linked to sewing patterns; UR-053 requires structured project data; UR-019 requires project notes separate from pattern metadata.

### Affected Components

**New files:**
- `src-tauri/src/commands/projects.rs` — CRUD Tauri commands for projects and project details

**Modified files:**
- `src-tauri/src/db/migrations.rs` — Migration v12: `projects` and `project_details` tables
- `src-tauri/src/db/models.rs` — `Project`, `ProjectDetail`, `ProjectCreate`, `ProjectUpdate` structs
- `src-tauri/src/commands/mod.rs` — Add `pub mod projects`
- `src-tauri/src/lib.rs` — Register project commands in `invoke_handler`

### Root Cause / Rationale

The current data model is pattern-centric. A project is a distinct entity: it references a pattern file but carries its own lifecycle (status, notes, customization). Without a separate `projects` table, users would have to overload pattern metadata fields or use custom fields, which conflates pattern data with project data (violating UR-019's separation requirement).

### Proposed Approach

1. **Migration v12** — Create two tables in a single migration:

   ```sql
   CREATE TABLE projects (
       id              INTEGER PRIMARY KEY AUTOINCREMENT,
       name            TEXT NOT NULL,
       pattern_file_id INTEGER REFERENCES embroidery_files(id) ON DELETE SET NULL,
       status          TEXT NOT NULL DEFAULT 'not_started',
       notes           TEXT,
       created_at      TEXT NOT NULL DEFAULT (datetime('now')),
       updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
   );
   CREATE INDEX idx_projects_pattern_file_id ON projects(pattern_file_id);
   CREATE INDEX idx_projects_status ON projects(status);

   CREATE TABLE project_details (
       id         INTEGER PRIMARY KEY AUTOINCREMENT,
       project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
       key        TEXT NOT NULL,
       value      TEXT,
       UNIQUE(project_id, key)
   );
   CREATE INDEX idx_project_details_project_id ON project_details(project_id);
   ```

   - `pattern_file_id` uses `ON DELETE SET NULL` so deleting a pattern does not destroy the project — the project retains its notes/details but loses its pattern link. This is intentional: a user may keep sewing notes even after removing a pattern from the library.
   - `status` values: `not_started`, `planned`, `in_progress`, `completed`, `archived` (matching UR-018).

2. **Rust models** (`src-tauri/src/db/models.rs`):

   ```rust
   pub struct Project {
       pub id: i64,
       pub name: String,
       pub pattern_file_id: Option<i64>,
       pub status: String,
       pub notes: Option<String>,
       pub created_at: String,
       pub updated_at: String,
   }

   pub struct ProjectDetail {
       pub id: i64,
       pub project_id: i64,
       pub key: String,
       pub value: Option<String>,
   }

   pub struct ProjectCreate {
       pub name: String,
       pub pattern_file_id: Option<i64>,
       pub status: Option<String>,
       pub notes: Option<String>,
   }

   pub struct ProjectUpdate {
       pub name: Option<String>,
       pub status: Option<String>,
       pub notes: Option<String>,
   }
   ```

3. **Rust commands** (`src-tauri/src/commands/projects.rs`):

   Following the `folders.rs` pattern (lock_db, query, map rows):
   - `get_projects(db, status_filter: Option<String>, pattern_file_id: Option<i64>) -> Vec<Project>` — list all projects, optionally filtered
   - `get_project(db, project_id) -> Project` — single project with details
   - `create_project(db, project: ProjectCreate) -> Project`
   - `update_project(db, project_id, update: ProjectUpdate) -> Project`
   - `delete_project(db, project_id) -> ()`
   - `get_project_details(db, project_id) -> Vec<ProjectDetail>`
   - `set_project_detail(db, project_id, key, value) -> ProjectDetail`
   - `delete_project_detail(db, project_id, key) -> ()`
   - `get_projects_for_pattern(db, pattern_file_id) -> Vec<Project>` — all projects linked to a pattern

4. **Registration** — Add all commands to `lib.rs` invoke_handler and `pub mod projects` to `commands/mod.rs`.

5. **Tests** — Add tests in `commands/projects.rs`:
   - CRUD cycle (create, read, update, delete)
   - Cascade delete removes project_details
   - ON DELETE SET NULL for pattern_file_id
   - Status filter query
   - Unique constraint on (project_id, key) for details

6. **Update migration tests** — Update `test_init_database_creates_tables` expected list and `test_schema_version_is_eleven` to version 12.

---

## S5-02: Project-Specific Information

### Problem Description

UR-053 specifies that users must be able to record project-specific data: chosen size, fabric used, planned modifications, cut version, and sewing notes. This data is distinct from pattern metadata (UR-019) and must be stored per-project, not per-pattern.

### Affected Components

**New files:**
- `src/components/ProjectPanel.ts` — Project detail editing UI
- `src/services/ProjectService.ts` — Tauri invoke wrappers for project commands

**Modified files:**
- `src/types/index.ts` — Add `Project`, `ProjectDetail`, `ProjectCreate`, `ProjectUpdate` interfaces
- `src/state/AppState.ts` — Add `selectedProjectId` to State, possibly `projects` array
- `src/components/MetadataPanel.ts` — Add "Projects" section or link to ProjectPanel

### Root Cause / Rationale

The `project_details` key-value table (S5-01) provides the storage mechanism. S5-02 is the frontend layer that makes this data accessible. Pre-defined keys (`chosen_size`, `fabric_used`, `planned_modifications`, `cut_version`) provide structure, while the key-value model allows future extension without schema changes. Free-form notes are stored in `projects.notes`.

### Proposed Approach

1. **TypeScript interfaces** (`src/types/index.ts`):

   ```typescript
   export interface Project {
     id: number;
     name: string;
     patternFileId: number | null;
     status: string;
     notes: string | null;
     createdAt: string;
     updatedAt: string;
   }

   export interface ProjectDetail {
     id: number;
     projectId: number;
     key: string;
     value: string | null;
   }

   export interface ProjectCreate {
     name: string;
     patternFileId?: number | null;
     status?: string;
     notes?: string;
   }

   export interface ProjectUpdate {
     name?: string;
     status?: string;
     notes?: string;
   }
   ```

2. **State extension** (`src/state/AppState.ts` and `src/types/index.ts`):
   - Add `selectedProjectId: number | null` and `projects: Project[]` to the `State` interface
   - Initialize both in `initialState`

3. **Service layer** (`src/services/ProjectService.ts`):

   Following the `FolderService.ts` pattern:
   - `getAll(statusFilter?, patternFileId?) -> Project[]`
   - `getOne(projectId) -> Project`
   - `create(data: ProjectCreate) -> Project`
   - `update(projectId, data: ProjectUpdate) -> Project`
   - `remove(projectId) -> void`
   - `getDetails(projectId) -> ProjectDetail[]`
   - `setDetail(projectId, key, value) -> ProjectDetail`
   - `deleteDetail(projectId, key) -> void`
   - `getForPattern(patternFileId) -> Project[]`

4. **ProjectPanel component** (`src/components/ProjectPanel.ts`):

   Extends `Component`. Renders when a project is selected. Contains:
   - Project name (editable text input)
   - Status dropdown: `not_started`, `planned`, `in_progress`, `completed`, `archived` (German labels)
   - Pattern link: shows linked pattern name, click navigates to pattern
   - Structured fields section with labeled inputs for the four predefined keys:
     - Gewaehlte Groesse (chosen_size)
     - Stoff (fabric_used)
     - Geplante Aenderungen (planned_modifications)
     - Schnittversion (cut_version)
   - Free-form notes textarea (projects.notes)
   - Save button, dirty tracking (same pattern as MetadataPanel's FormSnapshot)
   - Delete project button with confirmation

5. **MetadataPanel integration**: When viewing a pattern file, add a "Projekte" subsection showing project count with a button to open the project list for that pattern.

---

## S5-03: Project Duplication

### Problem Description

UR-054 requires duplicating a project from the same sewing pattern without duplicating source files. Users often sew the same pattern multiple times (different sizes, fabrics). They need to create a new project that references the same pattern, optionally pre-filled with data from a previous project.

### Affected Components

**Modified files:**
- `src-tauri/src/commands/projects.rs` — Add `duplicate_project` command
- `src/services/ProjectService.ts` — Add `duplicate(projectId, newName) -> Project`
- `src/components/ProjectPanel.ts` — "Duplicate" button
- `src/components/ProjectListView.ts` (S5-05) — "New Project from Pattern" action

### Root Cause / Rationale

Without duplication, users would have to manually re-enter project details when sewing the same pattern again. The duplication references the same `pattern_file_id` (no file copy) and copies all `project_details` rows to the new project. The name is customized (e.g., appending " (2)" or user-provided name).

### Proposed Approach

1. **Rust command** — `duplicate_project(db, project_id, new_name: Option<String>) -> Project`:
   - Read the source project
   - Insert a new `projects` row with:
     - `name`: `new_name` or `"{source.name} (Kopie)"`
     - `pattern_file_id`: same as source
     - `status`: reset to `not_started`
     - `notes`: copied from source (or empty, configurable)
   - Copy all `project_details` rows from source to new project
   - Return the new project

2. **Frontend**:
   - `ProjectService.duplicate(projectId, newName?) -> Project`
   - ProjectPanel: "Duplizieren" button that prompts for a new name, then calls duplicate
   - PatternFileId is preserved (FK to same embroidery_files row) — no file duplication

3. **"New Project from Pattern" action**:
   - In MetadataPanel's project section (or FileList context): "Neues Projekt" button
   - Calls `create_project` with `pattern_file_id` set to the current file
   - Optionally shows a dialog to select a previous project as template (calls `duplicate_project` variant)

---

## S5-04: Collections / Pattern Grouping

### Problem Description

UR-017 requires grouping patterns into categories or collections. The existing folder structure is filesystem-based (a file belongs to exactly one folder). Collections are a logical grouping layer: a file can belong to multiple collections (many-to-many). This enables cross-folder organization like "Summer Dresses", "Baby Gifts", "To Sell".

### Affected Components

**New files:**
- `src/components/CollectionSection.ts` — Sidebar section for collections
- `src/services/CollectionService.ts` — Tauri invoke wrappers

**Modified files:**
- `src-tauri/src/db/migrations.rs` — Migration v12 (same migration): `collections` and `collection_items` tables
- `src-tauri/src/db/models.rs` — `Collection`, `CollectionItem` structs
- `src-tauri/src/commands/projects.rs` (or new `collections.rs`) — Collection CRUD commands
- `src-tauri/src/commands/mod.rs` — Add module
- `src-tauri/src/lib.rs` — Register commands
- `src/components/Sidebar.ts` — Render collections section below folders
- `src/types/index.ts` — Add `Collection` interface
- `src/state/AppState.ts` — Add `collections: Collection[]`, `selectedCollectionId: number | null`

### Root Cause / Rationale

Folders are tied to the filesystem and enforce single-parent hierarchy. Collections are user-defined logical groups with no filesystem coupling. A pattern can be in "Summer Projects" and "Quick Gifts" simultaneously. This is a standard many-to-many relationship distinct from the folder tree.

### Proposed Approach

1. **Migration v12** (combined with S5-01 tables):

   ```sql
   CREATE TABLE collections (
       id          INTEGER PRIMARY KEY AUTOINCREMENT,
       name        TEXT NOT NULL,
       description TEXT,
       created_at  TEXT NOT NULL DEFAULT (datetime('now'))
   );

   CREATE TABLE collection_items (
       collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
       file_id       INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
       added_at      TEXT NOT NULL DEFAULT (datetime('now')),
       PRIMARY KEY (collection_id, file_id)
   );
   CREATE INDEX idx_collection_items_file_id ON collection_items(file_id);
   ```

2. **Rust models**:

   ```rust
   pub struct Collection {
       pub id: i64,
       pub name: String,
       pub description: Option<String>,
       pub created_at: String,
   }
   ```

3. **Rust commands** (`src-tauri/src/commands/collections.rs`):
   - `get_collections(db) -> Vec<Collection>` — list all, with item counts via LEFT JOIN
   - `create_collection(db, name, description) -> Collection`
   - `update_collection(db, collection_id, name, description) -> Collection`
   - `delete_collection(db, collection_id) -> ()`
   - `add_to_collection(db, collection_id, file_id) -> ()`
   - `remove_from_collection(db, collection_id, file_id) -> ()`
   - `get_collection_files(db, collection_id) -> Vec<EmbroideryFile>` — files in a collection, using FILE_SELECT_ALIASED with JOIN
   - `get_file_collections(db, file_id) -> Vec<Collection>` — collections a file belongs to

4. **Frontend service** (`src/services/CollectionService.ts`):
   - Mirrors the Rust commands as async invoke wrappers

5. **TypeScript interface** (`src/types/index.ts`):

   ```typescript
   export interface Collection {
     id: number;
     name: string;
     description: string | null;
     itemCount: number;
     createdAt: string;
   }
   ```

6. **State** — Add `collections` and `selectedCollectionId` to `State` interface and `initialState`.

7. **Sidebar integration** (`src/components/Sidebar.ts`):
   - After the folder list, render a "Sammlungen" (Collections) section with a similar structure:
     - Header with "Sammlungen" title and "+" button
     - List of collections with item counts and delete buttons
   - Clicking a collection sets `selectedCollectionId` and loads files via `get_collection_files`
   - When a collection is selected, the FileList shows only files in that collection (deselects folder)

8. **MetadataPanel integration**: Show which collections the selected file belongs to, with options to add/remove from collections.

9. **FileList interaction**: When collection is selected, the file query uses `get_collection_files` instead of the standard folder-based query. This requires modifying the `reloadFiles()` function in `main.ts` to check for `selectedCollectionId`.

---

## S5-05: Project List and Navigation

### Problem Description

Users need a way to browse all projects, filter by status, and navigate between projects and their linked patterns. Currently there is no project-level view — only the pattern file list exists.

### Affected Components

**New files:**
- `src/components/ProjectListView.ts` — Project list/dashboard component

**Modified files:**
- `src/main.ts` — Event handlers for project navigation, component init
- `src/components/Toolbar.ts` — Add "Projects" menu item
- `src/components/Sidebar.ts` — Possibly a "Projekte" section
- `src/components/MetadataPanel.ts` — "Show Projects" link from pattern detail
- `src/state/AppState.ts` — `viewMode: 'files' | 'projects'` state
- `src/types/index.ts` — ViewMode type
- `index.html` — Possibly a container for the project view (or reuse center panel)
- `src/styles/components.css` — Project list styles

### Root Cause / Rationale

S5-01 and S5-02 provide the data model and editing UI, but users also need an overview. A project dashboard shows status distribution (how many planned, in progress, completed) and allows quick filtering. The "Show Projects" link from a pattern detail view connects the pattern-centric and project-centric workflows.

### Proposed Approach

1. **View mode state**: Add `viewMode: 'files' | 'projects'` to `State`. Default `'files'`. When `'projects'`, the center panel shows ProjectListView instead of FileList/Dashboard.

2. **ProjectListView component** (`src/components/ProjectListView.ts`):
   - Extends `Component`
   - Renders a list of all projects, each showing:
     - Project name
     - Status badge (colored by status)
     - Linked pattern name (if any)
     - Created date
     - Last updated date
   - **Status filter bar** at top: "Alle", "Nicht begonnen", "Geplant", "In Arbeit", "Fertig", "Archiviert"
   - Clicking a project sets `selectedProjectId` and shows the ProjectPanel in the right panel
   - **Status summary** at top: counts per status (mini dashboard)
   - **"Neues Projekt" button** to create a project (with optional pattern selection)

3. **Navigation**:
   - **Toolbar/Burger menu**: Add "Projekte" item that sets `viewMode` to `'projects'`
   - **MetadataPanel**: When viewing a pattern, show project count. "Projekte anzeigen" button:
     - Sets `viewMode` to `'projects'`
     - Filters project list to only projects for that pattern
   - **ProjectListView**: Clicking a project's pattern name navigates back to file view with that pattern selected

4. **Center panel switching** (`src/main.ts`):
   - Subscribe to `viewMode` changes
   - When `'files'`: show Dashboard + FileList (current behavior)
   - When `'projects'`: show ProjectListView
   - The right panel switches between MetadataPanel (files mode) and ProjectPanel (projects mode)

5. **Event flow**:
   - `EventBus.emit("view:projects")` — switches to project view
   - `EventBus.emit("view:files")` — switches back to file view
   - `EventBus.emit("project:select", projectId)` — selects a project
   - `EventBus.emit("project:show-pattern", patternFileId)` — navigates from project to pattern

6. **Styles**: Project list cards, status badges with color coding:
   - `not_started` — gray
   - `planned` — blue
   - `in_progress` — orange/amber
   - `completed` — green
   - `archived` — muted/dimmed

---

## Migration Summary (v12)

All four new tables are created in a single migration `apply_v12`:

| Table | Purpose | Foreign Keys |
|-------|---------|-------------|
| `projects` | Project entries linked to patterns | `pattern_file_id -> embroidery_files(id) ON DELETE SET NULL` |
| `project_details` | Key-value project data | `project_id -> projects(id) ON DELETE CASCADE` |
| `collections` | Named pattern groups | None |
| `collection_items` | Many-to-many pattern-collection | `collection_id -> collections(id) ON DELETE CASCADE`, `file_id -> embroidery_files(id) ON DELETE CASCADE` |

Update `CURRENT_VERSION` to `12`.

---

## File Change Summary

### New Files (7)

| File | Purpose |
|------|---------|
| `src-tauri/src/commands/projects.rs` | Project CRUD commands |
| `src-tauri/src/commands/collections.rs` | Collection CRUD commands |
| `src/services/ProjectService.ts` | Project invoke wrappers |
| `src/services/CollectionService.ts` | Collection invoke wrappers |
| `src/components/ProjectPanel.ts` | Project detail editing |
| `src/components/ProjectListView.ts` | Project list/dashboard |
| `src/components/CollectionSection.ts` | Sidebar collection list |

### Modified Files (10)

| File | Changes |
|------|---------|
| `src-tauri/src/db/migrations.rs` | Add `apply_v12`, update `CURRENT_VERSION` to 12, update tests |
| `src-tauri/src/db/models.rs` | Add `Project`, `ProjectDetail`, `ProjectCreate`, `ProjectUpdate`, `Collection` structs |
| `src-tauri/src/commands/mod.rs` | Add `pub mod projects; pub mod collections;` |
| `src-tauri/src/lib.rs` | Register all new commands in `invoke_handler` |
| `src/types/index.ts` | Add `Project`, `ProjectDetail`, `ProjectCreate`, `ProjectUpdate`, `Collection` interfaces; extend `State` |
| `src/state/AppState.ts` | Add new state fields: `projects`, `collections`, `selectedProjectId`, `selectedCollectionId`, `viewMode` |
| `src/components/Sidebar.ts` | Add collections section below folders |
| `src/components/MetadataPanel.ts` | Add project count/link section for selected pattern |
| `src/main.ts` | View mode switching, project event handlers, collection-aware file reloading |
| `src/styles/components.css` | Styles for ProjectPanel, ProjectListView, CollectionSection, status badges |

---

## Implementation Order

1. **S5-01** first (data model) — all other issues depend on it
2. **S5-04** next (collections) — independent of projects, extends the migration
3. **S5-02** (project-specific information) — depends on S5-01 models
4. **S5-03** (duplication) — depends on S5-01 and S5-02
5. **S5-05** last (list/navigation) — depends on all prior issues for full integration

---

## Risk Notes

- **ON DELETE SET NULL for pattern_file_id**: When a pattern is deleted, projects survive but lose their link. The UI must handle `patternFileId === null` gracefully (show "Pattern removed" instead of crashing).
- **View mode complexity**: Switching between files and projects view reuses the center panel. Care must be taken to properly destroy/create components during mode switches to avoid memory leaks.
- **Collection + folder interaction**: When a collection is selected, folder selection should be cleared (and vice versa). The `reloadFiles` function needs branching logic.
- **State growth**: Adding 5 new state fields requires updating `initialState` and ensuring all subscribers handle initial null/empty values.
