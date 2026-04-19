# Wave 3 Security Review (regression check) — 2026-04-19

## Summary
PASS. The Wave 3 usability diff introduces no new security issues. `ConfirmDialog` and `InputDialog` use `textContent` (and `placeholder`/`value` on a string-typed `<input type="text">`) for all caller-supplied content — no `innerHTML`, no template interpolation into HTML. `Splitter` persists only an internal numeric value (`String(clamped)`, clamped to `[min, max]` and routed through `Math.min/Math.max`) under a fixed key derived from a constructor-time CSS property name (`splitter:<prop>`); both arguments to `SettingsService.setSetting` flow into a parameterised Tauri `invoke`, so SQL/IPC injection is not possible. `Toast`'s new close button uses `textContent = "\u00D7"` and a static `aria-label`. The umlaut sweep is pure string-content (still passed through `textContent`); no escape contexts were converted to `innerHTML`. Focus traps in `DocumentViewer`/`ImageViewerDialog`/`PrintPreviewDialog` are pure JS. The single new `innerHTML` (in `ProjectListDialog.ts:632`) assigns a static literal with no user input. Wave 1 sanitisers (`sanitizeRichText`, `escapeHtml`) and Wave 2 perf fixes are intact.

## New findings (introduced by this diff)
No new findings.
