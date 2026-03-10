# Sprint 10 Codex Review 1

## Findings

### 1. BUG: `metadata:save` event emitted but never consumed

**File:** `src/main.ts` (line 234), `src/components/MetadataPanel.ts`

The keyboard shortcut Cmd+S triggers `shortcut:save`, which in `main.ts` re-emits as `metadata:save`. However, `MetadataPanel` never subscribes to the `metadata:save` event, so pressing Cmd+S does nothing. The MetadataPanel has a `save()` method but it is only wired to the save button click.

**Fix:** In `MetadataPanel`'s constructor, subscribe to `EventBus.on("metadata:save", () => this.save())`.

---

### 2. BUG: `toolbar:save` event emitted but never consumed

**File:** `src/components/Toolbar.ts` (line 44), `src/main.ts`

The toolbar "Speichern" button emits `toolbar:save`, but nothing in `main.ts` or any other component listens for this event. The save button in the toolbar is therefore non-functional.

**Fix:** Add a handler in `initEventHandlers()` in `main.ts`:
```ts
EventBus.on("toolbar:save", () => {
  EventBus.emit("metadata:save");
});
```

---

### 3. BUG: Right splitter dragging goes in wrong direction

**File:** `src/components/Splitter.ts`

The Splitter always computes `delta = ev.clientX - this.startX` and adds it to the start value. For the right splitter (`--center-width`), dragging rightward increases the center panel width, which is correct. However, this only works because the right splitter controls `--center-width`, not `--right-width`. This is actually fine as implemented. No issue here on closer inspection.

---

### 3. BUG (revised): `.file-list` CSS has `gap` but virtual scrolling uses absolute positioning

**File:** `src/styles/components.css` (line 184), `src/components/FileList.ts`

The `.file-list` CSS rule sets `gap: var(--spacing-2)` via flexbox column layout, but the virtual scrolling implementation positions cards absolutely within a spacer div. The `gap` property has no effect on absolutely-positioned children, so this is merely dead CSS -- not visually broken. However, the `.file-list` also lacks a `height: 100%` rule, which means the scroll container may not fill its parent and virtual scrolling may not work correctly.

**Fix:** Add `height: 100%` to the `.file-list` CSS rule, or set it on the `.app-center` inner wrapper. The parent `.app-center` has `overflow-y: auto`, which may conflict with the scroll container needing to be the scroll target. Consider setting `.app-center { overflow: hidden; }` and `.file-list { height: 100%; overflow-y: auto; }`.

---

### 4. BUG: `file-list` gap style interferes with virtual scroll spacer height calculation

**File:** `src/components/FileList.ts` (line 78)

The spacer height is set to `files.length * CARD_HEIGHT`, where `CARD_HEIGHT = 72`. The `.file-card` CSS also sets `height: 72px`. However, the absolute-positioned cards have `top: i * CARD_HEIGHT`, which leaves no gap between them. The flexbox `gap` in `.file-list` does not apply. The visual result is cards with no spacing, which works but deviates from the non-virtual design (which uses gap). This is cosmetic, not a bug.

---

### 5. LOGIC: `ThemeMode` values mismatch risk with `applyFontSize`

**File:** `src/components/SettingsDialog.ts` (line 194)

When the theme select changes, the value is cast `as ThemeMode` directly from the select value. The select options use values `"hell"` and `"dunkel"`, which match `ThemeMode`. This is correct. No issue.

---

### 6. BUG: `unwrap()` on Mutex lock in `lib.rs` setup can panic

**File:** `src-tauri/src/lib.rs` (line 63)

```rust
*watcher_holder.0.lock().unwrap() = Some(state);
```

If the Mutex is poisoned (e.g., from a prior panic), this will cause a second panic, crashing the application at startup. All other Mutex usages in the codebase use `.map_err()` to convert lock errors gracefully.

**Fix:** Use `.map_err()` or `.expect()` with a descriptive message instead of bare `.unwrap()`. Even better:
```rust
if let Ok(mut guard) = watcher_holder.0.lock() {
    *guard = Some(state);
}
```

---

### 7. PERFORMANCE: File watcher debounce uses linear search (`Vec::contains`)

**File:** `src-tauri/src/services/file_watcher.rs` (lines 65, 70)

`new_files.contains(&path_str)` and `removed_files.contains(&path_str)` perform O(n) linear scans. For directories with many rapid changes, this could become slow. Using `HashSet` would be O(1).

**Fix:** Replace `Vec<String>` with `HashSet<String>` for `new_files` and `removed_files`, converting to `Vec` only at flush time.

