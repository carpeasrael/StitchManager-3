# Analysis: Folder Creation & Type Awareness (Issue #126)

**Date:** 2026-03-19
**Author:** Analysis Agent (Phase 1)

---

## Problem Description

The folder creation workflow in StitchManager has two separate UX and data-model deficiencies:

### Deficiency 1: Primitive Folder Creation UI

Folder creation currently relies on native `window.prompt()` calls (Sidebar.ts lines 339-343) or a basic `open({ directory: true })` file picker (Toolbar.ts lines 371-393). Neither flow offers:

- A combined, cohesive dialog where the user can name the folder **and** browse for the directory in one step.
- A parent-folder dropdown for nesting.
- A folder-type selector.

The Sidebar path uses two sequential `prompt()` calls -- one for name, one for path -- which is clunky and error-prone (users may enter a non-existent path). The Toolbar path uses the native directory picker and auto-derives the folder name from the basename, bypassing any user customization.

### Deficiency 2: No Folder Type Distinction

The application manages two fundamentally different content types -- Stickmuster (embroidery patterns) and Schnittmuster (sewing patterns) -- but the `folders` table has no `folder_type` column. This means:

- All folders look the same in the Sidebar regardless of content type.
- There is no way to auto-activate relevant filter chips based on folder type.
- File counts cannot be split by content type.
- The `file_type` discriminator exists on `embroidery_files` (added in migration v9, values: `'embroidery'`, `'sewing_pattern'`) but there is no corresponding concept on folders.

---

## Affected Components

### Frontend

| File | Role | Lines of Interest |
|------|------|-------------------|
| `src/components/Sidebar.ts` | Folder tree rendering, `createFolder()` method | Lines 338-353: two `prompt()` calls |
| `src/components/Toolbar.ts` | Burger-menu "Ordner hinzufuegen" action | Lines 370-394: `addFolder()` with `open({ directory: true })` |
| `src/services/FolderService.ts` | Tauri invoke wrappers for folder CRUD | Lines 8-18: `create(name, path, parentId)` -- no `folderType` param |
| `src/types/index.ts` | `Folder` interface | Lines 1-9: no `folderType` field |
| `src/components/FilterChips.ts` | Format filter (PES/DST/JEF/VP3) | Lines 1-53: static FORMATS array, no folder-type awareness |
| `src/state/AppState.ts` | `State` interface | Referenced via `src/types/index.ts` line 666: `folders: Folder[]` |
| `src/styles/components.css` | Sidebar and dialog CSS | Lines 334-461 (sidebar), 1817-2089 (dialog base) |

### Backend

| File | Role | Lines of Interest |
|------|------|-------------------|
| `src-tauri/src/commands/folders.rs` | Folder CRUD commands | Lines 34-79: `create_folder` -- no `folder_type` parameter; Lines 7-17: `row_to_folder` maps 7 columns |
| `src-tauri/src/db/models.rs` | `Folder` struct | Lines 16-26: 7 fields, no `folder_type` |
| `src-tauri/src/db/migrations.rs` | Schema definitions | Lines 162-170 (v1): `folders` table with no `folder_type` column |

### Infrastructure

| File | Role | Lines of Interest |
|------|------|-------------------|
| `src-tauri/Cargo.toml` | Rust dependencies | Line 21: `tauri-plugin-dialog = "2"` already present |
| `package.json` | npm dependencies | Line 16: `@tauri-apps/plugin-dialog: "^2.6.0"` already present |
| `src-tauri/capabilities/default.json` | Tauri permissions | Line 8: `"dialog:default"` already granted |
| `src-tauri/src/lib.rs` | Plugin registration | Line 21: `tauri_plugin_dialog::init()` already registered |

---

## Root Cause / Rationale

### Why the current folder creation UX is insufficient

The original MVP design used the simplest possible UI (`window.prompt()`) to get folder creation working quickly. As the application matured to support sewing patterns alongside embroidery files, the simple prompt approach was never revisited. The Toolbar added a directory picker as an alternative but these two entry points are inconsistent with each other and neither offers the full set of options needed.

### Why folder type awareness is needed

Migration v9 (in `migrations.rs`) added a `file_type` discriminator column to `embroidery_files` with values `'embroidery'` and `'sewing_pattern'`. However, no corresponding metadata was added to the `folders` table. Without folder-level type awareness:

1. Users cannot visually distinguish folder types in the sidebar.
2. The FilterChips component cannot auto-activate format filters when a folder of a known type is selected.
3. File counts are aggregated without regard to content type, making mixed folders opaque.

### Why `@tauri-apps/plugin-dialog` is already available

All three wiring points for the dialog plugin are already in place:
- **Cargo.toml**: `tauri-plugin-dialog = "2"` (line 21)
- **lib.rs**: `.plugin(tauri_plugin_dialog::init())` (line 21)
- **capabilities/default.json**: `"dialog:default"` (line 8)
- **package.json**: `"@tauri-apps/plugin-dialog": "^2.6.0"` (line 16)

