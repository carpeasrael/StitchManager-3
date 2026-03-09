# Sprint 7 Analysis: Metadaten, Tags & KI-Vorbereitung

**Date:** 2026-03-09
**Sprint:** 7 (Weeks 8-9)
**Dependencies:** Sprint 4 (File-Commands), Sprint 6 (MetadataPanel base version)

---

## Problem Description

The application currently supports read-only viewing of embroidery file metadata. Users can browse files, view parsed stitch data (dimensions, stitch count, colors), and see thumbnails. However, there is no way to:

1. **Edit file metadata** (name, theme, description, license) through the UI
2. **Delete files** from the database
3. **Manage tags** (assign, remove, autocomplete) on files
4. **Access or modify application settings** from the frontend via Tauri commands
5. **Manage custom field definitions** for user-defined metadata
6. **Interact with a toolbar** for common actions (folder add, scan, save, AI analysis, settings)
7. **See status information** (file counts, scan status) in the status bar

The database schema already supports all of these features (tables: `settings`, `tags`, `file_tags`, `custom_field_definitions`, `custom_field_values`), but no Rust commands or frontend services expose write operations for files, tags, settings, or custom fields.

---

## Affected Components

### Backend (Rust / Tauri)
- `src-tauri/src/commands/files.rs` -- needs `update_file`, `delete_file`, `set_file_tags`, `get_thumbnail` commands
- `src-tauri/src/commands/settings.rs` -- **new file** for settings and custom field commands
- `src-tauri/src/commands/mod.rs` -- must register `settings` module
- `src-tauri/src/lib.rs` -- must register all new commands in `invoke_handler`
- `src-tauri/src/db/models.rs` -- may need a `FileUpdate` deserialization struct

### Frontend (TypeScript)
- `src/services/FileService.ts` -- needs `updateFile`, `deleteFile`, `setTags` functions
- `src/services/SettingsService.ts` -- **new file** wrapping settings/custom-field commands
- `src/components/MetadataPanel.ts` -- major extension: editable form, tag chips, custom fields, save button, dirty-state tracking
- `src/components/Toolbar.ts` -- **new file** for action buttons
- `src/components/StatusBar.ts` -- **new file** for status information
- `src/main.ts` -- must instantiate Toolbar and StatusBar components
- `src/types/index.ts` -- may need `CustomFieldDef` type and updates to `State`
- `src/styles/components.css` -- needs styles for toolbar, status bar, form fields, tag chips

### Configuration
- `src-tauri/capabilities/default.json` -- no changes needed (no new plugins; all commands go through the existing Tauri invoke handler)

---

## Root Cause / Rationale

Sprint 6 established a read-only MetadataPanel. The natural next step is enabling metadata editing, which requires:

- **Write commands on the backend** -- the existing `files.rs` only has read commands (`get_files`, `get_file`, `get_file_formats`, `get_file_colors`, `get_file_tags`). There are no update/delete commands for files, and no commands at all for settings or custom fields.
- **Tag management** -- the `tags` and `file_tags` tables exist but no command exposes tag creation or file-tag association. Tags need a "set" semantic (replace all tags for a file) rather than individual add/remove, to simplify the UI.
- **Settings access** -- `main.ts` currently reads settings via raw SQL through `tauri-plugin-sql`. This works but is inconsistent with the Rust-command pattern used everywhere else. A dedicated `SettingsService` backed by Rust commands provides validation and a cleaner API.
- **UI completeness** -- the toolbar area (`div.app-toolbar`) currently only contains SearchBar and FilterChips. The status bar (`div.app-status`) shows static text "Bereit". Both need to become functional components.

---

## Proposed Approach

### S7-T1: File update/delete Commands

**File:** `src-tauri/src/commands/files.rs`

1. Add a `FileUpdate` struct (with `#[derive(Deserialize)]`) containing optional fields: `name`, `theme`, `description`, `license`. This struct already exists in the frontend types (`src/types/index.ts`) as `FileUpdate`.
   - Add a corresponding Rust struct in `src-tauri/src/db/models.rs`:
     ```rust
     #[derive(Debug, Clone, Serialize, Deserialize)]
     #[serde(rename_all = "camelCase")]
     pub struct FileUpdate {
         pub name: Option<String>,
         pub theme: Option<String>,
         pub description: Option<String>,
         pub license: Option<String>,
     }
     ```

