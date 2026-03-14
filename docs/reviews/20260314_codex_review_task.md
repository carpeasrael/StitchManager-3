# Codex Task-Resolution Review

**Date:** 2026-03-14
**Task:** Ensure that all menus are in front when open

## Verification

The diff was reviewed against the analysis in `docs/analysis/20260314_41_menu_z_index.md`.

### Changes verified

1. `.app-layout::before` pseudo-element now has `z-index: -1`, pushing it behind all content without requiring layout panels to establish stacking contexts.
2. `z-index: 1` removed from all layout panels (`.app-menu`, `.app-sidebar`, `.app-center`, `.app-right`, `.app-splitter-l`, `.app-splitter-r`, `.app-status`). They retain `position: relative` but no longer create stacking contexts.
3. The burger menu (`z-index: 90`) and context menu (`z-index: 90`) now participate in the root stacking context and will render above all layout panels regardless of DOM order.

### Findings

Task resolved. No findings.

## Verdict

**PASS**
