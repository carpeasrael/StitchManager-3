# Issue #44 — Codex Code Review: Folder Deletion (Round 2)

## Review Scope

Reviewed the folder deletion implementation across:
- `src/components/Sidebar.ts` — inline delete button, `deleteFolder()` method
- `src/components/Toolbar.ts` — burger menu "Ordner loeschen" item, `toolbar:delete-folder` event
- `src/main.ts` — `toolbar:delete-folder` event handler (lines 271-309)
- `src/styles/components.css` — `.folder-delete-btn` styles
- `src/services/FolderService.ts` — `remove()`, `getFileCount()`, `getAllFileCounts()`
- `src/types/index.ts` — `Folder.parentId` field

## Previous Findings Verification

### 1. Duplicate deletion logic — FIXED
Sidebar.deleteFolder() (lines 152-158) no longer performs deletion itself. It sets the selectedFolderId and emits `toolbar:delete-folder`. The single handler in main.ts (lines 271-309) performs the actual deletion. No duplication exists.

### 2. Stale files after sidebar deletion — FIXED
The centralized handler in main.ts clears files via `appState.set("files", [])` (line 303) after successful deletion, and also clears selectedFileId and selectedFileIds (lines 298-300). No stale file state remains.

### 3. Subfolder cascade warning — FIXED
The handler checks `folders.some((f) => f.parentId === folderId)` (line 288) and appends " und Unterordner" to the confirmation message when subfolders exist (line 291). The Folder type includes `parentId: number | null`.

### 4. Accessibility — FIXED
CSS uses `opacity: 0; pointer-events: none` (lines 93-94 in components.css) instead of `display: none`, preserving the element in the accessibility tree. `focus-visible` is present (lines 109, 115) to show the button on keyboard focus and apply error color on hover/focus. The delete button also has an `aria-label` with the folder name (Sidebar.ts line 123).

### 5. Stale folder counts — FIXED
Sidebar constructor subscribes to `appState.on("folders", () => this.loadCounts())` (line 13). After folder deletion, the main.ts handler calls `FolderService.getAll()` and sets `appState.set("folders", updatedFolders)` (lines 301-302), which triggers the Sidebar subscription to reload counts.

## New Issues Check

No new issues found. The implementation is clean:
- Event flow is correct: Sidebar emits event, main.ts handles deletion logic
- Error handling is present with try/catch and toast notifications
- State cleanup is thorough (files, selectedFileId, selectedFileIds, selectedFolderId all cleared)
- Toolbar correctly disables the delete-folder menu item when no folder is selected (line 296)
- stopPropagation on the delete button click prevents folder selection conflict (Sidebar line 125)
- File count is fetched and shown in confirmation message (lines 281-285, 292)
- Confirmation message correctly differentiates between folders with/without subfolders and files

## Result

Code review passed. No findings.
