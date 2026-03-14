# Sprint 15 Analysis — Accessibility & Theming (WCAG AA)

**Date:** 2026-03-14
**Issues:** #54, #55, #56, #57
**Severity:** All high

---

## Issue #54 — WCAG AA contrast failures

### Problem
`--color-muted` (#5c5e63) on `--color-surface` (#1f1f23) = ~2.9:1 in dark theme. Light theme `--color-muted-light` (#b4b7bd) on white = ~2.2:1. Both fail WCAG AA 4.5:1.

### Approach
Increase lightness of muted colors in dark theme, decrease in light theme to meet 4.5:1.

## Issue #55 — Dialog overflow at minimum window width

### Problem
`.dialog-ai-preview` (800px), `.dialog-text-popup` (700px), `.dialog-ai-result` (640px) lack `max-width: 90vw`.

### Approach
Add `max-width: 90vw; max-height: 85vh; overflow: auto` to all fixed-width dialogs.

## Issue #56 — Missing keyboard focus indicators

### Problem
Only `.folder-delete-btn` has `:focus-visible`. 8 inputs suppress outline without adequate replacement.

### Approach
Add global `:focus-visible` rule for interactive elements. Keep `outline: none` for `:focus:not(:focus-visible)`.

## Issue #57 — Hardcoded colors + undefined --color-error

### Problem
20+ hardcoded hex colors (#28a745, #dc3545, #ffc107, #333, #fff). `--color-error` used but never defined.

### Approach
Define `--color-error`, `--color-success`, `--color-warning` in both themes. Replace all hardcoded colors.
