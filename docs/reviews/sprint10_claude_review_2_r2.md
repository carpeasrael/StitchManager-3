# Sprint 10 — Claude Review 2 (Round 2): Issue Verification

Reviewer: Claude Opus 4.6
Date: 2026-03-10

## Ticket Verification

### S10-T1: SettingsDialog — PASS

- 5 tabs present: Allgemein, Erscheinungsbild, KI-Einstellungen, Dateiverwaltung, Benutzerdefiniert (lines 68-74 of SettingsDialog.ts).
- Live preview: theme change fires `appState.set("theme", ...)` and sets `data-theme` attribute on `change` event (line 226-229). Font size applied live via `applyFontSize()` on `change` (line 249-251).
- Cancel reverts: `close(saved=false)` restores `originalTheme` and `originalFontSize` (lines 643-648).
- Double-open guard: `static isOpen()` check at top of `open()` (line 15).
- Watcher restart on library_root change: after save, invokes `watcher_stop` then `watcher_start` with new path (lines 148-154).

### S10-T2: Keyboard Shortcuts — PASS

- Input guards: `isInputFocused()` checks for input/textarea/select (lines 3-8 of shortcuts.ts).
- Cmd+S works in inputs: handled before the `isInputFocused()` guard (lines 21-25).
- All shortcuts mapped: Cmd+S (save), Cmd+F (search), Cmd+, (settings), Delete/Backspace (delete), ArrowUp/Down (navigate), Escape (close/clear) — all with correct `preventDefault()`.
- Escape does not call `preventDefault()` (no unnecessary browser interference) — correct.
- All shortcuts wired to EventBus handlers in main.ts (lines 260-316).

### S10-T3: Filesystem Watcher — PASS

- `notify` v6 crate used with `RecommendedWatcher` (file_watcher.rs line 1, Cargo.toml line 27).
- Create/Modify/Remove event handling with pattern matching (lines 64-72).
- 500ms debounce with `recv_timeout` and `Instant` tracking (lines 57, 104).
- HashSet dedup for accumulated file paths (lines 52-53).
- Embroidery file filter via `is_embroidery_file()` (line 60).
- Auto-start from DB library_root in lib.rs setup (lines 31-72).
- `watcher_auto_import` and `watcher_remove_by_paths` commands in scanner.rs handle DB operations.
- Frontend bridges Tauri events to EventBus, shows toasts, calls `reloadFiles()` (main.ts lines 233-257).
- `watcher_start`/`watcher_stop` commands registered (lib.rs lines 121-122).

### S10-T4: Splitter Handles — PASS

- Two splitters in index.html: `.app-splitter-l` and `.app-splitter-r` (lines 14, 16).
- Splitter class creates draggable div, tracks mousedown/mousemove/mouseup (Splitter.ts).
- CSS custom properties: `--sidebar-width` (min 180, max 400, default 240) and `--center-width` (min 300, max 800, default 480) — enforced via `Math.min`/`Math.max` (line 48-50).
- Grid template uses the CSS variables: `var(--sidebar-width, 240px) 4px var(--center-width, 480px) 4px 1fr` (layout.css line 4).
- Proper cleanup: `removeEventListener` for mousemove/mouseup on mouseup (lines 62-63).

### S10-T5: Virtual Scrolling — PASS

- Fixed card height 72px, buffer of 5 (FileList.ts lines 6-7).
- Spacer div with `height = files.length * CARD_HEIGHT` for scrollbar accuracy (line 80).
- `calculateVisibleRange()` computes start/end from scrollTop (lines 106-117).
- Cards positioned absolutely with `top = i * CARD_HEIGHT` (line 138).
- rAF-throttled scroll handler via `scrollRafPending` flag (lines 91-104).
- Selection-only updates via `updateSelection()` method that iterates existing DOM children without re-rendering (lines 202-218).

### S10-T6: Toast Notifications — PASS

- Max 5 concurrent: `if (current.length >= 5)` slices to keep last 4 before adding new one (lines 61-63).
- 4-second auto-dismiss via `setTimeout` (line 66).
- Slide-in animation: `toast-slide-in 0.3s` (components.css line 1188, keyframes lines 1231-1240).
- Fade-out exit animation: `toast-exit` class with `toast-fade-out 0.3s` (components.css lines 1192-1194, keyframes 1242-1250).
- Exit animation race prevention: check `!el.classList.contains("toast-exit")` before adding exit class (line 22), and skip adding new elements for IDs still animating out (line 31).
- Three levels: success, error, info with distinct colors (components.css lines 1207-1229).

### S10-T7: Bundle Config — PASS

- `productName`: "StichMan" (tauri.conf.json line 3).
- `version`: "2.0.0" (line 4).
- `identifier`: "de.carpeasrael.stichman" (line 5).
- Icon paths configured: 32x32.png, 128x128.png, 128x128@2x.png, icon.icns, icon.ico (lines 32-37).
- Bundle active with targets "all" (lines 30-31).

### S10-T8: QA — PASS (deferred to caller)

- Cannot run `cargo check`, `cargo test`, or `npm run build` as Bash is restricted in this session.
- The analysis document confirms QA passed in the implementation phase: "cargo check clean, 99/99 tests pass, npm run build clean."
- All commands are registered in lib.rs invoke_handler, modules declared in services/mod.rs, dependencies present in Cargo.toml.

## Additional Verifications

### Font size persists on restart — FAIL

**Finding:** In `src/main.ts` line 66, `applyFontSize()` sets the CSS custom property `--font-size-base`, but this variable is never referenced anywhere in the CSS. The actual CSS variable used throughout all components and layout is `--font-size-body` (defined in `aurora.css` line 26 and referenced in `components.css` at ~20 locations and `layout.css` line 15).

The `SettingsDialog.applyFontSize()` correctly sets `--font-size-body` (SettingsDialog.ts line 266), so changing font size within the dialog works. But on application restart, `main.ts` reads the persisted font_size from DB and sets the wrong CSS variable (`--font-size-base` instead of `--font-size-body`), meaning the persisted font size has no visual effect after restart.

**Fix required:** Change `main.ts` line 66 from `"--font-size-base"` to `"--font-size-body"`.

### reloadFiles() respects filters — PASS

- `reloadFiles()` in main.ts (lines 319-325) reads `selectedFolderId`, `searchQuery`, and `formatFilter` from state and passes all three to `FileService.getFiles()`.

## Summary

| Ticket | Status |
|--------|--------|
| S10-T1 | PASS |
| S10-T2 | PASS |
| S10-T3 | PASS |
| S10-T4 | PASS |
| S10-T5 | PASS |
| S10-T6 | PASS |
| S10-T7 | PASS |
| S10-T8 | PASS (deferred) |
| Font size persists on restart | FAIL |
| reloadFiles() respects filters | PASS |

**1 FINDING:**

1. `src/main.ts` line 66: `applyFontSize()` sets `--font-size-base` but should set `--font-size-body` to match the CSS variable actually used throughout the application. This breaks font size persistence on restart.