2. **`update_file(file_id, updates: FileUpdate) -> EmbroideryFile`**
   - Validate that at least one field is `Some`.
   - Build a dynamic `UPDATE embroidery_files SET ... WHERE id = ?` query updating only the provided fields. Always set `updated_at = datetime('now')`.
   - Return the updated file by re-querying with `row_to_file`.
   - Follow the pattern used in `folders.rs::update_folder`.

3. **`delete_file(file_id) -> ()`**
   - Execute `DELETE FROM embroidery_files WHERE id = ?1`.
   - Check `changes == 0` to return `AppError::NotFound`.
   - Cascade deletes will automatically remove `file_formats`, `file_thread_colors`, `file_tags`, `custom_field_values`, and `ai_analysis_results` entries.
   - Follow the pattern used in `folders.rs::delete_folder`.

4. **`set_file_tags(file_id, tag_names: Vec<String>) -> Vec<Tag>`**
   - Verify the file exists first (query `embroidery_files`).
   - Within a transaction:
     - Delete all existing `file_tags` rows for this file.
     - For each tag name: `INSERT OR IGNORE INTO tags (name) VALUES (?)`, then query the tag id.
     - Insert `file_tags` junction rows.
   - Return the resulting tags by re-querying (reuse the existing `get_file_tags` query pattern).

5. **`get_thumbnail(file_id) -> String`**
   - Query `thumbnail_path` from `embroidery_files`.
   - If `thumbnail_path` is `None` or file does not exist on disk, return an empty string or an error.
   - Read the thumbnail file from disk, Base64-encode it, and return as a data URI string (`data:image/png;base64,...`).

6. **Tests:** Add unit tests for each new command following the existing pattern (use `init_database_in_memory`, insert test data, verify SQL behavior).

### S7-T2: Settings-Commands

**New file:** `src-tauri/src/commands/settings.rs`

1. **`get_setting(key: String) -> String`**
   - Query `SELECT value FROM settings WHERE key = ?1`.
   - Return `AppError::NotFound` if not found.

2. **`set_setting(key: String, value: String) -> ()`**
   - Execute `INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))`.
   - This handles both insert and update via SQLite's `INSERT OR REPLACE`.

3. **`get_all_settings() -> HashMap<String, String>`**
   - Query all rows from `settings`, collect into a `HashMap<String, String>`.

4. **`get_custom_fields() -> Vec<CustomFieldDefinition>`**
   - Query `SELECT * FROM custom_field_definitions ORDER BY sort_order, name`.
   - Map rows to the existing `CustomFieldDefinition` model.

5. **`create_custom_field(name: String, field_type: String, options: Option<String>) -> CustomFieldDefinition`**
   - Validate `name` is not empty.
   - Insert into `custom_field_definitions`.
   - Return the created record.

6. **`delete_custom_field(field_id: i64) -> ()`**
   - Delete from `custom_field_definitions WHERE id = ?1`.
   - Cascade will remove associated `custom_field_values`.
   - Return `AppError::NotFound` if no rows affected.

7. **Registration:**
   - Add `pub mod settings;` to `src-tauri/src/commands/mod.rs`.
   - Add all 6 commands to `invoke_handler` in `src-tauri/src/lib.rs`.

8. **Tests:** Unit tests for each command using the in-memory database.

### S7-T3: SettingsService (Frontend)

**New file:** `src/services/SettingsService.ts`

1. Export functions wrapping each settings Tauri command:
   - `getSetting(key: string): Promise<string>`
   - `setSetting(key: string, value: string): Promise<void>`
   - `getAllSettings(): Promise<Record<string, string>>`
   - `getCustomFields(): Promise<CustomFieldDef[]>`
   - `createCustomField(name: string, fieldType: string, options?: string): Promise<CustomFieldDef>`
   - `deleteCustomField(fieldId: number): Promise<void>`

2. Follow the exact pattern used in `FileService.ts` and `ScannerService.ts` (import `invoke` from `@tauri-apps/api/core`).

3. Add a `CustomFieldDef` interface to `src/types/index.ts` if not already present:
   ```typescript
   export interface CustomFieldDef {
     id: number;
     name: string;
     fieldType: string;
     options: string | null;
     required: boolean;
     sortOrder: number;
     createdAt: string;
   }
   ```
   Note: The Rust model `CustomFieldDefinition` already exists in `models.rs` with `#[serde(rename_all = "camelCase")]`, so the camelCase mapping is automatic.

