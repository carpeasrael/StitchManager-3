# Release Test Report — StitchManager 26.04-a1

**Date:** 2026-03-17
**Version:** 26.4.1
**Auditors:** Claude Reviewer Agent, Codex Reviewer Agent
**Methodology:** Independent dual-agent static analysis with cross-validation

---

## Executive Summary

StitchManager underwent a comprehensive pre-release audit covering functional correctness, performance, and security. Two independent agents executed 117 tests each across the full codebase (47 TypeScript files, 155+ Rust commands, 39+ database tables). Findings were cross-validated, and only verified issues were documented.

**Verdict: CONDITIONAL RELEASE** — The application is well-engineered with solid security fundamentals. One High-severity finding (plaintext API key storage) should be addressed before production deployment. All other findings are Medium or Low severity.

---

## Baseline Results

| Check | Result |
|-------|--------|
| Rust tests (197) | **ALL PASSED** |
| TypeScript build | **PASSED** (warning: 780KB bundle > 500KB) |
| npm audit | **0 vulnerabilities** |
| cargo audit | **8 warnings** (unmaintained GTK3 bindings — transitive from Tauri, not actionable) |

---

## Test Coverage

| Category | Tests | Passed | Findings |
|----------|-------|--------|----------|
| Functional (FT-01..FT-67) | 67 | 63 | 4 |
| Performance (PT-01..PT-15) | 15 | 12 | 3 |
| Security (ST-01..ST-35) | 35 | 29 | 6 |
| **Total** | **117** | **104** | **13** |

---

## Validated Findings Summary

### High Severity (1)

