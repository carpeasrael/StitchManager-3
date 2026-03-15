# Release Test Report — StitchManager v26.03-rc1

**Date:** 2026-03-14
**Version:** 26.03-rc1 (commit 2798a49)
**Tester:** Automated deep code audit (4 parallel analysis agents)

---

## Executive Summary

Version 26.03-rc1 was subjected to a comprehensive code audit covering all frontend TypeScript (34 files), all backend Rust code (30+ files), all CSS/config files, and cross-cutting integration concerns. The audit identified **28 unique issues** requiring attention before release.

**Verdict: NOT READY FOR RELEASE** — 4 critical issues, 14 major issues must be resolved first.

---

## Build & Test Baseline

| Check | Result |
|-------|--------|
| `npm run build` (TS + Vite) | PASS — 41 modules, 167 kB JS, 44 kB CSS |
| `cargo check` | PASS — compiles clean |
| `cargo test` | PASS — 156/156 tests passed |

---

## Test Case Summary

| Test Suite | Total | Pass | Fail |
|-----------|-------|------|------|
| TC01 — File Management | 10 | 6 | 4 |
| TC02 — Folder Management | 8 | 4 | 4 |
| TC03 — Batch Operations | 8 | 4 | 4 |
| TC04 — Parsers & Conversion | 12 | 6 | 6 |
| TC05 — Search & Filter | 10 | 8 | 2 |
| TC06 — AI Integration | 7 | 6 | 1 |
| TC07 — Settings & Theme | 10 | 8 | 2 |
| TC08 — UI & Accessibility | 11 | 0 | 11 |
| TC09 — File Watcher | 6 | 3 | 3 |
| TC10 — Version History | 5 | 2 | 3 |
| TC11 — State & Events | 7 | 3 | 4 |
| **Total** | **94** | **50** | **44** |

**Pass rate: 53%** (50/94)

---

## Issues by Severity

### CRITICAL (4) — Must fix before release

| # | Issue | Area | Impact |
|---|-------|------|--------|
| 1 | Duplicate `toolbar:delete-folder` handler — user sees double confirmation dialogs | Frontend | UX broken for core folder delete operation |
| 2 | `restore_version` SQL bug — pre-restore snapshots permanently skipped after first restore | Backend | Version safety degrades silently; data loss risk |
| 3 | `watcher_auto_import` holds DB lock during thumbnail I/O — app freezes | Backend | App becomes unresponsive when file watcher detects new files |
| 4 | `batch_rename`/`batch_organize` silent rollback failure — files orphaned | Backend | Data loss: files renamed on disk but DB has no record |

### MAJOR (14) — Should fix before release

| # | Issue | Area | Impact |
|---|-------|------|--------|
| 5 | PES writer: malformed PEC header + missing color change byte | Backend | Written PES files are corrupt and unreadable |
| 6 | DST writer: wrong units (mm vs 0.1mm) for dimensions and stitches | Backend | DST output has 10x incorrect dimensions |
| 7 | Custom fields never saved or loaded in MetadataPanel | Frontend | Feature appears functional but silently discards data |
| 8 | Missing path traversal protection on `parse_embroidery_file` + `batch_export_usb` | Backend | Arbitrary file read / arbitrary write location |
| 9 | WCAG AA contrast failures — dark theme muted text (~2.9:1) and light theme muted-light (~2.2:1) | CSS | Accessibility violation affecting large portions of UI |
| 10 | Dialog overflow at minimum window width (960px) — AI Preview dialog 800px | CSS | Dialog unusable at minimum supported window size |
| 11 | Missing keyboard focus indicators on interactive elements | CSS | Keyboard-only users cannot navigate the application |
| 12 | Hardcoded colors bypass theme system + undefined `--color-error` | CSS | 20+ elements don't adapt between themes; delete hover invisible |
| 13 | No user feedback on batch operation partial failures | Frontend | Users unaware when some files in a batch fail |
| 14 | Dual file-loading race between FileList and main.ts | Frontend | File list may briefly show stale data |
| 15 | Sidebar folder counts not refreshed after scan/batch | Integration | Sidebar shows wrong file counts until manual refresh |
| 16 | No ARIA roles or landmarks in HTML | HTML | Screen reader users see flat div structure |
| 17 | `attach_file` unbounded filename dedup loop | Backend | Potential DoS/infinite loop |
| 18 | `batch_organize` path traversal uses unresolved vs canonical path comparison | Backend | Symlink-based path traversal possible |

