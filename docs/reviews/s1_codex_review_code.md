# Code Review: Sprint 1 — Data Model & Metadata Extension (Cycle 2)
## Reviewer: Codex CLI (code review)
## Date: 2026-03-15

## Verification of Prior Findings

### Finding 1: Missing server-side validation for skill_level
**Status: FIXED**
In `src-tauri/src/commands/files.rs` lines 639-645, `update_file` now validates `skill_level` against a whitelist `["beginner", "easy", "intermediate", "advanced", "expert"]` before persisting. Empty strings are allowed (to clear the field). The validation returns `AppError::Validation` with a German-language message on mismatch. Correct.

### Finding 2: Ambiguous "Status" label in SearchBar
**Status: FIXED**
In `src/components/SearchBar.ts` line 430, the `buildBoolFilter` method now uses label `"KI-Status"` instead of the generic `"Status"`, disambiguating it from the new project-status filter. Correct.

### Finding 3: Unsanitized URL scheme in addLinkField
**Status: FIXED**
In `src/components/MetadataPanel.ts` line 797, the link button is only rendered when `value` matches `/^https?:\/\//i`, effectively whitelisting only `http://` and `https://` schemes. The `href` is only set on values that pass this check. Correct.

## Full Code Review

### Migration (`src-tauri/src/db/migrations.rs`)
- Migration v9 adds all 7 new columns (`file_type`, `size_range`, `skill_level`, `language`, `format_type`, `file_source`, `purchase_link`, `status`) with correct defaults.
- `file_type` defaults to `'embroidery'` (NOT NULL), `status` defaults to `'none'` (NOT NULL). Other columns are nullable. Correct.
- FTS5 index is rebuilt to include `language`, `file_source`, `size_range`. Triggers are recreated for all 14 indexed columns. Correct.
- Indexes added for `file_type` and `status`. Correct.
- Schema version recorded as 9. Correct.

### Models (`src-tauri/src/db/models.rs`)
- `EmbroideryFile` struct includes all new fields with correct types (`String` for non-nullable, `Option<String>` for nullable). Correct.
- `FileUpdate` struct includes all new editable fields as `Option<String>`. Correct.
- `SearchParams` struct includes `file_type`, `status`, `skill_level`, `language`, `file_source` filter fields. Correct.

### Queries (`src-tauri/src/db/queries.rs`)
- `FILE_SELECT` and `FILE_SELECT_ALIASED` include all new columns in correct positional order matching `row_to_file`. Verified: 37 columns (indices 0-36), all aligned. Correct.
- `row_to_file` maps all 37 columns positionally. Correct.

### Commands (`src-tauri/src/commands/files.rs`)
- `build_query_conditions` handles all new `SearchParams` fields with parameterized queries. Correct.
- `update_file` validates `skill_level` and `status` against whitelists. Correct.
- `update_file_status` also validates status against the same whitelist. Correct.
- LIKE fallback in text search includes `language`, `file_source`, `size_range`. Matches FTS5 columns. Correct.

### Frontend Types (`src/types/index.ts`)
- `EmbroideryFile` interface includes all new fields with correct TypeScript types. Correct.
- `FileUpdate` interface includes all new editable fields as optional strings. Correct.
- `SearchParams` interface includes all new filter fields. Correct.

### MetadataPanel (`src/components/MetadataPanel.ts`)
- `FormSnapshot` includes all new fields. Correct.
- `takeSnapshot`, `getCurrentFormValues`, `checkDirty`, `save` all handle new fields consistently. Correct.
- Sewing pattern section conditionally rendered for `file_type === "sewing_pattern"`. Correct.
- Status section rendered for all file types. Correct.
- `addLinkField` validates URL scheme before rendering clickable link. Correct.

### SearchBar (`src/components/SearchBar.ts`)
- `activeFilterCount` counts all new filter fields. Correct.
- `buildBoolFilter` uses `"KI-Status"` label. Correct.
- New select filters for file type, status, skill level, and text filters for language and source. Correct.
- Active chips section handles all new filters with German labels and clear functions. Correct.

## Findings

Code review passed. No findings.

## Summary

All three prior findings have been correctly fixed. The full implementation across migration, models, queries, commands, frontend types, MetadataPanel, and SearchBar is consistent and complete. No new issues identified.
