Issue resolved. No findings.

## Verification Details

### Issue #3: MetadataPanel.save() race conditions

**Race 1: Stale currentFile after selection change during async save** -- FIXED

The fix captures `saveFileId = this.currentFile.id` at the top of `save()` (line 546) and uses this captured ID for all async operations (`FileService.updateFile`, `FileService.setTags`, `EventBus.emit("file:saved")`). After each `await` point, the code checks whether `this.currentFile?.id !== saveFileId` and aborts early if the user has selected a different file (lines 587-590, 606-609). This prevents:
- Writing `this.currentFile = updatedFile` for a stale file (the old bug on line 585)
- Updating `appState.files` with wrong data
- Emitting `file:saved` with the wrong file ID

The early `return` still reaches the `finally` block (JS spec guarantees this), so `this.saving` is properly reset and `checkDirty()` is called.

**Race 2: Lost update from concurrent watcher events** -- FIXED

The old code did a non-atomic read-modify-write pattern:
```ts
const files = appState.get("files");  // snapshot
files[idx] = updatedFile;             // modify snapshot
appState.set("files", files);         // write back (may overwrite watcher updates)
```

The fix introduces `appState.update()` (AppState.ts, line 46-48) which performs a synchronous read-modify-write:
```ts
appState.update("files", (files) =>
  files.map((f) => (f.id === updatedFile.id ? updatedFile : f))
);
```

Since JavaScript is single-threaded and there is no `await` inside `update()`, no watcher event can interleave between the read and write. The updater function operates on the current state at call time, not a stale snapshot captured before an async gap. If a watcher's `reloadFiles()` called `appState.set("files", freshFiles)` before this line, the `update` will see the fresh files. If it runs after, the watcher will overwrite with its own fresh data from the database, which is also correct.

### Additional observations (no action required)

- The `finally` block properly resets `this.saving = false` and calls `this.checkDirty()`, which re-enables the save button via `saveBtn.disabled = !this.dirty || this.saving`. This works correctly for both normal completion and early-abort paths.
- The `appState.update` method is cleanly typed with generics matching the existing `get`/`set` pattern.
- The `saveFileId` variable is also used for the `setTags` call (line 603), preventing tags from being written to the wrong file.
