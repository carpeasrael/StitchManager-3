Issue resolved. No findings.

## Verification Details

### Race 1: Stale currentFile after selection change during async save

**Status: FIXED**

The `save()` method in `MetadataPanel.ts` now:

1. Captures the file ID at the start of the operation (`const saveFileId = this.currentFile.id` at line 546).
2. Uses `saveFileId` (not `this.currentFile`) for all async calls: `FileService.updateFile(saveFileId, updates)` and `FileService.setTags(saveFileId, values.tags)`.
3. Checks for selection changes after each async boundary with `if (this.currentFile?.id !== saveFileId) return;` (lines 587 and 601).
4. Only writes back to `this.currentFile` if the selection has not changed.
5. Emits `file:saved` with the captured `saveFileId`, not `this.currentFile.id`.

This prevents the stale-write scenario described in the issue.

### Race 2: Lost update from concurrent watcher events (appState get/modify/set)

**Status: FIXED**

The save method now uses `appState.update("files", updater)` (line 592-594) instead of separate `get`/`set` calls. The `update` method in `AppState.ts` (line 46-48) performs a synchronous read-modify-write on the internal state reference (`this.state[key]`), which is atomic within a single JavaScript event loop tick. The updater function surgically replaces only the saved file via `.map()`, preserving any changes the watcher may have made to other files in the array.

Additionally, the `AppState.update()` method was added to support this pattern, providing an atomic read-modify-write primitive that prevents the get-then-set interleaving described in the issue.
