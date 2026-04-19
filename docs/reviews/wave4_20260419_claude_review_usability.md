# Wave 4 Usability Review (regression check) — 2026-04-19

## Summary
Pass. The Wave 4 token-only diff introduces no usability regressions. The new
`.btn`/`.btn-primary`/`.btn-secondary` rules give the four previously-bare
dialogs (FolderDialog, FolderMoveDialog, SmartFolderDialog,
ImportPreviewDialog) consistent affordances aligned with the canonical
`.dialog-btn` family. Primary (filled accent) vs secondary (outlined surface)
is clearly distinguishable in both themes. `.btn-small` is declared after
`.btn`, so its compact padding still wins via source order. The new `.btn`
inherits the global `button:focus-visible` outline. The tokenised
`.folder-type-sewing_pattern` purple (#7c3aed light / #c4b5fd dark) remains
visually distinct from the blue accent and neutral grey badges. Star rating
glyphs (#e6a700 / #ffc107) sit on the surface background, not the muted
text colour — contrast is acceptable in both themes. Z-index ordering is now
correct: toasts (1000) above all dialogs (100/110/120) and above the context
menu (200).

## New findings (regressions introduced by this diff)
No new findings.
