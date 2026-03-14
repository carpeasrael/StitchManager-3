# Sprint Plan — StitchManager v26.03-rc1 Release Fixes

**Created:** 2026-03-14
**Base commit:** 2798a49 (v26.03-rc1)
**Source:** 20 open GitHub issues from release testing audit
**Pass rate at start:** 53% (50/94 test cases)

---

## Overview

| Sprint | Focus | Issues | Severity | Est. Effort |
|--------|-------|--------|----------|-------------|
| Sprint 12 | Critical: Data Safety & Integrity | #46, #47, #48, #49 | 4x critical | High |
| Sprint 13 | File Writers & Security | #50, #51, #53 | 3x high | High |
| Sprint 14 | UI State, Feedback & Data Persistence | #52, #58, #59, #60 | 3x high, 1x medium | Medium |
| Sprint 15 | Accessibility & Theming (WCAG AA) | #54, #55, #56, #57 | 4x high | Medium |
| Sprint 16 | Event System, UI Polish & Missing Menus | #61, #62, #63, #64, #66 | 5x medium | Medium |
| Sprint 17 | Low-Priority Fixes & Hardening | #67, #68, #69, #70, #71, #72, #73 | 7x low | Low |

**Total:** 20 issues across 6 sprints

---

## Sprint 12 — Critical: Data Safety & Integrity

**Goal:** Eliminate all data-loss and corruption risks.
**Priority:** MUST complete before any other sprint.

| # | Issue | Component | Problem |
|---|-------|-----------|---------|
| #46 | Duplicate delete-folder handler | `src/main.ts` | Two `toolbar:delete-folder` handlers cause double confirmation dialogs; merge artifact |
| #47 | restore_version SQL bug | `src-tauri/src/commands/versions.rs` | COUNT query is a no-op with ORDER/LIMIT; pre-restore snapshots disabled after first restore |
| #48 | DB lock held during thumbnail I/O | `src-tauri/src/commands/scanner.rs` | `watcher_auto_import` and `import_files` hold DB lock during thumbnail generation, freezing the app |
| #49 | Silent rollback failure orphans files | `src-tauri/src/commands/batch.rs` | Failed filesystem rollback silently ignored; files become orphaned from DB |

**Acceptance criteria:**
- [ ] #46: Single delete-folder handler, one confirmation dialog
- [ ] #47: Pre-restore snapshot created on every restore, not just the first
- [ ] #48: DB lock released before thumbnail generation in all import paths
- [ ] #49: Rollback failures logged, reported to user, and recoverable
- [ ] `cargo test` passes
- [ ] `npm run build` passes

**Test cases affected:** TC02-04, TC10-03, TC09-02, TC03-02

---

## Sprint 13 — File Writers & Security

**Goal:** Fix format conversion output correctness and close path traversal vulnerabilities.

| # | Issue | Component | Problem |
|---|-------|-----------|---------|
| #50 | PES writer produces corrupt files | `src-tauri/src/parsers/writers.rs` | Malformed PEC header + missing 3rd byte in color change sequence |
| #51 | DST writer wrong units | `src-tauri/src/parsers/writers.rs` | Header dimensions and stitch displacements 10x too small (mm vs 0.1mm) |
| #53 | Missing path traversal protection | `scanner.rs`, `batch.rs` | `parse_embroidery_file` has no path validation; `batch_export_usb` accepts arbitrary paths; `batch_organize` uses non-canonical path comparison |

**Acceptance criteria:**
- [ ] #50: PES round-trip test (write then read back) produces identical stitch data
- [ ] #51: DST header dimensions match source file; stitches render at correct scale
- [ ] #53: Path traversal attempts rejected with error in all three endpoints
- [ ] `cargo test` passes (add new tests for writers and path validation)
- [ ] `npm run build` passes

**Test cases affected:** TC04-05, TC04-06, TC04-07, TC04-08, TC04-11, TC03-04, TC03-06

---

## Sprint 14 — UI State, Feedback & Data Persistence

**Goal:** Fix data flows — custom fields actually persist, batch errors are surfaced, file list is consistent, sidebar counts stay current.

| # | Issue | Component | Problem |
|---|-------|-----------|---------|
| #52 | Custom fields never saved/loaded | `src/components/MetadataPanel.ts` | Custom field inputs rendered but values never read on save, never populated on load |
| #58 | No feedback on batch partial failures | `src/main.ts` | Batch handlers catch errors with `console.warn` only; no toast with success/failure counts |
| #59 | Dual file-loading race condition | `FileList.ts`, `main.ts` | Two independent file-fetch paths with no shared generation tracking; stale data possible |
| #60 | Sidebar counts stale after scan | `main.ts`, `Toolbar.ts`, `Sidebar.ts` | Folder counts not refreshed after scan/import/batch operations |

**Acceptance criteria:**
- [ ] #52: Custom field values persist across save/reload cycle
- [ ] #58: Toast shown after batch operations with success/failure counts
- [ ] #59: Single file-loading path or shared generation guard prevents stale data
- [ ] #60: Sidebar counts update after scan, import, and batch operations
- [ ] `npm run build` passes

**Test cases affected:** TC01-07, TC01-08, TC03-03, TC11-02, TC02-06, TC02-07

---

## Sprint 15 — Accessibility & Theming (WCAG AA)

**Goal:** Achieve WCAG AA compliance for contrast, focus indicators, color system, and responsive dialogs.

