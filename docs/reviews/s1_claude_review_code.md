# Code Review: Sprint 1 — Data Model & Metadata Extension (Cycle 2)
## Reviewer: Claude CLI (code review)
## Date: 2026-03-15

## Verification of 3 Prior-Cycle Fixes

### Fix 1: skill_level server-side validation in update_file
**Status: Correctly applied.**
- `src-tauri/src/commands/files.rs` lines 639-648: When `skill_level` is `Some`, empty strings are allowed (clearing the field), and non-empty values are validated against the whitelist `["beginner", "easy", "intermediate", "advanced", "expert"]`. Invalid values return `AppError::Validation`. The value is then written to the SET clause regardless, which is correct (empty string clears the field).

### Fix 2: Bool filter label renamed "Status" to "KI-Status" in SearchBar
**Status: Correctly applied.**
- `src/components/SearchBar.ts` line 430: `lbl.textContent = "KI-Status";` — the boolean filter group (aiAnalyzed / aiConfirmed) is now correctly labeled "KI-Status", avoiding confusion with the separate "Status" dropdown filter at line 250 (which filters the project workflow status field).

### Fix 3: URL scheme whitelist (http/https only) in addLinkField
**Status: Correctly applied.**
- `src/components/MetadataPanel.ts` line 797: The external link button is only rendered when `value && /^https?:\/\//i.test(value)`, preventing `javascript:`, `data:`, `file:`, or other dangerous URI schemes from being rendered as clickable links.
- The `<input>` uses `type="url"` (line 788) which provides browser-level validation.
- The `<a>` element includes `rel="noopener noreferrer"` and `target="_blank"` (lines 809-810).

## Full Review

### SQL Migration (migrations.rs)
- **v9 migration**: Correctly adds `file_type`, `size_range`, `skill_level`, `language`, `format_type`, `file_source`, `purchase_link`, and `status` columns to `embroidery_files`.
- `file_type` defaults to `'embroidery'` and `status` defaults to `'none'` — consistent with frontend expectations.
- FTS5 table is properly rebuilt: old triggers dropped, old table dropped, new table created with 3 additional columns (`language`, `file_source`, `size_range`), triggers recreated with all 14 columns.
- `CURRENT_VERSION` is 9, migration chain runs correctly (v1 through v9).
- Indexes on `file_type` and `status` are appropriate for filter queries.
- Tests verify schema version 9 and idempotent migration.

### Rust Models (models.rs)
- `EmbroideryFile` struct includes all 7 new fields with correct types (`Option<String>` for nullable, `String` for NOT NULL with defaults).
- `FileUpdate` struct exposes all 7 new fields as `Option<String>` for partial updates.
- `SearchParams` includes `file_type`, `status`, `skill_level`, `language`, `file_source` filters.
- `PaginatedFiles` struct is correct.
- Field ordering in `EmbroideryFile` matches the SELECT column order in queries.rs.

### Query Layer (queries.rs)
- `FILE_SELECT` and `FILE_SELECT_ALIASED` include all 37 columns in the correct order matching `row_to_file` indices 0-36.
- `row_to_file` maps all 37 columns correctly to the `EmbroideryFile` struct.

### Commands (files.rs)
- `build_query_conditions`: All new SearchParams filters (`file_type`, `status`, `skill_level`, `language`, `file_source`) are handled with proper trimming and parameterized queries.
- `update_file`: All 7 new fields are handled. `skill_level` has server-side whitelist validation (allows empty to clear). `status` has server-side whitelist validation (rejects empty). Both are correct.
- `update_file_status`: Separate command with its own status whitelist validation — consistent.
- Both `get_files` and `get_files_paginated` use the shared `build_query_conditions` — no duplication.

### Frontend Types (types/index.ts)
- `EmbroideryFile` interface includes all 7 new fields with correct types.
- `FileUpdate` interface includes all 7 new fields as optional.
- `SearchParams` includes the 5 new filter fields.
- Field naming follows camelCase convention consistently.

### MetadataPanel (MetadataPanel.ts)
- `FormSnapshot` includes all new fields.
- `takeSnapshot`, `getCurrentFormValues`, `checkDirty` all handle the new fields.
- `save()` method sends only changed fields (delta updates) — correct.
- Sewing pattern section is conditionally shown only for `fileType === "sewing_pattern"`.
- Status section is shown for all files.
- `addLinkField` renders `type="url"` input with http/https-only link button guard.
- Skill level uses a `<select>` with an empty "-- Auswahlen --" option, matching the backend whitelist + empty string allowance.

### SearchBar (SearchBar.ts)
- `activeFilterCount` correctly counts all 14 possible filter dimensions.
- All new filters (fileType, status, skillLevel, language, fileSource) have both select/text filter builders and active chip renderers with German labels.
- "KI-Status" label for the boolean filter group is correct.
- Panel cleanup properly destroys TagInput and removes outside click handler.

### Security
- SQL injection: All queries use parameterized statements. FTS5 input is sanitized by stripping special characters.
- XSS via link: `addLinkField` only renders clickable `<a>` for `https?://` URLs. The `rel="noopener noreferrer"` attribute is set.
- Server-side validation: `skill_level` and `status` both have whitelist validation. `purchase_link` has no server-side URL validation (relies on frontend `type="url"` and the http/https display guard). This is acceptable since the field stores user data and the security concern is about rendering, not storage.
- Path traversal: Not applicable to these changes (no file path operations).

### Tests
- Migration tests verify schema version 9, table existence, idempotent re-run, cascade deletes.
- `test_update_file` tests basic update at the SQL level.
- Search tests cover combined filters, empty params, and legacy search.
- Note: Migration function definitions (v6, v7, v8) remain out of source order, but execution order is correct via `run_migrations`. This is a pre-existing code organization observation carried from the prior review, not a defect.

## Findings

Code review passed. No findings.

## Summary

All three prior-cycle fixes are correctly applied and complete. The v9 migration, Rust models, query layer, commands, frontend types, MetadataPanel, and SearchBar are consistent and well-integrated. Server-side validation exists for both `skill_level` (whitelist with empty-to-clear) and `status` (whitelist). The URL scheme whitelist in `addLinkField` properly prevents dangerous URI schemes from being rendered as clickable links. The "KI-Status" label correctly disambiguates the boolean AI filters from the workflow status dropdown. No new issues were introduced.
