# Sprint 8 — Codex Code Review (Issue #28: 50k+ file performance)

**Scope:** `migrations.rs` (apply_v6), `commands/files.rs` (FTS5 search, get_files_paginated, get_thumbnails_batch), `FileList.ts` (getRef, batch thumbnails, data-file-id, O(1) Map lookup), `FileService.ts` (new API methods)

---

## Findings

### Finding 1 — `param_idx` incremented even when no FTS condition/param is pushed (BUG)

**File:** `src-tauri/src/commands/files.rs`, lines 47-76

When the FTS5 table exists but the sanitized search string is empty (i.e., the user entered only FTS5 special characters like `"*+-`), the code correctly skips pushing a condition and parameter. However, `*param_idx += 1` at line 75 executes unconditionally within the `if let Some(ref q) = text_query` block, regardless of whether a param was actually pushed. This causes a mismatch between the `?N` parameter indices used in subsequent conditions (e.g., format_filter, search_params) and the actual number of parameters in the `params` Vec.

For example, if a user searches for `"***"` with a format filter of `PES`:
- FTS exists, sanitized is empty, no condition/param pushed, but `param_idx` becomes 2
- Format filter pushes a condition with `?2` and one param
- The params Vec has one element at index 0, but rusqlite expects `?2` to map to index 1
- This causes a runtime parameter binding error

**Recommendation:** Move `*param_idx += 1` inside the `if !sanitized.is_empty()` block (and keep it in the `else` LIKE branch). Alternatively, only increment when a param is actually pushed.

**Severity:** Medium (edge case, requires all-special-character input combined with another filter)

---

### Finding 2 — Attachment counts loop does not use O(1) Map lookup (PERFORMANCE, INCONSISTENCY)

**File:** `src/components/FileList.ts`, lines 196-217

The thumbnail batch update (lines 162-193) correctly builds a `cardsByFileId` Map for O(1) card lookups. However, the attachment counts batch update (lines 196-217) iterates over all `renderedCards` for each file ID in the response, using `getCardFileId()` on each card. While `getCardFileId` is now O(1) thanks to `data-file-id`, the overall pattern is still O(k*n) where k is the number of counts and n is the number of rendered cards. This is inconsistent with the thumbnail approach and could be unified.

**Recommendation:** Build a `cardsByFileId` Map (or share the one from thumbnail loading if both run in the same cycle) and use direct Map lookups for attachment count updates.

**Severity:** Low (n is bounded by visible cards + buffer, typically 30-50)

---

## Summary

| # | Severity | Description |
|---|----------|-------------|
| 1 | Medium | `param_idx` incremented without pushing param when FTS sanitized string is empty |
| 2 | Low | Attachment counts loop uses O(k*n) iteration instead of O(1) Map lookup |

**Findings count: 2 actionable findings (1 Medium, 1 Low).**