The `open()` function from this plugin is already used in `SettingsDialog.ts` (line 8, line 372) and `Toolbar.ts` (line 5, line 372). No new plugin installation is required.

---

## Proposed Approach

### Step 1: Database Migration (v25) -- Add `folder_type` Column

**File:** `src-tauri/src/db/migrations.rs`

1. Increment `CURRENT_VERSION` from 24 to 25.
2. Add `apply_v25()` function:
   ```sql
   ALTER TABLE folders ADD COLUMN folder_type TEXT NOT NULL DEFAULT 'mixed';
   ```
   Valid values: `'embroidery'`, `'sewing_pattern'`, `'mixed'`.
   Default `'mixed'` ensures backward compatibility -- existing folders will be typed as mixed.
3. Insert version record into `schema_version`.
4. Add migration call `if current < 25 { apply_v25(conn)?; }` in `run_migrations`.

### Step 2: Update Rust `Folder` Model and Commands

**File:** `src-tauri/src/db/models.rs`

1. Add `pub folder_type: String` field to the `Folder` struct (after `sort_order`).

**File:** `src-tauri/src/commands/folders.rs`

1. Update `FOLDER_SELECT` constant to include `folder_type` (8 columns total):
   ```
   SELECT id, name, path, parent_id, sort_order, folder_type, created_at, updated_at FROM folders
   ```
2. Update `row_to_folder` to extract the new column at index 5, shifting `created_at` to index 6 and `updated_at` to index 7.
3. Update `create_folder` command:
   - Add `folder_type: Option<String>` parameter (default `"mixed"` if None).
   - Validate that the value is one of `"embroidery"`, `"sewing_pattern"`, `"mixed"`.
   - Include `folder_type` in the INSERT statement.
4. Update `update_folder` command:
   - Add `folder_type: Option<String>` parameter.
   - If provided and valid, update the `folder_type` column.
5. Update existing tests to account for the new column.

### Step 3: Update TypeScript `Folder` Interface and FolderService

**File:** `src/types/index.ts`

1. Add `FolderType` union type:
   ```ts
   export type FolderType = 'embroidery' | 'sewing_pattern' | 'mixed';
   ```
2. Add `folderType: FolderType` field to the `Folder` interface (after `sortOrder`).

**File:** `src/services/FolderService.ts`

1. Update `create()` to accept an optional `folderType` parameter and pass it to the Tauri command.
2. Update `update()` to accept an optional `folderType` parameter.

### Step 4: Create `FolderDialog` Component

**File:** `src/components/FolderDialog.ts` (new file)

Create a modal dialog following the existing pattern used by `SettingsDialog`, `BatchDialog`, and `AiPreviewDialog`:

- **Structure:** `dialog-overlay` > `dialog dialog-folder` with `role="dialog"`, `aria-modal="true"`.
- **Header:** Title "Neuer Ordner" (or "Ordner bearbeiten" for edit mode) with close button.
- **Body** containing a form with:
  1. **Ordnername** -- text input, pre-filled from directory basename when "Durchsuchen" is used.
  2. **Pfad** -- text input (read-only) + "Durchsuchen..." button that calls `open({ directory: true })` from `@tauri-apps/plugin-dialog`. When the user picks a directory, auto-fill the name field with the directory basename if the name is still empty or matches the previous auto-derived name.
  3. **Uebergeordneter Ordner** -- `<select>` dropdown populated from `appState.get("folders")`, with an empty "-- Keiner --" option (null parent). Defaults to the currently selected folder or none.
  4. **Ordnertyp** -- `<select>` with three options:
     - `mixed` / "Gemischt" (default)
     - `embroidery` / "Stickmuster"
     - `sewing_pattern` / "Schnittmuster"
- **Footer:** "Abbrechen" (secondary) and "Erstellen" (primary) buttons.
- **Behavior:**
  - Focus trap using existing `trapFocus()` utility from `src/utils/focus-trap.ts`.
  - Escape key closes the dialog (via focus-trap keydown or overlay click handler).
  - On submit: validate name is non-empty, path is non-empty; call `FolderService.create(name, path, parentId, folderType)`; reload folders; close dialog; show success toast.
  - Static `FolderDialog.open()` method (same pattern as `SettingsDialog.open()`).

### Step 5: Wire Up Sidebar and Toolbar to Use FolderDialog

**File:** `src/components/Sidebar.ts`

1. Replace `createFolder()` method body (lines 338-353) with a call to `FolderDialog.open()`.
2. Remove the two `prompt()` calls entirely.

**File:** `src/components/Toolbar.ts`

1. Replace `addFolder()` method body (lines 370-394) with a call to `FolderDialog.open()`.
2. Remove the inline `open({ directory: true })` call and manual folder name derivation.

