# Wave 4 Design Consistency Review — 2026-04-19

## Summary
**Pass.** Wave 4 closes the two Critical findings and the High findings it set out
to address. The new Aurora tokens (`--color-text-muted`, `--color-bg-hover`,
`--color-danger[-bg]`, `--color-secondary[-bg|-text]`, `--color-accent-rgb`,
`--color-canvas`, `--color-scrim-light/strong`, `--color-overlay-medium`,
`--radius-sm`, `--font-family-mono`, `--font-size-micro/badge`,
`--btn-size-sm/md`) and the new z-index scale
(`--z-base/popover/overlay/dialog/dialog-fullscreen/context-menu/toast`)
are wired into both the light and dark themes (`src/styles/aurora.css:33-44`,
`66`, `75-88`, `129-141`). The previously orphan `.btn`/`.btn-primary`/
`.btn-secondary` rules are now defined (`src/styles/components.css:632-687`),
so FolderDialog, FolderMoveDialog, SmartFolderDialog, ImportPreviewDialog
no longer render as bare browser-default buttons. `.dialog-close-btn` is
aliased to `.dialog-close` (`2283-2297`), six magic z-indexes are now token
references, the toast container correctly sits above the folder context
menu, and the hard-coded Tailwind palette in folder-type/manufacturing/star
badges has been replaced with theme-aware tokens. Build (`npm run build`)
succeeds. Two minor leftovers and one comment/CSS mismatch noted below — none
block the wave.

## Verification of original 26 findings

### [Critical] #1 Undefined CSS variables — **addressed**
- `--color-text-muted` defined at `aurora.css:33,130` (alias of `--color-muted`).
- `--color-bg-hover` defined at `aurora.css:34,131` (alias of `--color-accent-10`).
- `--color-danger`, `--color-danger-bg` defined at `aurora.css:36-37,133-134`.
- `--color-accent-rgb` defined at `aurora.css:41,138` (theme-aware: 10,132,255 → 45,127,249).
- `--radius-sm` defined at `aurora.css:66` (4 px).
- The `.mfg-kalk-total` rule (`components.css:4604`) still uses the literal-fallback pattern
  `var(--color-accent-bg, rgba(var(--color-accent-rgb, 99, 102, 241), 0.08))`. `--color-accent-bg`
  is **still undefined**, so the fallback expression resolves — but because
  `--color-accent-rgb` is now defined, the inner indigo fallback `99, 102, 241` is no longer
  reached and the result is a 0.08-alpha tint of the brand accent (correct visual). Acceptable
  as a closure of the visual bug; flagged below as a minor follow-up to drop the dead fallback.

### [Critical] #2 Two parallel button systems, one undefined — **addressed**
- `.btn`, `.btn-primary`, `.btn-secondary` rules added at `components.css:632-673` using Aurora tokens.
- `.btn-small` refactored to use `--color-bg-hover` (`components.css:686`).
- Cascade verified: `class="btn btn-small"` (used in `ImportPreviewDialog.ts:89,100,218` and
  `SmartFolderDialog.ts:177`) inherits `display/align-items/gap/font-family/font-weight/cursor/
  white-space` from `.btn` and overrides `font-size`/`padding`/`border`/`background` via `.btn-small`.

### [High] #3 Six different close-button class names — **partially addressed (per wave brief)**
- `.dialog-close-btn` aliased to `.dialog-close` at `components.css:2283-2297`. The four dialogs
  using `dialog-close-btn` (FolderDialog, FolderMoveDialog, SmartFolderDialog,
  ImportPreviewDialog) now render the canonical × style.
- `.dv-close-btn`, `.edit-close-btn`, `.image-preview-close`, `.text-popup-close-x` remain
  separate rules — explicitly **deferred-with-justification** in the wave brief.
- Note: the comment on `2278-2281` reads "Alias the four historical class names" then lists
  five and only aliases one — see "New findings" below.

### [High] #4 19 native `confirm()`/`prompt()` — **deferred (closed in Wave 3)**
- `src/components/ConfirmDialog.ts` and `src/components/InputDialog.ts` exist (Wave 3).
  Acknowledged in the wave brief.

### [High] #5 100+ inline `el.style.*` — **deferred-with-justification**
- Acknowledged in the wave brief as a significant per-call-site refactor.

### [High] #6 Theme parity — **addressed**
- `.folder-type-sewing_pattern` now uses `--color-secondary-bg`/`--color-secondary-text`
  (`components.css:315-318`); the dark-theme override block was deleted.
- `.mfg-badge-warn` now uses `--color-warning-bg`/`--color-warning-text` (`4170-4173`); dark
  override deleted.
- `.mfg-stock-warn` now uses `var(--color-warning)` (`4259`) — flips correctly between themes.
- `.mfg-inv-status.mfg-inv-warn` and `.mfg-inv-status.mfg-inv-low` now token-driven
  (`4426-4434`); dark overrides deleted.
- `.mfg-tt-diff-over` now uses `--color-error-bg`/`--color-error` (`4493-4496`); dark override
  deleted.
