# Sprint 14 Analysis — UI State, Feedback & Data Persistence

**Date:** 2026-03-14
**Issues:** #52, #58, #59, #60
**Severity:** 3x high, 1x medium

---

## Issue #52 — Custom fields never saved or loaded

### Problem
Custom field inputs are rendered in MetadataPanel but values are never read on save or populated on load. The `custom_field_values` DB table exists but no Tauri commands exist to access it.

### Approach
1. Add `get_custom_field_values` and `set_custom_field_values` Tauri commands in settings.rs
2. Register commands in lib.rs
3. Add service wrappers in SettingsService.ts
4. In MetadataPanel: load values in onSelectionChanged, populate inputs in renderCustomField, read+save in save()

## Issue #58 — No user feedback on batch partial failures

### Problem
Batch rename/organize/AI handlers catch errors with `console.warn` only. The `BatchResult` with success/failed counts is available but ignored.

### Approach
Capture `BatchResult` return value and show toast with success/failure counts after each batch operation.

## Issue #59 — Dual file-loading race condition

### Problem
`FileList.loadFiles()` and `main.ts:reloadFiles()` both set `appState.files` independently. Only FileList has generation tracking.

### Approach
Remove `reloadFiles()` from main.ts. Instead, after operations that change files, trigger FileList's existing loadFiles via state changes (set selectedFolderId or emit a reload signal). This consolidates to a single file-loading path.

## Issue #60 — Sidebar folder counts stale after scan/batch

### Problem
Sidebar only refreshes counts on `folders` state change. Scan/batch operations update files but not folders.

### Approach
After reloadFiles (or the equivalent trigger), also refresh folder counts. Add a helper that updates both files and folder counts.