Both components should import `FolderDialog` and delegate all folder creation logic to it, ensuring a single consistent entry point.

### Step 6: Visual Folder Type Distinction in Sidebar

**File:** `src/components/Sidebar.ts`

1. In the folder rendering loop (lines 110-196), add a type badge/icon element before or after the folder name span:
   - `embroidery`: A small needle/thread icon or badge text "S" (Stickmuster).
   - `sewing_pattern`: A scissors icon or badge text "N" (Schnittmuster / Naehmuster).
   - `mixed`: A combined icon or badge text "G" (Gemischt).
   Use a `<span class="folder-type-badge folder-type-{type}">` element.
2. Add corresponding CSS classes in `src/styles/components.css` for the badge styling:
   - Small pill badge next to the folder name.
   - Distinct colors for each type (using existing design tokens).

### Step 7: Folder-Type-Aware File Counts (Optional Enhancement)

**File:** `src-tauri/src/commands/folders.rs`

1. Add a new command `get_all_folder_file_counts_by_type` that returns counts split by `file_type`:
   ```sql
   SELECT folder_id, file_type, COUNT(*)
   FROM embroidery_files
   WHERE deleted_at IS NULL
   GROUP BY folder_id, file_type
   ```
   Return type: `HashMap<i64, HashMap<String, i64>>`.

**File:** `src/services/FolderService.ts`

1. Add `getAllFileCountsByType()` wrapper.

**File:** `src/components/Sidebar.ts`

1. Update `loadCounts()` and `render()` to optionally show split counts (e.g., "12S / 3N" or stacked badges).

### Step 8: Auto-Activate Filter Chips Based on Folder Type (Optional Enhancement)

**File:** `src/components/FilterChips.ts`

1. Subscribe to `selectedFolderId` changes.
2. When a folder is selected, look up its `folderType` from `appState.get("folders")`.
3. If folder type is `embroidery`, auto-set `formatFilter` to show only embroidery formats (PES/DST/JEF/VP3).
4. If folder type is `sewing_pattern`, auto-set a file-type filter for sewing patterns (likely via `searchParams.fileType`).
5. If `mixed`, leave filters unchanged.

This behavior should be additive (user can override) and only trigger on folder selection change, not on every render.

### Step 9: CSS Additions

**File:** `src/styles/components.css`

1. Add `.dialog-folder` sizing rules (similar to `.dialog-settings`, approximately 480px width).
2. Add `.folder-dialog-browse-row` styling for the path input + browse button layout.
3. Add `.folder-type-badge` and `.folder-type-embroidery`, `.folder-type-sewing_pattern`, `.folder-type-mixed` styling.
4. Ensure dialog input styling uses existing `.settings-input` and `.settings-form-group` patterns for consistency.

### Step 10: Update Rust Tests

**File:** `src-tauri/src/commands/folders.rs` (test module)

1. Update `test_folder_crud_cycle` to include `folder_type` in INSERT and verify it in SELECT.
2. Add a new test `test_folder_type_default` verifying that folders created without explicit type get `'mixed'`.
3. Add a new test `test_folder_type_create_and_update` verifying creation with each type and update between types.

### Implementation Priority

Steps 1-6 are **core** (required to resolve the issue). Steps 7-8 are **enhancements** that build on the core and can be deferred if needed. Step 9-10 are required support work for the core steps.

**Recommended order:** 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 9 -> 10 -> 7 -> 8

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Migration v25 fails on existing databases | `ALTER TABLE ... ADD COLUMN ... DEFAULT` is safe for SQLite; existing rows get the default value |
| Breaking change to `row_to_folder` column indices | Update all column index references atomically; run `cargo test` to verify |
| Two entry points (Sidebar + Toolbar) for folder creation | Both delegate to `FolderDialog.open()`, eliminating duplication |
| `@tauri-apps/plugin-dialog` `open()` may return different types | Already handled in Toolbar.ts (line 379): `typeof selected === "string" ? selected : String(selected)` -- reuse this pattern |
| FilterChips auto-activation may override user preference | Only auto-activate on folder selection change, not on filter interaction; user can always override manually |

---

## Definition of Done

- [ ] `folders` table has `folder_type TEXT NOT NULL DEFAULT 'mixed'` column (migration v25)
- [ ] Rust `Folder` model and commands support `folder_type`
- [ ] TypeScript `Folder` interface and `FolderService` support `folderType`
- [ ] `FolderDialog` component provides cohesive folder creation UI with name, path (browse), parent, and type
- [ ] Both Sidebar.ts and Toolbar.ts use `FolderDialog` instead of `prompt()` / inline `open()`
- [ ] Folder type is visually indicated in the Sidebar with icon/badge
- [ ] All existing tests pass; new tests cover folder type CRUD
- [ ] `cargo check`, `cargo test`, `npm run build` all pass