### S7-T4: MetadataPanel -- Form Extension

**File:** `src/components/MetadataPanel.ts`

1. **Extend `renderFileInfo`** to replace read-only info rows with editable form fields for:
   - `name` -- text input
   - `theme` -- text input
   - `description` -- textarea
   - `license` -- text input (or dropdown with common license options)

2. **Add a Tags section** with:
   - Display current tags as removable chips (small pill elements with an "x" button).
   - A text input for adding new tags with basic autocomplete (query all existing tags from DB for suggestions).
   - Pressing Enter or comma creates a new tag chip.

3. **Add a Custom Fields section**:
   - Fetch custom field definitions via `SettingsService.getCustomFields()`.
   - Render appropriate input for each field based on `fieldType` (text, number, select, checkbox).
   - Fetch existing values from a new `get_custom_field_values(file_id)` command or via `tauri-plugin-sql` direct query.

4. **Keep the existing read-only sections** (thumbnail, dimensions, stitch count, colors) as they are -- these come from parsing and should not be editable.

5. **Track dirty state**: Store original values on load. Compare current form values to detect changes. This state drives the save button (enabled/disabled).

6. **Load tags** alongside existing data in `onSelectionChanged` -- the current `Promise.all` already fetches file, formats, colors. Add `FileService.getTags(fileId)` to this call.

### S7-T5: Save Logic

**Files:** `src/components/MetadataPanel.ts`, `src/services/FileService.ts`

1. **Extend `FileService.ts`** with:
   - `updateFile(fileId: number, updates: FileUpdate): Promise<EmbroideryFile>` -- calls `invoke("update_file", { fileId, updates })`
   - `deleteFile(fileId: number): Promise<void>` -- calls `invoke("delete_file", { fileId })`
   - `setTags(fileId: number, tagNames: string[]): Promise<Tag[]>` -- calls `invoke("set_file_tags", { fileId, tagNames })`

2. **Add a Save button** to the MetadataPanel:
   - Positioned at the top or bottom of the form.
   - Disabled when form is clean (no changes).
   - On click: collect changed fields into a `FileUpdate` object, call `FileService.updateFile()`. If tags changed, call `FileService.setTags()`. Show success/error feedback.
   - After successful save, update `appState.files` to reflect the changes.

3. **Dirty-state tracking**:
   - On file load, snapshot `{ name, theme, description, license, tags }`.
   - On any input change, compare current values to snapshot.
   - Set a `dirty` boolean that controls save button state.
   - Optionally warn on file selection change if there are unsaved changes.

### S7-T6: Toolbar Component

**New file:** `src/components/Toolbar.ts`

1. Extend `Component` base class.

2. Render action buttons in the toolbar area. The toolbar currently hosts SearchBar and FilterChips (created in `main.ts`). The new Toolbar component should add action buttons **after** the search/filter area. Options:
   - **Option A:** Replace the current toolbar setup in `main.ts` entirely -- let the Toolbar component own SearchBar, FilterChips, and the action buttons.
   - **Option B (recommended):** Add a separate `toolbar-actions` container in `main.ts` after the existing search/filters, and mount the Toolbar component there.

3. Action buttons:
   - **Add Folder** -- emits an event or triggers folder creation dialog (reuse existing Sidebar logic).
   - **Scan** -- triggers directory scan for the selected folder.
   - **Save** -- triggers save on the MetadataPanel (alternative: keep save only in MetadataPanel).
   - **AI Analysis** -- placeholder button (disabled for now, Sprint 8+).
   - **Settings** -- opens a settings dialog/panel (can be a simple modal for now).

4. Style with icon-only buttons or icon+label, using Aurora theme variables. Add corresponding CSS to `src/styles/components.css`.

5. Subscribe to `appState` changes to enable/disable buttons contextually (e.g., Scan disabled when no folder selected).

### S7-T7: StatusBar Component

**New file:** `src/components/StatusBar.ts`

1. Extend `Component` base class.

2. Mount on the `div.app-status` element (currently showing static "Bereit" text).

3. Display:
   - Total file count from `appState.files.length` (update on `files` state change).
   - Selected folder name (if any).
   - Scan progress (subscribe to `EventBus` `scan:progress` events).
   - Last action status ("Gespeichert", "Scan abgeschlossen", etc.).

