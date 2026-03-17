# Code Review: Issue #119 — Round 2

## Round 1 Fix Verification

| # | Round 1 Finding | Status |
|---|----------------|--------|
| 1 | status='active' in upload_sewing_pattern | **Resolved.** Line 1250 now uses `'none'`. |
| 2 | No HTML sanitization on instructionsHtml | **Resolved.** `sanitize_html` strips `<script>` and `<style>` tags; applied in both upload (line 1262) and update (line 775). |
| 3 | Deduplication loop may silently overwrite | Not in fix scope. Unchanged. |
| 4 | No instructionsHtml size limit | **Resolved.** 100 KB limit enforced in upload (line 1157-1162) and update (line 771-773). |
| 5 | document.execCommand deprecated | Not in fix scope. Informational. |
| 6 | Rating dirty tracking false positive | **Resolved.** Snapshot initializes rating to `"0"` for null (line 149), star widget sets `dataset.rating = "0"` (line 490), `getCurrentFormValues` reads consistently (line 222). |
| 7 | No test coverage for upload_sewing_pattern | Not in fix scope. Informational. |
| 8 | Star rating missing ARIA role/label | **Resolved.** Both PatternUploadDialog (lines 219-220) and MetadataPanel (lines 488-489) have `role="radiogroup"` and `aria-label="Bewertung"`. |

All six targeted fixes (1, 2, 4, 6, 8, plus deleted_at IS NULL on line 1279) are correctly applied.

## New Findings

1. **Unused variable `lower` in `sanitize_html`** (`src-tauri/src/commands/files.rs:1106`). The line `let lower = html.to_lowercase();` computes a value that is never referenced — each loop iteration calls `result.to_lowercase()` directly instead. This dead code will produce a Rust compiler warning (`unused variable: lower`). Remove the line.
