# S7 Claude Code Review

**Reviewer:** Claude CLI (code review)
**Date:** 2026-03-16
**Scope:** files.rs (build_query_conditions, build_order_clause, escape_like), models.rs (SearchParams), SearchBar.ts, FileList.ts (createCard, file type badge), types/index.ts, components.css
**Prior findings checked:** fileType in inline type (fixed), ESCAPE clause on LIKE (fixed)

## Verdict: PASS

Code review passed. No findings.

### Review notes

1. **escape_like** - Correctly escapes `\`, `%`, and `_` in the right order (backslash first). Used consistently in all LIKE clauses.

2. **ESCAPE clause** - All LIKE queries include `ESCAPE '\\'` (lines 75, 178, 231, 239, 247 of files.rs). The prior finding is confirmed fixed.

3. **build_query_conditions** - Parameterized queries throughout, no SQL injection vectors. FTS5 input sanitization strips special characters. Tag search uses AND logic via individual EXISTS subqueries. The `param_idx` counter is correctly incremented after each parameter binding.

4. **build_order_clause** - Allowlist-based field validation prevents SQL injection in ORDER BY. Direction is restricted to ASC/DESC via match.

5. **SearchParams (Rust)** - All fields properly typed with `Option<T>`, `serde(rename_all = "camelCase")` ensures correct JSON mapping. The `fileType` field is present as `file_type: Option<String>`.

6. **SearchParams (TypeScript)** - Mirrors the Rust struct exactly with matching camelCase field names. All fields are optional.

7. **EmbroideryFile (TypeScript)** - `fileType: string` is a required field, consistent with the Rust model where it has a default value.

8. **FileList.ts createCard** - The inline type parameter includes `fileType: string` (prior finding confirmed fixed). File type badge renders correctly for non-embroidery types with proper CSS class mapping.

9. **SearchBar.ts** - Debounce timer properly cleaned up in `destroy()`. Outside click handler properly managed (removed before re-render, removed on close). `activeFilterCount` covers all filter fields. Panel TagInput is destroyed on close/re-render to prevent leaks.

10. **components.css** - File type badge styles cover `type-embroidery`, `type-sewing_pattern`, and `type-document`. Search panel uses `position: fixed` with z-index 90. All interactive elements have hover states.
