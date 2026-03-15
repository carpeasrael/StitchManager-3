# Release 26.03-a2 — Implementation Plan

**Date:** 2026-03-13
**Open Issues:** 20 (#23–#42)

## Issue Inventory

| # | Title | Type | Priority | Sprint | Effort |
|---|-------|------|----------|--------|--------|
| 38 | Canvas event listener leak in MetadataPanel | Bug | Critical | 1 | S |
| 23 | Dead code cleanup | Cleanup | High | 1 | S |
| 42 | Improve error visibility | Enhancement | High | 1 | M |
| 41 | Accessibility: ARIA labels & focus management | Enhancement | High | 1 | M |
| 36 | DB query optimizations | Performance | High | 2 | M |
| 37 | Mutex lock contention in batch ops | Performance | High | 2 | M |
| 39 | Optimize virtual scroll & render cycles | Performance | High | 2 | L |
| 35 | Extract shared file import helpers | Refactor | Medium | 2 | M |
| 40 | Extract shared TagInput component | Refactor | Medium | 3 | M |
| 25 | AI prompt enhancement | Enhancement | Medium | 3 | S |
| 26 | Resizable settings popup | Enhancement | Medium | 3 | S |
| 31 | Picture popup with zoom | Feature | Medium | 3 | M |
| 33 | Unique ID + QR code | Feature | Medium | 4 | L |
| 32 | PDF report generation | Feature | Medium | 4 | L |
| 24 | License document attachments | Feature | Medium | 4 | M |
| 27 | USB device detection in status bar | Feature | Medium | 5 | M |
| 34 | Custom background image | Feature | Low | 5 | S |
| 30 | Thread color code mapping | Feature | Low | 5 | L |
| 28 | 50k+ file performance | Performance | Low | 6 | XL |
| 29 | Additional requirements (wishlist) | Epic | Backlog | — | XL |

## Sprint Structure

| Sprint | Focus | Issues | Goal |
|--------|-------|--------|------|
| 1 | Bugs & Foundations | #38, #23, #42, #41 | Fix critical bug, clean dead code, improve error UX and accessibility |
| 2 | Performance & Backend | #36, #37, #39, #35 | Optimize DB, batch ops, rendering, and reduce code duplication |
| 3 | UX Enhancements | #40, #25, #26, #31 | Refactor TagInput, improve AI prompts, popup UX, image zoom |
| 4 | New Features (Core) | #33, #32, #24 | Unique IDs, PDF reports, file attachments |
| 5 | New Features (Extended) | #27, #34, #30 | USB detection, custom backgrounds, thread color mapping |
| 6 | Scale | #28 | 50k+ file performance optimization |
| Backlog | Future | #29 | Large feature wishlist (file conversion, editing, collaboration) |

## Priority Rationale

- **Critical:** #38 is a memory leak bug — must be fixed first
- **High:** #23 (dead code), #42 (error visibility), #41 (accessibility) are foundational quality improvements
- **High:** #36, #37, #39, #35 are performance/refactoring that unblock future features
- **Medium:** UX and core feature issues (#40, #25, #26, #31, #33, #32, #24)
- **Low:** Extended features (#27, #34, #30) and scale (#28)
- **Backlog:** #29 is an epic-level wishlist requiring decomposition into separate issues

## Effort Legend

| Size | Estimate |
|------|----------|
| S | < 2 hours |
| M | 2–6 hours |
| L | 6–16 hours |
| XL | 16+ hours |

## Dependencies

```
#38 (canvas leak) ← no deps, fix first
#23 (dead code)   ← no deps
#42 (error visibility) ← no deps
#41 (a11y)        ← no deps
#36 (DB optimize) ← no deps
#37 (batch mutex) ← no deps
#39 (render perf) ← no deps
#35 (import helpers) ← no deps
#40 (TagInput)    ← no deps
#25 (AI prompt)   ← no deps
#26 (popup resize) ← no deps
#31 (picture zoom) ← no deps
#33 (unique ID)   ← no deps
#32 (PDF report)  ← depends on #33 (ID + QR code needed in PDF)
#24 (attachments) ← no deps
#27 (USB detect)  ← no deps
#34 (background)  ← no deps
#30 (thread colors) ← no deps
#28 (50k+ perf)   ← benefits from #36, #37, #39
#29 (wishlist)    ← requires decomposition, depends on many others
```
