# Code Review — Delete Menu Items — Round 1
Reviewer: Codex (Sonnet)
Date: 2026-03-13

## Findings

### Finding 1 — `deleteSelectedFiles`: partial deletion on error, state left inconsistent

**File:** `src/main.ts`, lines 189–200

**Problem:** The delete loop (`for (const id of fileIds)`) iterates sequentially and the `try/catch` wraps the entire loop. If `invoke("delete_file", ...)` throws on file N (not the first), then files 0..N-1 have already been deleted from the backend while `selectedFileIds` and `selectedFileId` have not been cleared yet, and `reloadFiles()` is never called. The UI state (selection) still references IDs that no longer exist in the database, and the file list shown to the user is stale.

**Expected behavior:** Either (a) clear state and reload even on partial failure, or (b) collect per-file errors and report which files failed while still reloading and clearing the succeeded deletions from state.

**Minimal fix example:**
```typescript
const failed: number[] = [];
for (const id of fileIds) {
  try {
    await invoke("delete_file", { fileId: id });
  } catch {
    failed.push(id);
  }
}
// Always clear selection and reload regardless of partial failure
appState.set("selectedFileIds", []);
appState.set("selectedFileId", null);
await reloadFiles();
if (failed.length > 0) {
  ToastContainer.show("error", `${failed.length} Datei(en) konnten nicht geloescht werden`);
} else {
  ToastContainer.show("success", fileIds.length === 1 ? "Datei geloescht" : `${fileIds.length} Dateien geloescht`);
}
```

---

### Finding 2 — `deleteSelectedFiles`: file lookup failure silently aborts for single-file case

**File:** `src/main.ts`, line 183

**Problem:** When exactly one file ID is selected, `files.find((f) => f.id === fileIds[0])` is used solely to obtain the display name for the confirmation dialog. If the file is not found in the local `files` array (e.g. because state is stale), the function returns early with no user feedback. The deletion is silently abandoned even though the file ID is valid in the database.

**Expected behavior:** If the file name cannot be resolved for the confirmation prompt, either use a fallback label (`ID: ${fileIds[0]}`) or still proceed to show a generic confirmation rather than silently doing nothing.

---

### Finding 3 — `updateItemStates`: `hasAny` logic conflates single-file and multi-select

**File:** `src/components/Toolbar.ts`, line 285

**Problem:**
```typescript
const hasAny = hasFile || hasMulti;
```
`hasMulti` is `multiCount > 1`. However `selectedFileIds` may contain exactly one entry (`multiCount === 1`) which is not covered by `hasFile` (which comes from `selectedFileId`) nor by `hasMulti`. The "Datei loeschen" button will be disabled (`!hasAny === true`) in the state where `selectedFileIds` has exactly one item and `selectedFileId` is `null`.

Whether this edge case actually occurs depends on how `FileList` sets state (if it always sets both), but the logic is fragile. A safer expression is:
```typescript
const hasAny = hasFile || multiCount >= 1;
```

---

### Finding 4 — `toolbar:delete-folder` handler: `reloadFiles()` called after folder is deselected, but selection state order matters

**File:** `src/main.ts`, lines 506–509

**Problem:** State is cleared in this order:
```typescript
appState.set("selectedFolderId", null);
appState.set("selectedFileId", null);
appState.set("selectedFileIds", []);
await reloadFiles();
```
`reloadFiles()` calls `FileService.getFiles(folderId, ...)` using `appState.get("selectedFolderId")` which is now `null`. This is correct intent (show no files), but subscribers to `selectedFolderId` fire before `selectedFileId` and `selectedFileIds` are cleared. If any subscriber to `selectedFolderId` triggers a render that reads `selectedFileId` (e.g. MetadataPanel), it may briefly render with a stale file selected against a null folder. This is a minor UI flicker risk rather than a data integrity issue, but worth noting.

**Suggested fix:** Clear all three state keys before any of the `set` calls triggers re-renders, or clear them in a single batch if `appState` supports it.

---

### Finding 5 — Missing `toolbar:delete-file` shortcut label vs actual shortcut

**File:** `src/components/Toolbar.ts`, line 119

**Problem:** The "Datei loeschen" menu item declares `shortcut: "Del"`. The shortcut registration (`shortcut:delete`) is wired in `src/shortcuts.ts` (not reviewed here), but the menu item emits `toolbar:delete-file`, which is separately handled in `main.ts` line 490. If the keyboard shortcut uses a different event name (`shortcut:delete`), the `"Del"` label shown in the menu is accurate only if `shortcut:delete` maps to the same action as `toolbar:delete-file`. This is correct as written (both call `deleteSelectedFiles()`), so no functional bug, but the coupling is indirect and not obvious — worth a clarifying comment.

This is a low-severity observation, not a blocking finding.
