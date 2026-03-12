# Codex Task-Resolution Review -- Go to Location (Issue #20)

**Reviewer:** Codex-style (task resolution)
**Date:** 2026-03-12

---

## Checklist

- [x] Toolbar button to reveal file in folder
  - `Toolbar.ts` line 48-52: button with id `toolbar-btn-reveal`, label "Im Ordner anzeigen", emits `toolbar:reveal-in-folder`
- [x] Keyboard shortcut (Cmd/Ctrl+Shift+R)
  - `shortcuts.ts` lines 27-32: intercepts Cmd/Ctrl+Shift+R and emits `shortcut:reveal-in-folder`
- [x] Clickable path in MetadataPanel
  - `MetadataPanel.ts` lines 308-309: calls `addClickableInfoRow` for the filepath
  - `MetadataPanel.ts` lines 946-968: `addClickableInfoRow` renders a clickable link that calls `revealItemInDir(filepath)`
- [x] Plugin properly wired (Rust, capabilities, NPM)
  - `Cargo.toml` line 38: `tauri-plugin-opener = "2.5.3"`
  - `lib.rs` line 20: `.plugin(tauri_plugin_opener::init())`
  - `capabilities/default.json` line 12: `"opener:default"`
  - `package.json` line 17: `"@tauri-apps/plugin-opener": "^2.5.3"`
- [x] Error handling present
  - `main.ts` lines 131-136: `revealSelectedFile` catches errors with console.warn and shows an error toast
  - `MetadataPanel.ts` lines 961-963: clickable path catches errors with console.warn
- [x] Button disabled when no file selected
  - `Toolbar.ts` line 131: `revealBtn.disabled = !hasFile || hasMulti` -- disabled when no file or multiple files selected
- [x] Builds pass (npm + cargo)
  - `npm run build`: passes (tsc + vite, 33 modules, no errors)
  - `cargo check`: passes (1 warning for unused `invalidate` method, unrelated)

## Event wiring verification

- `toolbar:reveal-in-folder` emitted by Toolbar button -> handled in `main.ts` line 163 -> calls `revealSelectedFile()`
- `shortcut:reveal-in-folder` emitted by keyboard shortcut -> handled in `main.ts` line 164 -> calls `revealSelectedFile()`
- Both paths converge on `revealSelectedFile()` (lines 123-137) which uses `revealItemInDir` from `@tauri-apps/plugin-opener`

## Findings

Task resolved. No findings.
