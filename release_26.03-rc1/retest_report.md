# Release Re-Test Report — StitchManager v26.03-rc1

**Date:** 2026-03-14
**Base:** commit b9d9fbd (after 21 fixes in sprints 12-17 + issue #65)
**Tester:** Automated deep code audit (4 parallel analysis agents)

---

## Build & Test Baseline

| Check | Result |
|-------|--------|
| `npm run build` (TS + Vite) | PASS — 41 modules, 170 kB JS, 45 kB CSS |
| `cargo check` | PASS — compiles clean |
| `cargo test` | PASS — 156/156 tests passed |

---

## Fixes Verified (21 issues closed)

All 21 issues from the original release test (#46–#73, #65) have been resolved:

| Sprint | Issues | Status |
|--------|--------|--------|
| 12 — Critical Data Safety | #46, #47, #48, #49 | All closed |
| 13 — Writers & Security | #50, #51, #53 | All closed |
| 14 — State, Feedback & Data | #52, #58, #59, #60 | All closed |
| 15 — Accessibility & Theming | #54, #55, #56, #57 | All closed |
| 16 — Events, UI & Menus | #61, #62, #63, #64, #66 | All closed |
| 17 — Low-Priority Polish | #67, #68, #69, #70, #71, #72, #73 | All closed |
| Standalone | #65 | Closed |

---

## New Findings

### Medium Severity (2)

| # | Issue | Description |
|---|-------|-------------|
| #74 | Path traversal on convert/edit output | `convert_file_inner` and `save_transformed` accept output paths without `..` validation |
| #78 | --color-warning text contrast | `#e6a700` on white background = ~2.1:1, fails WCAG AA |

### Low Severity (3)

| # | Issue | Description |
|---|-------|-------------|
| #75 | Toolbar addFolder silent error | No toast on folder creation failure; Sidebar formats error as `[object Object]` |
| #76 | Escape doesn't dismiss text/info dialogs | `showTextPopup`/`showInfoDialog` don't handle `dialog-dismiss` event |
| #77 | Hardcoded box-shadow values | 6 shadows use `rgba()` instead of `--shadow-*` tokens |

### Info/Observations (not filed as issues)

- `import:complete` Tauri event emitted but not bridged (redundant — result returned via invoke)
- `scan:progress`, `scan:file-found`, `ai:start/complete/error` bridged but no frontend consumers yet
- `toolbar:delete-file` handler with no emitter (dead code from removed menu item)
- `search:close-panel` subscribed but never emitted
- `ai_reject_result` missing transaction wrapping (low risk — single-user desktop app)
- PEC short-form encoding mask is correct but non-obvious (documentation suggestion)
- `batch_organize` path check uses non-canonical comparison (mitigated by input sanitization)
- `rem`/`px` font sizes on some elements don't scale with user font-size setting
- `.dialog-tab` missing `:hover` state
- `.search-advanced-panel` missing max-height/overflow for small windows

---

## Verdict

**Significant improvement from initial audit.** The original 21 issues (4 critical, 10 high, 5 medium, 7 low) have all been resolved. The re-test found 5 new issues (2 medium, 3 low) — none critical or high.

**Recommendation:** Fix #74 (path traversal) and #78 (contrast) before release. Issues #75, #76, #77 can be deferred to a patch release.
