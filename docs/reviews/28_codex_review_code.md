# Code Review — Issues #43, #44 — Round 2
Reviewer: Codex (Sonnet)
Date: 2026-03-13

## Verification of Round 1 Fixes

### Finding 8 — overflow:hidden clipping burger menu (HIGH)
**Status: FIXED**
`components.css` line 1108 now sets `position: fixed` on `.burger-menu` instead of `position: absolute`. A fixed-positioned element is not clipped by any ancestor's `overflow: hidden`, so the dropdown is no longer cut off by `.app-layout`.

### Finding 9 — z-index collision between dialog overlay and burger menu (MEDIUM)
**Status: FIXED**
`components.css` line 1116 now sets `z-index: 90` on `.burger-menu`, down from `z-index: 100`. The dialog overlay retains `z-index: 100`, so dialogs will always stack above the burger menu. The stacking order is now unambiguous.

### Finding 2 — Missing role="menu" on panel (LOW / Accessibility)
**Status: FIXED**
`Toolbar.ts` line 182 now calls `this.panel.setAttribute("role", "menu")` on the panel element before appending children. Each child button already has `role="menuitem"` (line 202). The ARIA ownership contract is satisfied.

## rAF guard verification (Finding 3 follow-up)
The `requestAnimationFrame` guard on the outside-click handler (Toolbar.ts lines 234–242) remains in place with the added early-return `if (!this.menuOpen) return` guard at line 235. This guard was present in Round 1 as well. The rAF pattern is functionally correct for this use case; the `setTimeout(0)` alternative noted in Finding 3 was flagged as a minor robustness concern and was not required to be changed. No regression here.

## batch.rs fallback value (Finding context)
`batch.rs` lines 277–283: querying `library_root` uses `.map_err(|_| AppError::Validation(...))` — an error is returned to the caller, not a silent fallback. Confirmed no spurious default value is introduced.

## New Findings

No new findings introduced by the Round 2 changes.

The three fixed items (Finding 2, 8, 9) are verified correct. The remaining Round 1 findings (1, 3–7, 10–13) were not part of the Round 2 fix scope and remain open from the prior cycle.

Code review passed for Round 2 scope. No new findings.
