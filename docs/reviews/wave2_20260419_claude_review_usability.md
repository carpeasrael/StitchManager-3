# Wave 2 Usability Review (regression check) — 2026-04-19

## Summary
Pass. The Wave 2 perf changes do not introduce any new usability regressions
on user-visible paths. The Sidebar selection refactor correctly applies
`.selected` via class-toggle and removes the prior full-DOM rebuild flicker
(net usability win). `FileList` incremental append keeps `lastRenderedCount`
in sync in both `render()` and the `onFilesChanged` append branch, and
preserves the cached thumbnails when scrolling past the load-more boundary.
`AppState.get()` returning a live ref does not expose any latent UX bug:
all 4 `appState.update()` callers and the inspected `get()` consumers build
fresh values rather than mutating in place. `FILE_SELECT_LIST_ALIASED`
masks `description`/`keywords`/`comments`/`purchase_link`/`instructions_html`
to empty strings, but no list-view consumer (FileList card, badges,
tooltips, search highlighting) reads these fields — they are only consumed
by `MetadataPanel`, which fetches via the full `FILE_SELECT` (`get_file`).
No new user-visible error strings introduced (only log::warn).

## Findings (regressions introduced by this diff)
No new findings.
