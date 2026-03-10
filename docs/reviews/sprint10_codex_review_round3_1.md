# Sprint 10 Codex Review Round 3 - Reviewer 1

## Finding 1: file_watcher.rs debounce flush only triggers after an event is received, not on pure timeout

**File:** `src-tauri/src/services/file_watcher.rs`, lines 56-122

The debounce flush logic at line 104 (`if last_flush.elapsed() >= ...`) is only reached after the `match rx.recv_timeout(...)` arm completes. When a `Timeout` occurs (line 78), execution falls through to the flush check on line 104 -- this is correct. However, when events arrive continuously within the debounce window, the `Ok(Ok(event))` arm on line 58 accumulates events and then also falls through to the flush check. The issue is that `last_flush` is only reset after a flush (line 121), but if events keep arriving faster than the debounce window, the `recv_timeout` will keep returning `Ok(...)` and `last_flush.elapsed()` will eventually exceed `DEBOUNCE_MS`, so events will be flushed. This is actually correct on closer inspection -- no bug here.

**Verdict:** Not a bug. Retracted.

## Finding 2: Splitter does not account for right-side splitter needing inverted delta

**File:** `src/components/Splitter.ts`, lines 47-55

The Splitter always computes `delta = ev.clientX - this.startX` and adds it to the start value. For the left sidebar splitter (`--sidebar-width`), dragging right correctly increases width. However, for the right panel splitter (`--center-width`), the same logic applies -- dragging the splitter right increases `--center-width`. Whether this is correct depends on the CSS layout. If `--center-width` controls the center column width and the splitter is on the right edge of the center column, then dragging right should increase center width, which is correct. This depends on the CSS layout, but the logic is consistent.

**Verdict:** Layout-dependent, not provably a bug without CSS context.

## Finding 3: `batch_export_usb` collision counter has no upper bound

**File:** `src-tauri/src/commands/batch.rs`, lines 355-366

The filename collision loop increments `counter` indefinitely with no upper bound. If somehow a very large number of collisions exist, this could loop for a very long time. However, in practice this is bounded by the number of files on disk and is extremely unlikely to be a real problem.

**Verdict:** Extremely unlikely, not a practical bug.

## Finding 4: `MetadataPanel.save()` mutates the shallow-copied array from `appState.get("files")`

**File:** `src/components/MetadataPanel.ts`, lines 588-593

```typescript
const files = appState.get("files");
const idx = files.findIndex((f) => f.id === updatedFile.id);
if (idx >= 0) {
  files[idx] = updatedFile;
  appState.set("files", files);
}
```

`appState.get("files")` returns a shallow copy of the array (each element is spread via `{ ...item }`). The code replaces `files[idx]` with `updatedFile` and then calls `appState.set("files", files)`. This is correct -- it's modifying the copy and then setting it back. No mutation of internal state occurs before `set()`.

**Verdict:** Not a bug.

## Finding 5: `initEventHandlers` registers EventBus handlers that are never cleaned up

**File:** `src/main.ts`, lines 118-283

All `EventBus.on(...)` calls in `initEventHandlers()` return unsubscribe functions that are never stored or called. Since `init()` is called once at startup and the app runs for the lifetime of the window, these handlers are never expected to be cleaned up. This is intentional for a single-page Tauri app.

**Verdict:** Not a bug in this context.

## Finding 6: `shortcuts.ts` prevents default browser Cmd+S behavior even when it may be useful

**File:** `src/shortcuts.ts`, line 25

`e.preventDefault()` is called for `Cmd+S` even when no file is selected and there's nothing to save. The handler emits `shortcut:save` which triggers `metadata:save` which calls `MetadataPanel.save()` which early-returns if `!this.currentFile || !this.dirty || this.saving`. This is harmless -- the save is a no-op when nothing is dirty, and preventing the browser's save-page dialog in a Tauri app is correct.

**Verdict:** Not a bug.

---

After thorough review of all new and modified files including:
- `src-tauri/src/services/file_watcher.rs`
- `src-tauri/src/lib.rs`
- `src/components/Splitter.ts`
- `src/components/Toast.ts`
- `src/shortcuts.ts`
- `src/utils/format.ts`
- `src/main.ts`
- `src/components/MetadataPanel.ts`
- `src/components/FileList.ts`
- `src/components/Toolbar.ts`
- `src/components/BatchDialog.ts`
- `src/components/SettingsDialog.ts`
- `src/components/StatusBar.ts`
- `src/state/AppState.ts`
- `src/state/EventBus.ts`
- `src/types/index.ts`
- `src-tauri/src/commands/batch.rs`
- `src-tauri/src/commands/settings.rs`

ZERO FINDINGS.

No bugs, logic errors, security issues, race conditions, memory leaks, or type safety issues were found that could cause crashes, incorrect behavior, or data corruption.