### MEDIUM (4)

| # | Issue | Area | Impact |
|---|-------|------|--------|
| 19 | Missing AI event bridge listeners (ai:start/complete/error) | Integration | No real-time progress during AI analysis |
| 20 | Features implemented but not exposed in UI (convert, transfer, versions, edit, info) | Integration | Backend features unreachable from user interface |
| 21 | Escape key propagation in TagInput and ImagePreviewDialog | Frontend | Double-close behavior on Escape |
| 22 | SearchBar outsideClickHandler leaks on panel re-render | Frontend | Accumulated handlers cause unexpected panel closes |

### MINOR (6)

| # | Issue | Area | Impact |
|---|-------|------|--------|
| 23 | AppState shallow copy doesn't protect nested arrays (latent) | Frontend | Future code could corrupt state via reference |
| 24 | Settings background image file orphaned on cancel | Frontend | Disk space waste |
| 25 | icon.ico single 16x16 resolution | Config | Pixelated icon on Windows |
| 26 | No `prefers-reduced-motion` media query | CSS | Animations ignore user preference |
| 27 | MetadataPanel tag dirty detection fails with comma-containing tags | Frontend | False dirty state indication |
| 28 | Sidebar `createFolder` uses native `alert()` instead of toast | Frontend | Inconsistent error display |

---

## Strengths

- **Rust unit tests:** 156 tests all passing, covering parsers, commands, services, DB migrations
- **Frontend build:** Clean TypeScript compilation, no type errors
- **Component lifecycle:** Base class pattern ensures consistent subscription cleanup
- **Virtual scrolling:** Well-implemented with generation tracking, proper boundaries
- **Service layer:** Consistent invoke() wrappers with proper error handling patterns
- **Event bridge:** All critical Tauri events correctly forwarded to EventBus
- **Database:** WAL mode, FK constraints, busy_timeout properly configured
- **HMR cleanup:** Thorough teardown in dev mode

---

## Recommendations

### Before Release (blockers)
1. Fix all 4 CRITICAL issues — these cause data loss, app freezes, or broken core operations
2. Fix MAJOR issues #5-6 (corrupt file writers) — format conversion is unusable
3. Fix MAJOR issue #7 (custom fields) — feature is misleading as-is
4. Fix MAJOR issues #9-12 (accessibility/CSS) — significant WCAG violations

### Before GA (should fix)
5. Fix remaining MAJOR issues (path traversal, race conditions, batch feedback)
6. Address MEDIUM issues (expose implemented features in UI)
7. Add integration tests for file writer round-trips
8. Add path traversal tests for all Tauri commands

### Post-GA (nice to have)
9. Address MINOR issues
10. Add `prefers-reduced-motion` support
11. Multi-resolution Windows icon

---

## Test Case Files

All test cases stored in `./test/`:

| File | Test Cases |
|------|-----------|
| `TC01_file_management.md` | 10 cases |
| `TC02_folder_management.md` | 8 cases |
| `TC03_batch_operations.md` | 8 cases |
| `TC04_parsers_conversion.md` | 12 cases |
| `TC05_search_filter.md` | 10 cases |
| `TC06_ai_integration.md` | 7 cases |
| `TC07_settings_theme.md` | 10 cases |
| `TC08_ui_accessibility.md` | 11 cases |
| `TC09_file_watcher.md` | 6 cases |
| `TC10_version_history.md` | 5 cases |
| `TC11_state_events.md` | 7 cases |

---

## GitHub Issues Created

28 issues filed on GitHub:

