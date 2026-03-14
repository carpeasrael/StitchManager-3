# Sprint 16 Analysis — Event System, UI Polish & Missing Menus

**Date:** 2026-03-14
**Issues:** #61, #62, #63, #64, #66
**Severity:** All medium

---

## #61 — Missing AI event bridge listeners
Add ai:start, ai:complete, ai:error to initTauriBridge() in main.ts.

## #62 — Escape key propagation in TagInput and ImagePreviewDialog
Add e.stopPropagation() in both Escape handlers.

## #63 — SearchBar outsideClickHandler leak
Remove previous outsideClickHandler before registering new one in renderPanel().

## #64 — Implemented features not exposed in UI
Add Convert, Transfer, Edit/Transform menu items to Toolbar burger menu.

## #66 — attach_file unbounded dedup loop
Cap counter at 100,000 and return error on exhaustion, matching batch.rs dedup_path pattern.
