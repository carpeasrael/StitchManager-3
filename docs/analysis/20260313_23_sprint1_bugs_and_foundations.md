# Analysis: Sprint 1 — Bugs & Foundations

**Date:** 2026-03-13
**Issues:** #38, #23, #42, #41
**Phase:** 1 — Analysis (before implementation)

---

## Issue #38 — Bug: Canvas event listener leak in MetadataPanel

### Problem Description
The stitch preview canvas in `MetadataPanel.loadStitchPreview()` registers event listeners that are never properly cleaned up when the user selects a different file. Each file selection adds a new set of orphaned listeners, causing a memory leak.

### Affected Components
- `src/components/MetadataPanel.ts` — `loadStitchPreview()` (lines 754–928)

### Root Cause
Three categories of leaked listeners:

1. **Canvas-level listeners (never removed):**
   - `wheel` on canvas (line 862) — never stored for removal
   - `mousedown` on canvas (line 867) — anonymous function, never stored
   - `dblclick` on canvas (line 896) — anonymous function, never stored
   - `click` on `zoomInBtn` (line 904), `zoomOutBtn` (line 913), `zoomResetBtn` (line 922) — never stored

2. **Document-level listeners (partially cleaned):**
   - `mousemove` and `mouseup` (lines 885–886) are stored in `this.previewCleanup` and cleaned on next render or destroy. This part works correctly.

3. **Accumulation pattern:**
   - `renderFileInfo()` calls `this.previewCleanup()` (line 160) and sets `innerHTML = ""` (line 161), which removes old canvas elements.
   - However, `loadStitchPreview()` is async — it starts after `renderFileInfo()` completes. By the time it attaches listeners to the new canvas, the old canvas is already gone from the DOM. The real issue is that the zoom button listeners are attached to buttons that are children of `wrapper` (which is still in the DOM), and the new canvas listeners accumulate if `loadStitchPreview` is called multiple times before the DOM is cleared.
   - The actual leak vector: `previewCleanup` only stores document-level listeners. Canvas `wheel`, `mousedown`, `dblclick` and button `click` listeners are never cleaned. Since `innerHTML = ""` removes the DOM elements, the canvas listeners ARE garbage-collected when the elements are removed. **But** the zoom button listeners on `zoomInBtn`, `zoomOutBtn`, `zoomResetBtn` are NOT leaked either since those elements are children of the cleared container.

   **Revised finding:** The actual leak is limited to: if `loadStitchPreview()` is called and completes while a previous `loadStitchPreview()` is still running (race condition), both sets of document-level listeners would be active, but only the second set is stored in `previewCleanup`. This is a minor leak. The main issue raised in #38 (canvas listeners) is mitigated by DOM element removal via `innerHTML = ""`, which garbage-collects listeners on removed elements. However, the code is fragile and should be hardened.

### Proposed Approach

1. Expand `previewCleanup` to store ALL listener teardown (canvas + document + buttons) as a single cleanup function
2. Add an `AbortController` to `loadStitchPreview()` to cancel stale async operations when a new file is selected
3. Store and invoke cleanup in `destroy()` and before every new `loadStitchPreview()` call
4. Use a generation counter inside `loadStitchPreview()` to guard against concurrent calls

---

## Issue #23 — Dead code cleanup

### Problem Description
Three instances of dead code identified during release testing v26.03-a2.

### Affected Components
- `src/main.ts` — lines 87–89 (AI event bridges)
- `src/types/index.ts` — lines 133–140 (`ImportProgress` interface)
- `src-tauri/src/services/thumbnail.rs` — lines 105–111 (`invalidate()` method)

### Root Cause / Rationale

1. **AI event bridges (`main.ts:87-89`):** The backend emits `ai:start`, `ai:complete`, `ai:error` from `ai_analyze_file`, but the frontend handles AI results via promise-based `invoke()` returns in `AiPreviewDialog`. No component subscribes to these EventBus events. These bridges serve no purpose.

