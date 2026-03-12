# Analysis: Global/Cross-Folder Search (Issue #22)

## Problem Description

Currently, the search functionality in StitchManager is scoped to the currently selected folder. When a user types a query in the search bar, results only appear from the folder selected in the sidebar. If no folder is selected (`selectedFolderId` is `null`), the backend returns files across all folders (since the `folder_id` condition is skipped), but the UI requires a folder to be selected before files are displayed. This means users cannot search across their entire embroidery file library at once.

The feature request asks for the ability to search across all folders simultaneously, regardless of which folder is currently selected.

## Affected Components

### Frontend
- **`src/components/Sidebar.ts`** -- Folder selection; needs an "All Folders" / deselect option to allow cross-folder browsing
- **`src/components/FileList.ts`** -- `loadFiles()` method passes `selectedFolderId` to `FileService.getFiles()`; needs to pass `null` when global search is active
- **`src/components/SearchBar.ts`** -- Search input; may need a toggle or visual indicator for global vs. folder-scoped search
- **`src/components/StatusBar.ts`** -- Displays "Kein Ordner ausgewahlt" when no folder selected; needs to show meaningful text during global search (e.g., "Alle Ordner")
- **`src/main.ts`** -- `reloadFiles()` function reads `selectedFolderId` and passes it to `FileService.getFiles()`; needs same logic adjustment
- **`src/state/AppState.ts`** -- State definition; `selectedFolderId: number | null` already supports `null`, but the UI doesn't leverage `null` as "all folders"
- **`src/types/index.ts`** -- `State` interface; no changes needed since `selectedFolderId` is already `number | null`

### Backend
- **`src-tauri/src/commands/files.rs`** -- `query_files_impl()` already handles `folder_id: None` correctly by simply not adding the `folder_id` condition, returning files from all folders. **No backend changes needed.**
- **`src/services/FileService.ts`** -- `getFiles()` already accepts `folderId?: number | null`. **No changes needed.**

## Root Cause / Rationale

The backend already supports cross-folder queries -- when `folder_id` is `None`, the SQL query omits the `e.folder_id = ?` condition and returns files from all folders. The limitation is purely in the frontend:

1. **Sidebar has no "All Folders" entry** -- there is no UI element to deselect the current folder or select "all folders." Once a folder is clicked, `selectedFolderId` is set and can never be unset back to `null`.

2. **FileList always passes selectedFolderId** -- in `loadFiles()` (line 49), `folderId` is read from state and forwarded to the backend. If it could be set to `null`, global search would work immediately.

3. **StatusBar shows "Kein Ordner ausgewahlt"** -- when `selectedFolderId` is `null`, the status bar shows a negative message rather than indicating global scope.

4. **No visual feedback** -- users have no way to tell whether they are searching within a folder or globally.

## Proposed Approach

### Step 1: Add "Alle Ordner" entry to Sidebar

In `src/components/Sidebar.ts`, add an "Alle Ordner" (All Folders) list item at the top of the folder list. Clicking it sets `appState.set("selectedFolderId", null)`. It should show the total file count across all folders and be visually highlighted when `selectedFolderId === null`.

- Add a sum of all folder counts for the badge number
- Style it like other folder items but with a distinguishing icon or style (e.g., slightly different background or a globe/library icon)
- It should be selected by default on app start (since `initialState.selectedFolderId` is already `null`)

### Step 2: Allow deselecting a folder by re-clicking

In `Sidebar.ts`, modify the click handler: if the clicked folder is already selected, deselect it by setting `selectedFolderId` to `null`. This provides an alternative way to return to global view.

### Step 3: Update StatusBar for global search context

In `src/components/StatusBar.ts`, change the left status text:
- When `selectedFolderId === null`: display "Alle Ordner" instead of "Kein Ordner ausgewahlt"
- When a folder is selected: continue showing the folder name as before

### Step 4: Update StatusBar to show search scope indicator

When a search query is active and `selectedFolderId === null`, the status bar should indicate that the search spans all folders. For example: "Alle Ordner -- Suche: <query>".

### Step 5: Verify end-to-end flow

No changes are needed in:
- `FileList.ts` -- already reads `selectedFolderId` from state and passes it (including `null`) to `FileService.getFiles()`
- `FileService.ts` -- already accepts `null` for `folderId`
- `main.ts` -- `reloadFiles()` already reads `selectedFolderId` and passes it through
- Backend `commands/files.rs` -- `query_files_impl()` already skips the folder filter when `folder_id` is `None`

The entire pipeline from frontend state through Tauri invoke to SQL query already supports `folder_id = null`. The only missing piece is the UI to set it to `null`.

### Step 6: Reset selection on folder change

When switching from a specific folder to "Alle Ordner" (or vice versa), clear `selectedFileId` and `selectedFileIds` to avoid referencing files that may not be in the new result set.

### Summary of file changes

| File | Change |
|------|--------|
| `src/components/Sidebar.ts` | Add "Alle Ordner" item at top of list; allow deselect by re-click |
| `src/components/StatusBar.ts` | Show "Alle Ordner" instead of "Kein Ordner ausgewahlt" when no folder selected |

### No changes needed

| File | Reason |
|------|--------|
| `src/components/FileList.ts` | Already passes `null` folderId when `selectedFolderId` is `null` |
| `src/components/SearchBar.ts` | Search input is folder-agnostic; no changes needed |
| `src/components/FilterChips.ts` | Format filter is folder-agnostic; no changes needed |
| `src/services/FileService.ts` | Already accepts `null` folderId |
| `src/main.ts` | `reloadFiles()` already handles `null` folderId |
| `src-tauri/src/commands/files.rs` | Backend already omits folder filter when `folder_id` is `None` |
| `src/types/index.ts` | `selectedFolderId` is already `number \| null` |
| `src/state/AppState.ts` | Initial state already has `selectedFolderId: null` |
