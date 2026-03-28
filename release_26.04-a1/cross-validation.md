# Cross-Validation Report
**Date:** 2026-03-17
**Release:** 26.04-a1

## Methodology

Two independent agents (Claude Reviewer, Codex Reviewer) executed 117 tests each (67 functional, 15 performance, 35 security). Findings were compared and disputed items verified by reading the actual source code. Only cross-validated findings proceed to GitHub issues.

## Agent Finding Summary

| Category | Claude Findings | Codex Findings |
|----------|----------------|----------------|
| Functional | 5 (1H, 3M, 1L) | 2 (2M) |
| Performance | 4 (1H, 2M, 1L) | 3 (2M, 1L) |
| Security | 8 (3H, 4M, 1L) | 6 (2H, 3M, 1L) |
| **Total** | **17** | **11** |

## Cross-Validation Results

### VALIDATED — Both Agents Agree (6 findings)

| ID | Severity | Finding | Claude | Codex |
|----|----------|---------|--------|-------|
| ST-14 | **High** | Plaintext API keys in SQLite | High | High |
| ST-05 | Medium | innerHTML fragile pattern, escapeHtml not shared | Medium | High |
| ST-12 | Medium | sql:default grants unrestricted frontend DB access | Medium | Medium |
| ST-17 | Medium | CSP unsafe-inline in style-src | Medium | Medium |
| FT-64 | Medium | Missing focus traps in dialogs | Medium (2 dialogs) | Medium (broader) |
| PT-15 | Low | Startup: ANALYZE unconditional + sync init | Low (sync init) | Low (ANALYZE) |

### VALIDATED — Claude Only, Confirmed by Code Review (4 findings)

| ID | Severity | Finding | Evidence |
|----|----------|---------|----------|
| FT-67a | Medium | FileList 5000-file limit, no pagination | `FileList.ts:57`: `getFilesPaginated(..., 0, 5000)` hardcoded |
| PT-05 | Low | Per-file SQL queries in batch operations | `batch.rs:126-143`: individual `query_row` per file_id |
| FT-63 | Low | Arrow keys don't scroll FileList to selected | `main.ts:936-940`: handlers set selectedFileId but no scroll |
| ST-18 | Low | CSP missing form-action, frame-ancestors | Per CSP spec, not fallback from default-src. Theoretical for desktop. |

### VALIDATED — Codex Only, Confirmed by Code Review (2 findings)

| ID | Severity | Finding | Evidence |
|----|----------|---------|----------|
| FT-67b | Medium | No unsaved-changes guard on file switch | `MetadataPanel.ts:78-113`: `onSelectionChanged` loads without dirty check |
| ST-09 | Low | open_attachment no app-data-dir verification | `files.rs:1238-1283`: path from DB, traversal-checked but not constrained to app dir |

### NOT VALIDATED — Rejected After Review (5 findings)

| ID | Agent | Reason for Rejection |
|----|-------|---------------------|
| ST-27 | Claude | 155+ commands with no rate limiting — accepted desktop architecture |
| ST-11 | Claude | Unicode normalization bypass — OS handles correctly, theoretical only |
| ST-20 | Claude | Log injection — debug-only logging, no production risk |
| FT-66 | Claude | Splitter not persisted — feature request, not a defect |
| PT-14 | Codex | Backend thumbnail cache unbounded — disk storage, not memory; acceptable growth |

### Severity Reconciliation

Where agents disagreed on severity:
- **ST-05**: Claude=Medium, Codex=High → **Medium** (currently safe, fragile pattern)
- **PT-09**: Claude=High, Codex=Medium → **Medium** (multiple concerns but manageable)

## Final Validated Findings (13 total)

| # | ID | Severity | Category | Title |
|---|---|----------|----------|-------|
| 1 | ST-14 | **High** | Security | Plaintext API keys stored in SQLite |
| 2 | ST-05 | Medium | Security | innerHTML fragile pattern, escapeHtml not shared utility |
| 3 | ST-12 | Medium | Security | sql:default grants unrestricted frontend DB access |
| 4 | ST-17 | Medium | Security | CSP unsafe-inline in style-src |
| 5 | FT-64 | Medium | Functional | Missing focus traps in ManufacturingDialog, ProjectListDialog |
| 6 | FT-67a | Medium | Functional | FileList hardcoded 5000-file limit without pagination |
| 7 | FT-67b | Medium | Functional | No unsaved-changes guard when switching files |
| 8 | PT-09 | Medium | Performance | Deep-copy state for large file arrays |
| 9 | ST-09 | Low | Security | open_attachment doesn't verify path within app data dir |
| 10 | ST-18 | Low | Security | CSP missing form-action and frame-ancestors |
| 11 | PT-05 | Low | Performance | Batch operations use per-file SQL queries |
| 12 | PT-15 | Low | Performance | ANALYZE runs unconditionally on every startup |
| 13 | FT-63 | Low | Functional | Arrow key navigation doesn't scroll to selected item |
