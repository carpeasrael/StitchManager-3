# Codex Code Review — #65 Cycle 2

**Reviewer:** Codex CLI
**Files reviewed:** `index.html`, `src/components/FileList.ts`, `src/components/StatusBar.ts`, `src/components/FilterChips.ts`
**Date:** 2026-03-14

---

## Previous Findings (Cycle 1)

- **Finding 1 (Medium):** Role conflict on `.app-status` — `index.html` set `role="contentinfo"` while `StatusBar.render()` overwrote it with `role="status"`. **Fixed:** `index.html` now uses `role="status"` directly; `StatusBar.render()` no longer sets any role. Confirmed resolved.
- **Finding 2 (Low):** `role="status"` was redundantly set on every `render()` call. **Fixed:** The role is now set only in `index.html`, not in JavaScript at all. Confirmed resolved.

---

## Cycle 2 Review

### index.html
- ARIA landmark roles (`banner`, `navigation`, `main`, `complementary`, `status`) are correctly applied to the layout containers.
- `aria-label` attributes on `app-sidebar` ("Ordner") and `app-right` ("Dateidetails") provide meaningful context.
- Splitter dividers correctly marked `aria-hidden="true"`.
- No issues found.

### FileList.ts
- `role="list"` and `aria-label="Dateien"` correctly added to the scroll container.
- `role="listitem"` and `aria-label` correctly added to each file card.
- `aria-label` on cards falls back from `file.name` to `file.filename`, which is correct.
- Virtual scrolling, selection logic, thumbnail caching, and attachment count batching are unchanged and consistent.
- No issues found.

### StatusBar.ts
- No changes in this cycle. `role="status"` is no longer set here, correctly relying on `index.html`.
- Existing code is clean: watcher status querying, USB device rendering, and version display are all consistent.
- No issues found.

### FilterChips.ts
- `role="toolbar"` and `aria-label="Formatfilter"` correctly added to the wrapper.
- `aria-pressed` correctly reflects the active state for each chip button, including the "Alle" chip.
- Toggle logic (`current === fmt ? null : fmt`) is correct.
- No issues found.

---

## Summary

All four files reviewed. **Zero findings.** The two findings from Cycle 1 (role conflict and redundant setAttribute) have been properly resolved. ARIA attributes are correctly applied and consistent across all reviewed files.
