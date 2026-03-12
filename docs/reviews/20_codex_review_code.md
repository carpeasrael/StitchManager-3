# Codex Code Review — Go to Location (Issue #20)

**Reviewer:** Codex-style (code review)
**Date:** 2026-03-12
**Scope:** lib.rs, Cargo.toml, capabilities/default.json, package.json, Toolbar.ts, MetadataPanel.ts, main.ts, shortcuts.ts, components.css

---

## Findings

No findings.

## Summary

The implementation is clean, correct, and consistent with the existing codebase patterns. Detailed assessment:

1. **Plugin wiring (3-point pattern):** `tauri-plugin-opener` is correctly added in `Cargo.toml` (v2.5.3), registered in `lib.rs` via `.plugin(tauri_plugin_opener::init())`, and granted via `"opener:default"` in `capabilities/default.json`. The NPM package `@tauri-apps/plugin-opener` (v2.5.3) is added to `package.json`. All four wiring points are present and version-aligned.

2. **`revealSelectedFile()` in `main.ts`:** Correctly guards against `null` file ID and missing filepath. Error handling uses `console.warn` + `ToastContainer.show("error", ...)`, matching the existing patterns for `shortcut:delete` and other actions. The `revealItemInDir` import from `@tauri-apps/plugin-opener` is the correct API (accepts a file path, reveals it in the native file browser with the file selected).

3. **Event wiring:** Both `toolbar:reveal-in-folder` and `shortcut:reveal-in-folder` are subscribed in `initEventHandlers()` and properly cleaned up via the returned unsub array, consistent with HMR teardown.

4. **Toolbar button:** The reveal button is created with the correct class `toolbar-btn-reveal` and disabled when no single file is selected (`!hasFile || hasMulti`). This correctly prevents the action when zero or multiple files are selected.

5. **Shortcut (`Cmd/Ctrl+Shift+R`):** The check `mod && e.shiftKey && (e.key === "r" || e.key === "R")` is correct. It is placed before the `isInputFocused()` guard, meaning it works even when typing in an input field, which is appropriate for a "reveal" action that does not modify input content. The `e.preventDefault()` call correctly prevents the browser/webview from interpreting it as a page reload.

6. **`addClickableInfoRow()` in `MetadataPanel.ts`:** The directory path extraction regex `filepath.replace(/[\\/][^\\/]+$/, "")` correctly strips the last path component on both Unix and Windows separators. The displayed value is the directory, while `revealItemInDir` is called with the full filepath (correct semantic: reveal the file inside its parent directory). The `.catch()` handler prevents unhandled rejection. The event listener is cleaned up implicitly when `renderFileInfo` calls `this.el.innerHTML = ""`, matching the existing pattern for all other DOM created in this method.

7. **CSS styles:** `.metadata-info-link` applies `cursor: pointer`, accent color, and a `text-decoration: underline` with `text-decoration-color: transparent` that transitions to visible on hover. The parent class `metadata-info-value` already provides `overflow: hidden` and `text-overflow: ellipsis`, so long directory paths are handled correctly.

8. **No memory leaks:** The click handler on the info link is attached to DOM that gets cleared on re-render. No new EventBus subscriptions are created without corresponding cleanup. All event subscriptions in `initEventHandlers()` are tracked in the unsub array and torn down on HMR dispose.

9. **Security:** `revealItemInDir` is a Tauri-provided API that opens the OS file browser. The `opener:default` permission scope does not grant arbitrary shell execution. The filepath comes from the database (originally from the scanner), not from user text input, so path injection is not a concern.

All changes are minimal, well-scoped, and follow existing conventions.
