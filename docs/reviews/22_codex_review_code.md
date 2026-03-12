# Codex Code Review — Global Search (Issue #22), Round 2

**Reviewer:** Codex-style (code review)
**Date:** 2026-03-12
**Scope:** Sidebar.ts, Toolbar.ts, StatusBar.ts — Round 2 fixes for "Alle Ordner" visibility and scan tooltip

---

## Findings

No findings.

## Summary

All three changes from the Round 2 diff are correct:

1. **"Alle Ordner" now visible even when folder list is empty** — The `<ul>` and "Alle Ordner" `<li>` are created before the `folders.length === 0` check. The early-return path now appends the list before the empty-state message, so "Alle Ordner" with count 0 is always rendered. This addresses the Round 1 finding (point 7 in the previous review) where the "Alle Ordner" entry was not visible with zero folders.

2. **Scan button tooltip** — `scanBtn.title` is set conditionally in `updateButtonStates()`: `"Ordner scannen"` when a folder is selected, `"Ordner auswaehlen, um zu scannen"` when disabled. This overwrites the initial title from `createButton()`, which is correct since `updateButtonStates()` runs immediately after `render()` and on every relevant state change. The tooltip provides clear UX guidance for why the button is disabled in "Alle Ordner" mode.

3. **StatusBar label** — `"Alle Ordner"` replaces `"Kein Ordner ausgewaehlt"`, consistent with sidebar terminology.

**Edge cases verified:**
- Empty folder list: "Alle Ordner" renders with count 0, empty message appears below it.
- App start with `selectedFolderId === null`: "Alle Ordner" selected, FileList loads all files via backend `folder_id: None`.
- Scan button after scan completes: `updateButtonStates()` in `finally` block updates both `disabled` and `title`.
- Folder re-click toggle: state reads are live at click time, no stale closure.

No correctness, state management, or consistency issues found.
