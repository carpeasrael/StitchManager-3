# Wave 2 Design Consistency Review (regression check) — 2026-04-19

## Summary
Pass. No new design regressions introduced by Wave 2.

The frontend diff is limited to `src/state/AppState.ts`, `src/components/FileList.ts`, and `src/components/Sidebar.ts`, all internal logic changes:

- `AppState`: `get()` now returns a live reference (was deep-copy); added explicit `clone()` for the rare detach case. No visual surface.
- `FileList`: routes `files`-state changes through `onFilesChanged()` to incrementally extend the spacer on pagination-appends. Same DOM, same card markup, same CARD_HEIGHT (72), same virtual-scroll behavior.
- `Sidebar`: introduces row-element index Maps and `updateSelectionClasses()` that toggles the existing `.selected` class instead of full re-render. Same markup, same `.selected` class already styled by `components.css`, same context-menu/drag wiring on structural renders.

No new CSS, dialogs, buttons, icons, colors, spacing, typography, or German text strings were added. The selection class name (`selected`) matches existing styling. Wave 4 remains the appropriate place to address the 26 pre-existing design-consistency findings.

## Findings (regressions introduced by this diff)
No new findings.