- `.star-rating .star.filled` and `.hover-fill` now use `var(--color-warning)` (`4621-4622`).

### [High] #7 Hard-coded `#fff` text and `background:white` canvases — **deferred-with-justification**
- `--color-canvas` and `--color-on-status` are defined and available; CSS rules that still
  hard-code `#fff`/`white` (e.g. `.dv-canvas`, `.image-viewer-*`) were not yet rewritten.
  Acknowledged in the wave brief; PDF canvases legitimately remain white.

### [Medium] z-index magic numbers — **addressed**
- All globally-stacked overlays now use tokens: context menu (`540`), dialog overlay (`2172`),
  popover (`934`, `2034`), drop-zone (`3223`), document/image viewer (`3255`, `3608`),
  print/project/manufacturing fullscreen (`3742`, `3973`, `4097`), toasts (`2825`).
- Toast (`--z-toast: 1000`) is now correctly above context menu (`--z-context-menu: 200`),
  reversing the previous wrong ordering.
- Local stacking literals (`51`, `10`, `2`, `1`) remain — acceptable as local stacking
  contexts do not interact with the global scale.

### [Medium] All other Mediums (input class systems, sort-dir buttons, dialog title h2/h3,
close-icon glyph, font-size off-scale, padding off-scale, Component base for dialogs,
PatternUploadDialog cancel variant, ProjectListDialog dv-btn primaries, status-badge
duplicates, MetadataPanel.showAttachmentTypeSelector) — **deferred** per wave brief.

### [Low] All Lows (opacity-as-color, settings legend 3 px radius, sidebar/right asymmetry,
image-viewer overlay opacities, ad-hoc icon sizes, Consolas font stack, settings-legend 11 px
literal, emoji vs Unicode, AiResultDialog button order, dv-canvas dark-mode, app chrome border
asymmetry) — **deferred** per wave brief. Tokens that enable the future refactor
(`--font-family-mono`, `--font-size-micro/badge`, `--btn-size-sm/md`, `--color-scrim-*`,
`--color-overlay-medium`) are now in place.

## New findings introduced by Wave 4

### [SEV: Low] `.mfg-kalk-total` still references undefined `--color-accent-bg`
- **File:** `src/styles/components.css:4604`
- **Description:** `background: var(--color-accent-bg, rgba(var(--color-accent-rgb, 99, 102, 241), 0.08));`.
  `--color-accent-bg` is never declared. The browser falls back to the inner expression. The inner
  `--color-accent-rgb` fallback (`99, 102, 241`, indigo) is no longer reached because Wave 4 defined
  `--color-accent-rgb` for both themes, so the visual is now correct. But the audit Critical #1
  explicitly said "Do not rely on the literal fallback in `var(name, fallback)` — that bypass is the
  bug." The cleanest fix is to either declare `--color-accent-bg` (as an alias of
  `--color-accent-10`) or rewrite the rule as
  `background: rgba(var(--color-accent-rgb), 0.08);`. Not a regression, but the same anti-pattern
  Wave 4 set out to eliminate.

### [SEV: Low] Stale comment on the close-button alias rule
- **File:** `src/styles/components.css:2277-2282`
- **Description:** The comment reads "Alias the four historical class names (`dialog-close-btn`,
  `dv-close-btn`, `edit-close-btn`, `image-preview-close`, `text-popup-close-x`) to the same visuals
  so every modal's × looks identical." It then lists **five** names and the selector only aliases
  one (`.dialog-close-btn`). The wave brief acknowledges the other four as deferred. Either
  shorten the comment to "Alias `.dialog-close-btn` to the canonical `.dialog-close`. The four
  remaining variants (dv-close-btn, edit-close-btn, image-preview-close, text-popup-close-x) are
  tracked as a future consolidation." or actually add them to the selector list. Documentation
  drift only — no visual impact.

### [SEV: Low] `.mfg-inv-table tr.mfg-inv-low` / `.mfg-inv-warn` row tints retain raw rgba
- **File:** `src/styles/components.css:4404-4410`
- **Description:** The wave removed the dark-theme overrides for these rows
  (`rgba(239, 68, 68, 0.12)` and `rgba(245, 158, 11, 0.12)`) but left the light-theme rules
  `rgba(239, 68, 68, 0.08)` and `rgba(245, 158, 11, 0.08)` intact. They now apply unchanged in
  dark mode too. The hex literals 239/68/68 and 245/158/11 are off-palette (Tailwind red-500 /
  amber-500 — Aurora `--color-error` dark is `#ff6b6b`, `--color-warning` dark is `#ffc107`).
  Visually this is fine because the alpha is only 0.08 so the tint reads as "slightly red" /
  "slightly amber" against the surface, and there is no dark-mode regression because the
  removed override was almost the same color. But to be true to the wave's "no hex literals"
  intent these should be `rgba(var(--color-accent-rgb), …)`-style or use `--color-error-bg` /
  `--color-warning-bg`. Minor.