2. **`ImportProgress` interface (`types/index.ts:133-140`):** This interface is defined but never imported anywhere. The `BatchDialog` receives the same payload shape via inline casting at lines 130-137 of `BatchDialog.ts`:
   ```typescript
   const p = payload as { current: number; total: number; filename: string; ... };
   ```
   The type exists but is unused.

3. **`ThumbnailGenerator::invalidate()` (`thumbnail.rs:105-111`):** This method exists and has tests (line 353–371), but is never called from any command. When a file is deleted (`delete_file` in `files.rs`), the thumbnail on disk is not cleaned up. Similarly, `watcher_remove_by_paths` doesn't invalidate thumbnails.

### Proposed Approach

**Decision: Mix of remove and repurpose based on utility:**

1. **AI event bridges → Remove.** No component uses them. If status feedback is desired in the future, it should be designed intentionally (covered by #42).

2. **`ImportProgress` → Repurpose.** Import and use it in `BatchDialog.ts` for type-safe payload handling instead of inline `as` casting. This improves type safety with zero additional code.

3. **`ThumbnailGenerator::invalidate()` → Repurpose.** Wire it into `delete_file` and `watcher_remove_by_paths` to properly clean up stale thumbnails. The method already exists and is tested.

---

## Issue #42 — Improve error visibility

### Problem Description
Multiple operations fail silently, leaving users unaware of issues. Errors are caught and logged to console only.

### Affected Components

**Frontend silent failures:**
- `src/components/SearchBar.ts:131-134` — `togglePanel()` catches error loading tags, falls back to empty array
- `src/components/Sidebar.ts:25-27` — `loadFolders()` catches error, no user feedback
- `src/components/FileList.ts:58-60` — `loadFiles()` catches error, no user feedback
- `src/components/MetadataPanel.ts:83-86` — `onSelectionChanged()` catches error, shows `renderError()` (this one already has visual feedback via a "Fehler beim Laden" message)

**Backend swallowed errors:**
- `src-tauri/src/lib.rs:72-74` — watcher mutex lock: `if let Ok(mut guard) = watcher_holder.0.lock()` silently ignores poisoned mutex
- `src-tauri/src/lib.rs:77` — watcher startup failure: logged with `warn!` but user not informed
- Thumbnail generation failures — logged but no user-visible fallback indicator (FileList shows format label as fallback, line 189)

### Proposed Approach

1. **SearchBar.loadTags():** Show `Toast.show("error", "Tags konnten nicht geladen werden")` in the catch block (line 133)
2. **Sidebar.loadFolders():** Show `Toast.show("error", "Ordner konnten nicht geladen werden")` in the catch block (line 26)
3. **FileList.loadFiles():** Show `Toast.show("error", "Dateien konnten nicht geladen werden")` in the catch block (line 59)
4. **MetadataPanel.onSelectionChanged():** Already shows visual error state via `renderError()`. No change needed.
5. **Watcher startup failure (lib.rs):** Emit a Tauri event `watcher:status` with `{ active: false, error: "..." }` when the watcher fails to start. Frontend `StatusBar` listens and shows an indicator.
6. **Watcher mutex poisoning (lib.rs:72):** Replace `if let Ok(...)` with proper error logging: `match watcher_holder.0.lock() { Ok(mut guard) => ..., Err(e) => log::error!("...") }`
7. **Thumbnail failures:** The current fallback (format label) is adequate. FileList line 189 already catches and ignores thumbnail errors gracefully. No change needed.

---

## Issue #41 — Accessibility: ARIA labels and focus management

### Problem Description
The application lacks ARIA labels on interactive elements, focus traps in dialogs, and semantic markup.

### Affected Components

**Missing ARIA labels:**
- `src/components/Toolbar.ts` — `createButton()` (line 103): buttons have `title` but no `aria-label`. Since buttons contain both icon and text label (`toolbar-btn-label`), they ARE accessible by text content. However, when screen readers encounter the emoji icon first, it may be confusing. Adding `aria-label` would be clearer.
- `src/components/FilterChips.ts` — filter chips (lines 23-45): buttons have text content ("Alle", "PES", etc.), which IS accessible. No `aria-label` needed.
- `src/components/SearchBar.ts` — clear button (line 62): has `title` but no `aria-label`. Filter toggle (line 72): has `title` but inner icon is a `<span>` with text `⚙`. Should have `aria-label`.
- `src/components/Toast.ts` — container (line 13): no `aria-live` region. Toasts appear dynamically and screen readers won't announce them.
- `src/components/BatchDialog.ts` — progress bar (line 69): no `role="progressbar"` or `aria-valuenow`.

**Missing focus management:**
- `src/components/SettingsDialog.ts` — no focus trap. Tab can navigate behind overlay.
- `src/components/AiPreviewDialog.ts` — no focus trap.
- `src/components/AiResultDialog.ts` — no focus trap.
- `src/components/BatchDialog.ts` — no focus trap.

**Missing semantics:**
- Dynamic `<img>` elements in FileList and AiPreviewDialog have `alt` attributes already set (FileList:166,183 and AiPreviewDialog:86). No change needed.

### Proposed Approach

#### Step 1: ARIA labels
- **Toolbar buttons:** Add `aria-label` attribute in `createButton()` using the `label` parameter
- **SearchBar clear button:** Add `aria-label="Suche leeren"`
- **SearchBar filter toggle:** Add `aria-label="Erweiterte Filter"`
- **Toast container:** Add `aria-live="polite"` and `role="status"` attributes

#### Step 2: Progress bar accessibility
- **BatchDialog:** Add `role="progressbar"`, `aria-valuenow`, `aria-valuemin="0"`, `aria-valuemax="100"` to the progress bar element. Update `aria-valuenow` in `setProgress()`.

#### Step 3: Focus trap utility
- Create a utility function `trapFocus(dialogEl: HTMLElement): () => void`
- Finds all focusable elements (`a, button, input, select, textarea, [tabindex]:not([tabindex="-1"])`)
- Traps Tab/Shift+Tab within the dialog
- Returns cleanup function

#### Step 4: Apply focus trap to dialogs
- **SettingsDialog:** Apply on `show()`, store `activeElement` before open, restore on `close()`
- **AiPreviewDialog:** Apply on `show()`, restore focus on `close()`
- **AiResultDialog:** Apply on `show()`, restore focus on `close()`
- **BatchDialog:** Apply on `show()`, restore focus on `close()`

#### Step 5: Dialog role
- Add `role="dialog"` and `aria-modal="true"` to all dialog elements

---

## Implementation Order

1. **#38** (canvas leak) — smallest, most isolated fix
2. **#23** (dead code) — small cleanup, repurpose thumbnail invalidate
3. **#42** (error visibility) — toast additions + watcher status event
4. **#41** (accessibility) — ARIA labels, focus trap, progress bar semantics

---

## Files to Modify

| File | Issues |
|------|--------|
| `src/components/MetadataPanel.ts` | #38 |
| `src/main.ts` | #23 |
| `src/types/index.ts` | #23 |
| `src-tauri/src/services/thumbnail.rs` | #23 (no change, keep `invalidate()`) |
| `src-tauri/src/commands/files.rs` | #23 (wire `invalidate()` into `delete_file`) |
| `src-tauri/src/commands/scanner.rs` | #23 (wire `invalidate()` into `watcher_remove_by_paths`) |
| `src/components/SearchBar.ts` | #42, #41 |
| `src/components/Sidebar.ts` | #42 |
| `src/components/FileList.ts` | #42 |
| `src-tauri/src/lib.rs` | #42 |
| `src/components/StatusBar.ts` | #42 |
| `src/components/Toolbar.ts` | #41 |
| `src/components/Toast.ts` | #41 |
| `src/components/BatchDialog.ts` | #41 |
| `src/components/SettingsDialog.ts` | #41 |
| `src/components/AiPreviewDialog.ts` | #41 |
| `src/components/AiResultDialog.ts` | #41 |
| New: `src/utils/focus-trap.ts` | #41 |
| `src/components/BatchDialog.ts` | #23 (use `ImportProgress` type) |
