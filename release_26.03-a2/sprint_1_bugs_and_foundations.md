# Sprint 1 — Bugs & Foundations

**Focus:** Fix critical bug, clean dead code, improve error UX and accessibility
**Issues:** #38, #23, #42, #41

---

## Issue #38 — Bug: Canvas event listener leak in MetadataPanel

**Type:** Bug (Critical)
**Effort:** S

### Problem
Stitch preview canvas in MetadataPanel registers `wheel`, `mousedown`, `dblclick` listeners that are never unregistered. Document-level `mousemove`/`mouseup` for panning are only cleaned on next render, not on destroy. Each file selection adds orphaned listeners.

### Affected Files
- `src/components/MetadataPanel.ts` — canvas event registration (~lines 862–896)

### Implementation Plan
1. Create a `cleanupCanvasListeners()` method that removes all canvas event listeners
2. Store all canvas event listener references (wheel, mousedown, dblclick) for later removal
3. Call `cleanupCanvasListeners()` before re-rendering the preview section
4. Call `cleanupCanvasListeners()` in `destroy()`
5. Ensure document-level `mousemove`/`mouseup` are also cleaned in the same method

### Verification
- Select file A → Select file B → Select file C → Verify only 1 set of listeners exists
- Check Chrome DevTools → Elements → Event Listeners on canvas element

---

## Issue #23 — Dead code cleanup

**Type:** Cleanup
**Effort:** S

### Problem
Three instances of dead code identified during release testing:
1. Unused AI event bridges in `src/main.ts:87-89`
2. Unused `ImportProgress` type in `src/types/index.ts:133`
3. Unused `ThumbnailGenerator::invalidate()` in `src-tauri/src/services/thumbnail.rs:105`

### Affected Files
- `src/main.ts` — lines 87-89
- `src/types/index.ts` — line 133
- `src-tauri/src/services/thumbnail.rs` — line 105

### Implementation Plan

#### Option A: Remove dead code
1. Remove the 3 `listen("ai:*")` bridge lines from `initTauriBridge()` in `main.ts`
2. Remove the `ImportProgress` interface from `types/index.ts`
3. Remove the `invalidate()` method from `thumbnail.rs`

#### Option B: Repurpose (preferred where useful)
1. **AI event bridges:** Repurpose to show toast notifications for AI analysis status (start/complete/error) — gives user feedback during long-running AI operations. If repurposed, wire them to `Toast.show()`.
2. **ImportProgress:** Import and use in `BatchDialog.ts` for type-safe event payload handling.
3. **ThumbnailGenerator::invalidate():** Wire into `delete_file` and `watcher_remove_by_paths` commands to clean up stale thumbnails.

**Decision:** Evaluate each on merit. If repurposing is trivial, do it. Otherwise remove.

### Verification
- `cargo check` — no compile warnings for unused code
- `npm run build` — no TS errors
- Grep for removed identifiers to confirm no remaining references

---

## Issue #42 — Improve error visibility

**Type:** Enhancement
**Effort:** M

### Problem
Multiple operations fail silently across frontend and backend, leaving users unaware of issues.

### Affected Files
- `src/components/SearchBar.ts` — `loadTags()` silent catch
- `src/components/Sidebar.ts` — `loadFolders()` silent catch
- `src/components/FileList.ts` — `loadFiles()` silent catch
- `src/components/MetadataPanel.ts` — `onSelectionChanged()` silent catch
- `src/components/StatusBar.ts` — needs watcher status indicator
- `src-tauri/src/lib.rs` — watcher startup failure (warn! only)
- `src-tauri/src/services/thumbnail.rs` — generation failures logged but not surfaced

### Implementation Plan

#### Frontend error toasts (Steps 1-4)
1. **SearchBar.loadTags():** Show `Toast.show("Tags konnten nicht geladen werden", "error")` on catch
2. **Sidebar.loadFolders():** Show `Toast.show("Ordner konnten nicht geladen werden", "error")` on catch
3. **FileList.loadFiles():** Show `Toast.show("Dateien konnten nicht geladen werden", "error")` on catch, plus empty-state message in file list area
4. **MetadataPanel.onSelectionChanged():** Show `Toast.show("Metadaten konnten nicht geladen werden", "error")` on catch

#### Watcher status (Step 5)
5. Emit a Tauri event `watcher:status` from `lib.rs` when watcher fails to start. Frontend StatusBar listens and shows an indicator icon (e.g., "Automatischer Import deaktiviert")

#### Thumbnail fallback (Step 6)
6. When thumbnail generation fails, store a flag or use a fallback placeholder icon in `FileList` cards instead of blank space

#### Mutex error handling (Step 7)
7. Replace `if let Ok(guard) = mutex.lock()` in `lib.rs` with explicit error logging via `error!()` macro

### Verification
- Simulate network/DB errors and verify toasts appear
- Stop file watcher and verify status bar indicator
- Corrupt a file and verify thumbnail placeholder appears

---

## Issue #41 — Accessibility: ARIA labels & focus management

**Type:** Enhancement
**Effort:** M

### Problem
Missing ARIA labels on icon-only buttons, no focus trap in dialogs, missing semantic attributes.

### Affected Files
- `src/components/Toolbar.ts` — icon buttons need `aria-label`
- `src/components/FilterChips.ts` — filter buttons need `aria-label`
- `src/components/SearchBar.ts` — icon buttons need `aria-label`
- `src/components/Toast.ts` — container needs `aria-live`
- `src/components/BatchDialog.ts` — progress bar needs `role="progressbar"`
- `src/components/SettingsDialog.ts` — needs focus trap
- `src/components/AiPreviewDialog.ts` — needs focus trap
- `src/components/AiResultDialog.ts` — needs focus trap
- `src/components/BatchDialog.ts` — needs focus trap

### Implementation Plan

#### ARIA labels (Steps 1-3)
1. Add `aria-label` attributes to all icon-only buttons in Toolbar, FilterChips, SearchBar (German text)
2. Add `aria-live="polite"` to the Toast container element
3. Add `role="progressbar"`, `aria-valuenow`, `aria-valuemax` to BatchDialog progress bar

#### Focus trap (Steps 4-5)
4. Create a utility function `trapFocus(dialogEl: HTMLElement)` that:
   - Finds all focusable elements within the dialog
   - On Tab: moves focus to next focusable element, wrapping to first at end
   - On Shift+Tab: moves focus to previous, wrapping to last at start
   - Returns a cleanup function to remove the keydown listener
5. Apply `trapFocus()` in all four dialog components on open, cleanup on close

#### Focus return (Step 6)
6. Store the `document.activeElement` before dialog open, restore focus to it on close

#### Semantics (Steps 7-8)
7. Add `alt` attributes to dynamically created image elements (thumbnails, previews)
8. Add `fieldset`/`legend` grouping to SettingsDialog form sections

### Verification
- Tab through the app with keyboard only — verify all controls are reachable
- Open each dialog — verify Tab stays within dialog
- Close dialog — verify focus returns to trigger element
- Test with screen reader (or browser a11y inspector) — verify labels are announced
