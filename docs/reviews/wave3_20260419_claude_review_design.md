# Wave 3 Design Consistency Review (regression check) — 2026-04-19

## Summary
PASS — no new design regressions introduced by Wave 3.

The new `ConfirmDialog` and `InputDialog` reuse the existing Aurora dialog
scaffolding (`.dialog-overlay`, `.dialog`, `.dialog-header`, `.dialog-body`,
`.dialog-footer`, `.dialog-btn-primary/secondary/danger`) with no ad-hoc
markup. The new CSS classes added to `src/styles/components.css`
(`.dialog-message`, `.dialog-hint`, `.dialog-label`, `.dialog-input` and its
`:focus` state, plus `.toast-close` and its hover/focus-visible states) all
reference Aurora design tokens: `--color-text`, `--color-muted`, `--color-bg`,
`--color-border`, `--color-accent`, `--color-accent-10`, `--font-size-body`,
`--font-size-caption`, `--font-size-label`, `--font-family`, `--spacing-2`,
`--spacing-3`, `--radius-input`. All tokens exist in `aurora.css` for both
light and dark themes. `InputDialog` references `.dialog-error`, which is a
pre-existing class. The few hard-coded pixel values in `.toast-close`
(24/24/18 px) match patterns already present in the file.

## New findings (regressions introduced by this diff)
No new findings.