| # | Issue | Component | Problem |
|---|-------|-----------|---------|
| #54 | WCAG AA contrast failures | `src/styles/aurora.css` | `--color-muted` and `--color-muted-light` fail 4.5:1 contrast ratio in both themes |
| #55 | Dialog overflow at min window size | `src/styles/components.css` | AI Preview (800px), text popup (700px), AI result (640px) exceed 960px viewport |
| #56 | Missing keyboard focus indicators | `src/styles/components.css` | Only `.folder-delete-btn` has `:focus-visible`; all other interactive elements have no focus ring |
| #57 | Hardcoded colors + undefined --color-error | `aurora.css`, `components.css` | 20+ hardcoded hex colors don't adapt to themes; `--color-error` never defined |

**Acceptance criteria:**
- [ ] #54: All text/background combinations meet 4.5:1 contrast ratio
- [ ] #55: All dialogs respect `max-width: 90vw; max-height: 85vh` constraints
- [ ] #56: All interactive elements show visible focus ring on keyboard navigation
- [ ] #57: Zero hardcoded colors in components.css; `--color-error/success/warning` defined in both themes
- [ ] `npm run build` passes

**Test cases affected:** TC08-01 through TC08-06, TC07-10

---

## Sprint 16 — Event System, UI Polish & Missing Menus

**Goal:** Wire up missing event bridges, fix event propagation, plug memory leaks, and expose implemented features in the UI.

| # | Issue | Component | Problem |
|---|-------|-----------|---------|
| #61 | Missing AI event bridge listeners | `src/main.ts` | `ai:start/complete/error` events not bridged from Tauri to EventBus; no progress indicator |
| #62 | Escape key propagation issues | `TagInput.ts`, `ImagePreviewDialog.ts` | Escape not stopped; propagates to global handler, triggering unintended actions |
| #63 | SearchBar outsideClickHandler leak | `src/components/SearchBar.ts` | `renderPanel()` registers new document click listener without removing previous one |
| #64 | Implemented features not in UI | `src/components/Toolbar.ts` | Convert, Transfer, Edit/Transform, Version History, Info — all implemented but no menu items |
| #66 | attach_file unbounded dedup loop | `src-tauri/src/commands/files.rs` | Filename deduplication counter has no upper bound; potential infinite loop |

**Acceptance criteria:**
- [ ] #61: AI analysis shows spinner/progress; events bridged to EventBus
- [ ] #62: Escape in TagInput/ImagePreviewDialog stops propagation
- [ ] #63: Panel re-render removes previous outsideClickHandler before registering new one
- [ ] #64: Burger menu includes entries for Convert, Transfer, Edit/Transform, Version History
- [ ] #66: Dedup loop capped at 100,000 with error on exhaustion
- [ ] `cargo test` passes
- [ ] `npm run build` passes

**Test cases affected:** TC06-04, TC08-08, TC08-09, TC05-08, TC11-07

---

## Sprint 17 — Low-Priority Fixes & Hardening

**Goal:** Clean up remaining low-severity issues for release polish.

| # | Issue | Component | Problem |
|---|-------|-----------|---------|
| #67 | Background image orphaned on cancel | `SettingsDialog.ts` | File copied immediately on selection; not cleaned up if dialog cancelled |
| #68 | icon.ico only 16x16 | `src-tauri/icons/icon.ico` | Windows requires multi-resolution ICO (16, 32, 48, 256) |
| #69 | No prefers-reduced-motion query | CSS | Animations/transitions ignore user motion preference |
| #70 | Tag dirty detection comma bug | `MetadataPanel.ts` | `tags.join(",")` comparison ambiguous with comma-containing tags |
| #71 | Sidebar createFolder uses alert() | `Sidebar.ts` | `alert()` blocks UI; inconsistent with toast system |
| #72 | convert_file_inner TOCTOU gap | `commands/convert.rs` | DB lock acquired twice with gap; file could change between acquisitions |
| #73 | Scan directory no error toast | `Toolbar.ts` | Scan errors caught but no toast shown to user |

**Acceptance criteria:**
- [ ] #67: Background image copy deferred to save, or cleaned up on cancel
- [ ] #68: icon.ico contains 16x16, 32x32, 48x48, 256x256 resolutions
- [ ] #69: `@media (prefers-reduced-motion: reduce)` disables animations
- [ ] #70: Tag comparison uses element-by-element or null-separator join
- [ ] #71: `alert()` replaced with `ToastContainer.show()`
- [ ] #72: Single lock acquisition for version snapshot + filepath read
- [ ] #73: Error toast shown on scan failure
- [ ] `cargo test` passes
- [ ] `npm run build` passes

**Test cases affected:** TC07-10, TC08-07, TC05-08

---

## Dependency Graph

```
Sprint 12 (Critical)
    │
    ├── Sprint 13 (Writers & Security) ── independent, but do after critical
    │
    ├── Sprint 14 (State & Feedback)
    │       │
    │       └── Sprint 16 (Events & UI) ── #64 depends on #52 custom fields working
    │
    ├── Sprint 15 (Accessibility)
    │       │
    │       └── Sprint 17 (Low Priority) ── #69 reduced-motion builds on #57 theme tokens
    │
    └── Sprint 17 (Low Priority) ── can start after Sprint 12
```

**Parallelizable:** Sprints 13, 14, and 15 can run concurrently after Sprint 12 completes.

---

## Release Gate

Release v26.03 is blocked until:
- All 20 issues closed
- `cargo test` — all tests pass
- `npm run build` — clean build
- Pass rate >= 90% on release test suite (currently 53%)
- Re-run full audit with zero critical/high findings