| # | Issue | ID | Title | Status |
|---|-------|----|-------|--------|
| 1 | [#102](https://github.com/carpeasrael/StitchManager-3/issues/102) | ST-14 | Plaintext API keys stored in SQLite | Open |

### Medium Severity (7)

| # | Issue | ID | Title | Status |
|---|-------|----|-------|--------|
| 2 | [#103](https://github.com/carpeasrael/StitchManager-3/issues/103) | ST-05 | innerHTML pattern fragile, escapeHtml not shared | Open |
| 3 | [#104](https://github.com/carpeasrael/StitchManager-3/issues/104) | ST-12 | sql:default grants unrestricted frontend DB access | Open |
| 4 | [#105](https://github.com/carpeasrael/StitchManager-3/issues/105) | ST-17 | CSP unsafe-inline in style-src | Open |
| 5 | [#106](https://github.com/carpeasrael/StitchManager-3/issues/106) | FT-64 | Missing focus traps in ManufacturingDialog, ProjectListDialog | Open |
| 6 | [#107](https://github.com/carpeasrael/StitchManager-3/issues/107) | FT-67a | FileList hardcoded 5000-file limit | Open |
| 7 | [#108](https://github.com/carpeasrael/StitchManager-3/issues/108) | FT-67b | No unsaved-changes guard on file switch | Open |
| 8 | [#109](https://github.com/carpeasrael/StitchManager-3/issues/109) | PT-09 | Deep-copy state for large file arrays | Open |

### Low Severity (5)

| # | Issue | ID | Title | Status |
|---|-------|----|-------|--------|
| 9 | [#110](https://github.com/carpeasrael/StitchManager-3/issues/110) | ST-09 | open_attachment path not constrained to app dir | Open |
| 10 | [#111](https://github.com/carpeasrael/StitchManager-3/issues/111) | ST-18 | CSP missing form-action, frame-ancestors | Open |
| 11 | [#112](https://github.com/carpeasrael/StitchManager-3/issues/112) | PT-05 | Per-file SQL queries in batch operations | Open |
| 12 | [#113](https://github.com/carpeasrael/StitchManager-3/issues/113) | PT-15 | ANALYZE unconditional on every startup | Open |
| 13 | [#114](https://github.com/carpeasrael/StitchManager-3/issues/114) | FT-63 | Arrow keys don't scroll to selected file | Open |

---

## Security Posture

### Strengths (verified by both agents)

- **SQL Injection:** All 155+ commands use parameterized queries. FTS5 metacharacters sanitized. LIKE wildcards escaped. ORDER BY uses allowlist. **Zero SQL injection vectors found.**
- **XSS Prevention:** No `eval()` or `Function()` usage. `textContent` preferred for dynamic data. `escapeHtml()` used where innerHTML has dynamic content.
- **Path Traversal:** `validate_no_traversal()` using `Path::components()` analysis applied across 15+ file-handling commands. Batch operations use `sanitize_path_component()` and canonical path verification.
- **Input Validation:** Comprehensive validation at all boundaries (file size, field types, status allowlists, delivery quantities, path existence).
- **Error Handling:** `AppError` enum with structured JSON serialization. No stack trace leakage. German user-facing messages.
- **Dependencies:** npm audit clean. cargo audit shows only transitive GTK3 maintenance warnings (upstream Tauri dependency).
- **No unsafe code:** Zero `unsafe` blocks in application code.
- **No hardcoded secrets:** Source code clean.

### Weaknesses

- **API key storage (High):** Plaintext in SQLite instead of OS keychain
- **CSP:** `unsafe-inline` for styles, missing form-action/frame-ancestors
- **Frontend DB access:** `sql:default` bypasses backend validation
- **innerHTML pattern:** Fragile; escapeHtml not shared across components

### OWASP Top 10 Coverage

| OWASP Category | Status | Notes |
|----------------|--------|-------|
| A01 Broken Access Control | **Addressed** | Path traversal protection, capability restrictions |
| A02 Cryptographic Failures | **Finding** | ST-14: plaintext API key storage |
| A03 Injection | **Clean** | All SQL parameterized, no XSS, no command injection |
| A04 Insecure Design | **Clean** | Three-phase batch, mutex-protected DB, error propagation |
| A05 Security Misconfiguration | **Finding** | ST-17/ST-18: CSP gaps |
| A06 Vulnerable Components | **Clean** | npm/cargo audit clean |
| A07 Auth Failures | **N/A** | Single-user desktop app |
| A08 Data Integrity | **Clean** | FK cascades, transactions, audit logging |
| A09 Logging Failures | **Clean** | Debug-only logging, structured events |
| A10 SSRF | **N/A** | AI calls from backend only |

---

## Performance Assessment

### Strengths

- **Virtual scrolling:** Efficient RAF-based rendering, ~23 DOM nodes for 10K files
- **FTS5 search:** O(log n) indexed search with proper fallback
- **Debouncing:** 300ms search, 500ms file watcher, RAF scroll handler
- **Three-phase batch:** Lock minimization, filesystem rollback
- **WAL mode:** Concurrent read support

### Improvement Opportunities

- Batch operations: per-file queries → batch IN() queries
- State management: structuredClone on large arrays → getRef for reads
- Startup: conditional ANALYZE, async service initialization

---

## Functional Completeness

### Features Verified (63 of 67 tests passed)

- Folder CRUD with cascading delete
- File import/export across 6 formats (PES, DST, JEF, VP3, PDF, images)
- Full-text search with 18+ advanced filter types
- AI integration (Ollama + OpenAI) with per-field accept/reject
- Batch rename/organize/USB export with rollback
- Manufacturing subsystem (suppliers, materials, BOM, workflow, quality)
- Procurement (purchase orders, deliveries, suggestions)
- Project management with collections
- Backup/restore with ZIP
- File versioning, attachments, audit logging
- Virtual scrolling, keyboard shortcuts, ARIA accessibility

### Gaps

- Focus traps missing in 2 dialogs
- No pagination beyond 5000 files
- No unsaved-changes guard on file switch
- Arrow key navigation doesn't auto-scroll

---

## Recommendations

### Before Release (High priority)

1. **Fix ST-14:** Migrate API key storage to OS keychain

### Post-Release (Medium priority)

2. **Fix FT-64:** Add focus traps to ManufacturingDialog, ProjectListDialog
3. **Fix FT-67a/b:** Implement pagination and unsaved-changes guard
4. **Fix ST-05:** Extract escapeHtml to shared utility
5. **Fix ST-12:** Restrict sql:default capability

### Backlog (Low priority)

6. Address remaining 5 Low-severity findings
7. Add `cargo audit` and `npm audit` to CI pipeline
8. Consider frontend unit test framework (Vitest)

---

## Audit Trail

### Reports Generated

| File | Agent | Content |
|------|-------|---------|
| `claude-functional-report.md` | Claude | 67 functional tests |
| `claude-performance-report.md` | Claude | 15 performance tests |
| `claude-security-report.md` | Claude | 35 security tests |
| `codex-functional-report.md` | Codex | 67 functional tests |
| `codex-performance-report.md` | Codex | 15 performance tests |
| `codex-security-report.md` | Codex | 35 security tests |
| `cross-validation.md` | Combined | Reconciliation of all findings |
| `final-report.md` | Combined | This report |

### Test Plan

- `docs/analysis/20260317_001_release-deep-test.md`

### GitHub Issues

- Issues #102-#114 (13 validated findings)

---

## Sign-Off

| Role | Status | Date |
|------|--------|------|
| Claude Reviewer Agent | Complete — 17 findings | 2026-03-17 |
| Codex Reviewer Agent | Complete — 11 findings | 2026-03-17 |
| Cross-Validation | Complete — 13 validated | 2026-03-17 |
| GitHub Issues | 13 created (#102-#114) | 2026-03-17 |