---

### 8. BUG: Escape key handler removes dialog overlay without proper cleanup

**File:** `src/main.ts` (lines 274-280)

The Escape handler does `overlay.remove()` directly on any `.dialog-overlay` element. However, `SettingsDialog` maintains internal state (`this.overlay`) and its `close()` method sets `this.overlay = null`. When Escape removes the overlay via DOM manipulation, the `SettingsDialog` instance still holds a stale reference and its internal state is not cleaned up. This could cause issues if the dialog is reopened.

**Fix:** Instead of raw DOM removal, emit an event that the dialog can respond to, or call the dialog's close method. At minimum, the `SettingsDialog.close()` method should tolerate a missing overlay (it already does via the null check, but the instance itself is not cleaned up since it's created fresh each time via `static open()`). On closer inspection, since `SettingsDialog` creates a new instance each time `open()` is called and does not persist any global reference, this is acceptable. The stale reference in the local instance is harmless because the instance goes out of scope. **Downgraded to no-action.**

---

### 8. (revised) NO-OP: Escape dialog handling is acceptable.

---

### 9. MISSING: No `fs-extra` or similar permission for filesystem watcher events

**File:** `src-tauri/capabilities/default.json`

The capabilities file only grants `core:default`, `sql:default`, and `dialog:default`. The file watcher uses `app_handle.emit()` which is a Tauri core event (not a plugin), so no additional permission is needed. The watcher commands (`watcher_start`, `watcher_stop`) are registered as custom invoke handlers, which are covered by `core:default`. No issue.

---

### 10. CODE QUALITY: Duplicated `formatSize` and `getFormatLabel` utility functions

**Files:** `src/components/FileList.ts` (lines 231-239), `src/components/MetadataPanel.ts` (lines 689-698)

Both `FileList` and `MetadataPanel` contain identical `formatSize()` and `getFormatLabel()` methods. These should be extracted to a shared utility module.

**Fix:** Create a `src/utils/format.ts` file with shared helper functions and import from both components.

---

### 11. BUG: `SettingsDialog` applies font size on open but does not persist if canceled

**File:** `src/components/SettingsDialog.ts` (lines 217-224)

The `buildAppearanceTab` method calls `this.applyFontSize(settings.font_size || "medium")` during dialog construction, which is correct. However, the font-size select's `change` handler (line 217-219) applies the font size immediately via `this.applyFontSize()`. If the user changes the font size but then clicks "Abbrechen" (Cancel), the font size change is visually applied but not persisted. The user sees the changed font size without it being saved. The dialog does not revert the change on cancel.

**Fix:** Store the original font size before opening the dialog and restore it in `close()` unless the save button was clicked. Or defer applying the font size until save.

---

### 12. BUG: Same issue with theme - live preview not reverted on cancel

**File:** `src/components/SettingsDialog.ts` (lines 193-197)

The theme select's `change` handler applies the theme immediately via `document.documentElement.setAttribute`. If the user cancels, the theme change persists visually but is not saved to the database. This is inconsistent -- either all changes should preview live and revert on cancel, or none should.

**Fix:** Same pattern as finding 11. Save original theme and restore on cancel, or do not apply until save.

---

### 13. MINOR: Custom field "date" type offered in backend but not in SettingsDialog frontend

**File:** `src-tauri/src/commands/settings.rs` (line 100), `src/components/SettingsDialog.ts` (lines 446-455)

The backend validates custom field types against `["text", "number", "date", "select"]`, but the frontend SettingsDialog only offers `["text", "number", "select"]` -- missing "date". This means users cannot create date-type custom fields from the UI, even though the backend supports it.

**Fix:** Add a `{ value: "date", label: "Datum" }` option to the type select in `buildCustomTab`.

---

### Summary

| # | Severity | Description |
|---|----------|-------------|
| 1 | BUG | `metadata:save` event not consumed -- Cmd+S does nothing |
| 2 | BUG | `toolbar:save` event not consumed -- toolbar save button does nothing |
| 6 | BUG | `unwrap()` on Mutex lock can panic at startup |
| 7 | PERF | File watcher uses `Vec::contains` for dedup -- should use `HashSet` |
| 10 | QUALITY | Duplicated `formatSize`/`getFormatLabel` in FileList and MetadataPanel |
| 11 | BUG | Font size change applied immediately, not reverted on dialog cancel |
| 12 | BUG | Theme change applied immediately, not reverted on dialog cancel |
| 13 | MINOR | Backend supports "date" custom field type but frontend omits it |

Total findings: **8**
