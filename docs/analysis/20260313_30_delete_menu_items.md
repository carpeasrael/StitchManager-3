# Analysis — Delete Entries and Folders via Burger Menu

Date: 2026-03-13

## Problem Description

File deletion is only available via Delete/Backspace keyboard shortcut (single file). Folder deletion has no UI at all. Both should be accessible from the burger menu.

## Affected Components

| File | Change |
|------|--------|
| `src/components/Toolbar.ts` | Add "Datei loeschen" and "Ordner loeschen" menu items |
| `src/main.ts` | Add EventBus handlers for `toolbar:delete-file` and `toolbar:delete-folder` |

## Root Cause / Rationale

Backend commands (`delete_file`, `delete_folder`) and service methods (`FileService.deleteFile`, `FolderService.remove`) already exist and are tested. Only the menu UI entry points are missing.

## Proposed Approach

### Step 1: Add menu items to Toolbar.ts

- **Datei group**: Add "Datei loeschen" (Delete/Backspace shortcut hint), disabled when no file selected, emits `toolbar:delete-file`
- **Ordner group**: Add "Ordner loeschen", disabled when no folder selected, emits `toolbar:delete-folder`

### Step 2: Add event handlers in main.ts

- `toolbar:delete-file`: Reuse existing `shortcut:delete` logic (confirm, delete single or multi, reload)
- `toolbar:delete-folder`: Confirm with folder name and file count warning, call `FolderService.remove()`, reload folders, clear selection