### Critical (4)
| # | Issue | Title |
|---|-------|-------|
| 1 | [#46](https://github.com/carpeasrael/StitchManager-3/issues/46) | Duplicate toolbar:delete-folder handler causes double confirmation dialogs |
| 2 | [#47](https://github.com/carpeasrael/StitchManager-3/issues/47) | restore_version SQL bug skips pre-restore snapshots after first restore |
| 3 | [#48](https://github.com/carpeasrael/StitchManager-3/issues/48) | watcher_auto_import holds DB lock during thumbnail I/O — app freezes |
| 4 | [#49](https://github.com/carpeasrael/StitchManager-3/issues/49) | Batch rename/organize silent rollback failure can orphan files |

### High (10)
| # | Issue | Title |
|---|-------|-------|
| 5 | [#50](https://github.com/carpeasrael/StitchManager-3/issues/50) | PES writer produces corrupt files (malformed header + missing color byte) |
| 6 | [#51](https://github.com/carpeasrael/StitchManager-3/issues/51) | DST writer uses wrong units — dimensions and stitches 10x too small |
| 7 | [#52](https://github.com/carpeasrael/StitchManager-3/issues/52) | Custom fields never saved or loaded in MetadataPanel |
| 8 | [#53](https://github.com/carpeasrael/StitchManager-3/issues/53) | Missing path traversal protection on parse_embroidery_file and batch_export_usb |
| 9 | [#54](https://github.com/carpeasrael/StitchManager-3/issues/54) | WCAG AA contrast failures in dark and light themes |
| 10 | [#55](https://github.com/carpeasrael/StitchManager-3/issues/55) | Dialog overflow at minimum window width (960px) |
| 11 | [#56](https://github.com/carpeasrael/StitchManager-3/issues/56) | Missing keyboard focus indicators on interactive elements |
| 12 | [#57](https://github.com/carpeasrael/StitchManager-3/issues/57) | Hardcoded colors bypass theme system + undefined --color-error |
| 13 | [#58](https://github.com/carpeasrael/StitchManager-3/issues/58) | No user feedback on batch operation partial failures |
| 14 | [#65](https://github.com/carpeasrael/StitchManager-3/issues/65) | No ARIA roles or landmarks in HTML |

### Medium (6)
| # | Issue | Title |
|---|-------|-------|
| 15 | [#59](https://github.com/carpeasrael/StitchManager-3/issues/59) | Dual file-loading race condition between FileList and main.ts |
| 16 | [#60](https://github.com/carpeasrael/StitchManager-3/issues/60) | Sidebar folder counts stale after scan and batch operations |
| 17 | [#61](https://github.com/carpeasrael/StitchManager-3/issues/61) | Missing AI event bridge listeners — no real-time progress |
| 18 | [#62](https://github.com/carpeasrael/StitchManager-3/issues/62) | Escape key propagation issues in TagInput and ImagePreviewDialog |
| 19 | [#63](https://github.com/carpeasrael/StitchManager-3/issues/63) | SearchBar outsideClickHandler leaks on panel re-render |
| 20 | [#64](https://github.com/carpeasrael/StitchManager-3/issues/64) | Multiple implemented features not exposed in UI |

### Low (8)
| # | Issue | Title |
|---|-------|-------|
| 21 | [#66](https://github.com/carpeasrael/StitchManager-3/issues/66) | attach_file unbounded filename deduplication loop |
| 22 | [#67](https://github.com/carpeasrael/StitchManager-3/issues/67) | Settings background image file orphaned on cancel |
| 23 | [#68](https://github.com/carpeasrael/StitchManager-3/issues/68) | icon.ico contains only 16x16 — pixelated on Windows |
| 24 | [#69](https://github.com/carpeasrael/StitchManager-3/issues/69) | No prefers-reduced-motion media query |
| 25 | [#70](https://github.com/carpeasrael/StitchManager-3/issues/70) | MetadataPanel tag dirty detection fails with comma-containing tags |
| 26 | [#71](https://github.com/carpeasrael/StitchManager-3/issues/71) | Sidebar createFolder uses native alert() instead of toast |
| 27 | [#72](https://github.com/carpeasrael/StitchManager-3/issues/72) | convert_file_inner TOCTOU gap between snapshot and conversion |
| 28 | [#73](https://github.com/carpeasrael/StitchManager-3/issues/73) | Scan directory shows no error toast on failure |
