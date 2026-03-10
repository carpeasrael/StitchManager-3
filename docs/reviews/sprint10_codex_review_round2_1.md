# Sprint 10 - Code Review Round 2 - Reviewer 1 (Code Quality)

## Findings

### 1. Escape shortcut bypasses SettingsDialog revert logic

**File:** `src/main.ts`, line ~269 (shortcut:escape handler)

**Description:** When the user presses Escape while the SettingsDialog is open, the `shortcut:escape` handler directly removes the `.dialog-overlay` DOM element. However, the `SettingsDialog` instance's `close()` method is never called, so the live-preview theme and font-size changes are **not reverted**. The `close(saved = false)` method is specifically designed to revert `data-theme` and `--font-size-body` to their original values, but this code path skips it entirely.

**Impact:** If a user changes the theme to "Dunkel" in the appearance tab, then presses Escape instead of clicking "Abbrechen", the dark theme persists even though the setting was never saved. On next app launch it reverts to the saved value, creating an inconsistent experience.

**Suggested fix:** Instead of removing the overlay directly in the Escape handler, emit a `dialog:close` event that the SettingsDialog listens for and calls its own `close()` method. Alternatively, have SettingsDialog register its own Escape key listener that calls `this.close()`.

---

### 2. Virtual scroll `renderVisible()` clears innerHTML on every selection change

**File:** `src/components/FileList.ts`, `renderVisible()` method

**Description:** The `renderVisible()` method sets `this.listEl.innerHTML = ""` and then re-creates all visible card elements from scratch. This is called on every `selectedFileId` and `selectedFileIds` state change (lines ~30-33). For a simple selection toggle, the entire visible DOM is torn down and rebuilt, which is unnecessarily expensive and causes visual flickering (loss of scroll position is avoided since the scroll container is separate, but re-rendering ~20+ cards on every click is wasteful).

**Impact:** Performance degradation on rapid selection changes (e.g., holding arrow keys to navigate). Not a bug per se, but a significant inefficiency in what is supposed to be a virtual scrolling optimization.

**Suggested fix:** On selection-only changes, update just the `selected` class on existing card elements rather than rebuilding the entire visible set. Alternatively, diff against existing children by file ID.

---

### 3. Scroll event handler not throttled/debounced

**File:** `src/components/FileList.ts`, `render()` method

**Description:** The scroll event listener (`this.scrollContainer.addEventListener("scroll", () => this.onScroll())`) fires on every scroll event (potentially 60+ times per second during a smooth scroll). While `onScroll` does check whether the visible range changed before calling `renderVisible`, the `calculateVisibleRange` computation runs on every single scroll event without any throttling via `requestAnimationFrame`.

**Impact:** On large file lists, this may cause jank during scrolling as `calculateVisibleRange` and the comparison logic run at high frequency.

**Suggested fix:** Wrap `onScroll` in `requestAnimationFrame` to batch updates to once per frame.

---

### 4. Missing `Cargo.toml` diff - `notify` and `dirs` dependencies

**File:** `src-tauri/Cargo.toml`

**Description:** The file_watcher.rs uses the `notify` and `dirs` crates, which are present in Cargo.toml (verified: `notify = "6"` and `dirs = "5"`). This is not a finding -- dependencies are correctly declared. No issue here.

*(Retracted -- included during analysis but confirmed correct.)*

---

### 5. `Cmd+S` shortcut fires even when no file is selected

**File:** `src/main.ts`, line ~226 (shortcut:save handler) and `src/components/MetadataPanel.ts`

**Description:** The `shortcut:save` handler emits `metadata:save`, which calls `MetadataPanel.save()`. Looking at MetadataPanel's `save()` method, it does check for a selected file and returns early if none is selected. So this is handled correctly. No issue.

*(Retracted -- false alarm.)*

---

### Actual finding count: 3

Findings 1, 2, and 3 are real issues. Findings 4 and 5 were investigated and retracted.

**Summary:**
1. **Escape key bypasses SettingsDialog revert** -- theme/font changes not reverted when Escape is pressed
2. **renderVisible rebuilds all cards on selection change** -- inefficient DOM thrashing in virtual scroll
3. **Scroll handler not throttled** -- potential jank on rapid scrolling