4. Subscribe to relevant state changes and EventBus events.

5. **Registration in `main.ts`:**
   - Query `div.app-status` and instantiate `new StatusBar(statusEl)`.

6. Add CSS styles for status bar layout (flex row, items spaced apart).

---

## Implementation Order

The tickets have the following dependency graph:

```
S7-T1 (file commands)  ──┐
                          ├── S7-T5 (save logic) ── S7-T4 (form extension)
S7-T2 (settings cmds) ──┤
                          └── S7-T3 (SettingsService)
S7-T6 (Toolbar) ── standalone (may reference save/scan events)
S7-T7 (StatusBar) ── standalone
```

Recommended order:
1. **S7-T1** -- Backend file update/delete/tags commands
2. **S7-T2** -- Backend settings commands
3. **S7-T3** -- Frontend SettingsService
4. **S7-T5** -- Frontend FileService extensions + save logic
5. **S7-T4** -- MetadataPanel form extension (depends on T1, T3, T5)
6. **S7-T7** -- StatusBar (independent)
7. **S7-T6** -- Toolbar (independent, but logically last as it ties actions together)

---

## Key Observations from Codebase Analysis

1. **`FileUpdate` type already exists** in `src/types/index.ts` (line 73-78) with fields `name`, `theme`, `description`, `license`. The Rust-side struct needs to mirror this.

2. **`CustomFieldDefinition` model already exists** in `src-tauri/src/db/models.rs` (line 117-127). The frontend `CustomFieldDef` type needs to be added to `src/types/index.ts`.

3. **Settings table already seeded** with 10 default values (theme_mode, ai_provider, ai_url, etc.) in `migrations.rs`.

4. **`appState` already has a `settings` field** of type `Record<string, string>` -- ready for use by `SettingsService`.

5. **The layout already has all grid areas** defined: `menu`, `toolbar`, `sidebar`, `center`, `right`, `status`. The `app-status` div exists in `index.html` and has basic styling in `layout.css`.

6. **No new Tauri plugins** are needed. All new functionality uses existing Rust commands registered via `invoke_handler`. The `capabilities/default.json` does not need changes.

7. **The `log` crate** is used in `folders.rs` (`log::warn!`). The same pattern should be used in new commands.

8. **Error handling** follows a consistent pattern: `AppError` variants with `lock_db(&db)?` for mutex access. All new commands should follow this.

9. **The toolbar area** in `main.ts` (lines 101-113) creates SearchBar and FilterChips directly. The Toolbar component (S7-T6) needs to integrate with this existing setup without breaking it.

10. **Tag autocomplete** will require a new command `get_all_tags() -> Vec<Tag>` (not listed in the tickets but needed for the autocomplete feature in S7-T4). This should be added as part of S7-T1.

---

## Solution Summary (Phase 4: Closure)

**Commit:** `afc4225` — Implement Sprint 7: Metadata editing, tags, settings, toolbar, and status bar

### What was implemented

All 7 tickets (S7-T1 through S7-T7) fully implemented and verified:

- **S7-T1 (File commands):** `update_file`, `delete_file`, `set_file_tags` (transaction-wrapped, deduplicated), `get_all_tags`, `get_thumbnail` (base64 crate). 4 new tests.
- **S7-T2 (Settings commands):** 6 commands for settings CRUD and custom field management with field_type validation. 3 new tests.
- **S7-T3 (SettingsService):** Frontend service with 6 methods wrapping Tauri invoke calls.
- **S7-T4 (MetadataPanel):** Full rewrite from read-only to editable form — name/theme/description/license fields, tag chips with autocomplete, dynamic custom field rendering (text/number/date/select).
- **S7-T5 (Save logic):** Dirty-state tracking via form snapshots, save button enables only on changes, persists to DB, updates app state, emits events.
- **S7-T6 (Toolbar):** 5 action buttons including native folder picker via `tauri-plugin-dialog`, scan with state management, AI button (disabled placeholder).
- **S7-T7 (StatusBar):** Folder name display, file count per format, event-driven updates.

### Review results

- 4 review agents passed with 0 findings (Round 2)
- 84/84 Rust tests passing
- TypeScript build clean
- 8 findings from Round 1 all fixed (transaction wrapping, deduplication, native dialog, base64 crate, button state, cross-field validation)
