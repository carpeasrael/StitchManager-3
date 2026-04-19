# Full-App Design Consistency Review — 2026-04-19

## Summary
The Aurora design system is well-defined in `aurora.css` (clean tokens for color, spacing, radius, typography, shadows) and most CSS in `components.css` does pull from those tokens. However, the codebase shows **significant drift**: multiple parallel button class systems coexist (some undefined in CSS), three different dialog close-button class names are in use, several CSS variables are referenced but never declared (silent fallbacks to hard-coded colors), there are large numbers of inline `element.style.*` assignments in TypeScript that bypass the design system, native `confirm()`/`prompt()` are used 19 times instead of the styled dialog system, and theme parity is broken in several spots through hard-coded hex/white colors. This is a fail with many discrete findings.

## Findings

### [SEV: Critical] Undefined CSS variables produce hard-coded fallback colors that break tokenization and theme parity
- **File:** `src/styles/components.css` lines 3103, 4508, 4509, 4523, 4533, 4538, 4545, 4563, 4567, 4572; `src/components/PatternUploadDialog.ts:298`; `src/components/ProjectListDialog.ts:746,761`
- **Description:** The CSS uses `var(--color-text-muted, …)`, `var(--radius-sm, 4px)`, `var(--color-bg-hover)`, `var(--color-accent-bg, …)`, `var(--color-accent-rgb, …)`, `var(--color-danger)`, `var(--color-danger-bg, #fff0f0)` — none of which are defined in `aurora.css`. The fallbacks (`#fff0f0`, `#c00`, `99,102,241`, etc.) are loaded every time, so design-token uniformity is silently lost. `.star-rating .star`, the entire Rich-Text-Editor (`.rt-btn`, `.rt-editor`), the Pattern Preview rules, the Manufacturing total card (`.mfg-kalk-total`) and parts of the project requirements table all render in colors that have no relationship to the Aurora palette and do not adapt to dark mode.
- **Visual impact:** Stars render in a non-accent yellow even in light mode where it works by accident, but `--color-text-muted` resolves to the empty default (no color) and falls back to `currentColor`; danger highlights show as Bootstrap-era `#fff0f0` / `#c00` regardless of theme; the kalkulation total uses an indigo `99,102,241` accent that does not match the brand blue `#0a84ff`/`#2d7ff9`.
- **Recommendation:** Either add the missing tokens to `aurora.css` (`--color-text-muted` → alias `--color-muted`, `--radius-sm` → `4px`, `--color-bg-hover` → `--color-accent-10`, `--color-danger` → `--color-error`, `--color-danger-bg` → `--color-error-bg`, `--color-accent-rgb` → R,G,B for both themes) **or** rename every reference to use the existing Aurora token. Do not rely on the literal fallback in `var(name, fallback)` — that bypass is the bug.

### [SEV: Critical] Two parallel button class systems; one is completely undefined in CSS
- **File:** `src/components/FolderDialog.ts:91,197,202`; `src/components/FolderMoveDialog.ts:112,117`; `src/components/ImportPreviewDialog.ts:391,396`; `src/components/SmartFolderDialog.ts:177,196,201`
- **Description:** These dialogs use `btn` / `btn-primary` / `btn-secondary` / `btn-small` class names. Only `.btn-small` exists in `components.css` (lines 627, 637). `.btn`, `.btn-primary`, `.btn-secondary` are **never defined**. Other dialogs (Settings, AiPreview, AiResult, Manufacturing, BatchDialog, PatternUpload) correctly use `dialog-btn dialog-btn-primary|secondary|danger`.
- **Visual impact:** "Erstellen", "Abbrechen", "Verschieben", "Importieren", "Durchsuchen…" buttons in four dialogs render as unstyled browser-default buttons (or whatever browser/UA defaults apply) — no Aurora padding, radius, accent fill, or focus ring. Visible visual divergence between FolderDialog (default UA grey) and SettingsDialog (Aurora-styled) when shown side-by-side.
- **Recommendation:** Replace every `btn btn-primary` → `dialog-btn dialog-btn-primary`, `btn btn-secondary` → `dialog-btn dialog-btn-secondary`, and `btn btn-small` → reuse `btn-small` rules but tied to the `dialog-btn` family. Delete the orphan `.btn-small` block once unified, or re-home it under the canonical button family.

### [SEV: High] Three different dialog close-button class names with three different visual styles
- **File:** `src/components/AiPreviewDialog.ts:58`, `AiResultDialog.ts:62`, `SettingsDialog.ts:83` use `.dialog-close`; `src/components/FolderDialog.ts:46`, `FolderMoveDialog.ts:49`, `SmartFolderDialog.ts:41`, `ImportPreviewDialog.ts:68` use `.dialog-close-btn` (**undefined in CSS**); `src/components/DocumentViewer.ts:198`, `ManufacturingDialog.ts:140`, `ProjectListDialog.ts:141`, `PrintPreviewDialog.ts:169`, `PatternUploadDialog.ts:79` use `.dv-close-btn`; `src/components/EditDialog.ts:103` uses bespoke `.edit-close-btn`; `src/components/ImagePreviewDialog.ts:47` uses `.image-preview-close`; `src/main.ts:218` uses `.text-popup-close-x`.
- **Description:** Six different close-button classes for the same UI element. `.dialog-close-btn` has no CSS rule at all (renders as default UA button). Each defined variant has different size, padding, hover treatment, and position.
- **Visual impact:** The "×" close button looks visibly different in every dialog: large unstyled UA button in FolderDialog/SmartFolderDialog/ImportPreviewDialog/FolderMoveDialog; Aurora-text `.dialog-close` in Settings/AI dialogs; rounded `.dv-close-btn` in viewer dialogs; full-width text button in EditDialog; absolutely-positioned circle in ImagePreviewDialog.
- **Recommendation:** Standardise on a single `.dialog-close` (or `.dialog__close`) class with one CSS rule, used in every modal. Remove the unused `.dialog-close-btn`, `.dv-close-btn` (rename), `.edit-close-btn`, `.text-popup-close-x` aliases.

### [SEV: High] Native `confirm()` and `prompt()` used 19 times — bypasses the design system
- **File:** `src/main.ts:298,300,397,418,610,744,805`; `src/components/MetadataPanel.ts:89,1131`; `src/components/ManufacturingDialog.ts:381,506,1050,1180,1322,1537`; `src/components/ProjectListDialog.ts:1098`; `src/components/Sidebar.ts:508`; `src/components/SettingsDialog.ts:804`
- **Description:** Destructive confirmations ("Datei wirklich löschen?", "Material wirklich löschen?", purge trash, restore-all etc.) and text input ("Sammlungsname:", "Zielformat wählen", "Maschine wählen") use the browser's native `window.confirm()` / `window.prompt()` chrome instead of the project's modal dialog system. There is no Aurora styling, no theme adaptation, no focus trap, no German-locale layout, no consistent button order.
- **Visual impact:** A jarring native OS dialog appears mid-flow that visually has nothing in common with the rest of the app — wrong font, wrong colors, wrong button positions, no dark-mode support. On macOS the dialogs look entirely out of place. The "Maschine wählen: …\nNummer eingeben:" prompt is a numbered text list — unusable as a dialog.
- **Recommendation:** Build a reusable `ConfirmDialog` and `InputDialog` component that match the Aurora dialog convention (`.dialog-overlay` + `.dialog` + header/body/footer + focus trap + Esc-to-close). Replace all 19 call sites.

### [SEV: High] Inline `element.style.*` assignments bypass the design system in 100+ places
- **File:** `src/components/MetadataPanel.ts` (~30 occurrences), `src/components/ManufacturingDialog.ts` (~50 occurrences), `src/components/ProjectListDialog.ts` (~25 occurrences), `src/components/SettingsDialog.ts`, `PatternUploadDialog.ts`, `ImportPreviewDialog.ts`, `SmartFolderDialog.ts`, `FileList.ts`, `Sidebar.ts`
- **Description:** Components set padding, margins, gaps, widths, font-sizes, colors, and even structural flexbox via inline `el.style.padding = "var(--spacing-2) var(--spacing-3)"`, `el.style.fontSize = "0.85em"`, `el.style.opacity = "0.7"`, `el.style.marginLeft = "8px"`, `el.style.width = "70px"`, `el.style.color = "var(--color-danger)"` etc. Even when the value references a token, the rule lives in TypeScript instead of CSS — uneditable from the stylesheet, untestable, and bypasses cascade and dark-mode overrides. Many use bare pixel values (`"8px"`, `"4px"`, `"12px"`, `"70px"`, `"80px"`, `"90px"`) that are not on the Aurora spacing scale (`4/8/12/16/20/24/32/48`).
- **Visual impact:** Form layout in Manufacturing/Project dialogs uses arbitrary spacing that doesn't match the rest of the app; "0.85em" font sizes appear in tables and add buttons, breaking the typography scale (`10/11/13/15/20`); ad-hoc opacity (`0.6`, `0.7`, `0.85`) used as a substitute for `--color-muted`.
- **Recommendation:** Move every inline style into a CSS class in `components.css`. Reserve inline `style` only for genuinely dynamic values (transform/translate during drag, virtual-scroll absolute positioning, computed paddingLeft for tree depth — those are legitimate). Anything static (font-size, color, padding constants, width modifiers) belongs in CSS classes.

### [SEV: High] MetadataPanel ad-hoc dialog has no header/footer/focus-trap/Esc handling
- **File:** `src/components/MetadataPanel.ts:1785-1848` (`showAttachmentTypeSelector`)
- **Description:** Builds a custom modal entirely inline: `dialog-overlay` (correct) but inside uses a `.dialog-content` class (does not exist in CSS — silent fallback to no styling), all spacing/colors via `el.style.*`, no header element, no `.dialog-footer`, no `.dialog-btn-primary/secondary` semantic buttons, no `trapFocus()` call, no Escape key handler, no `aria-label`/`role="dialog"` attributes.
- **Visual impact:** Attachment-type chooser appears as an unstyled stack of buttons inside an overlay. Visually incongruous with every other dialog in the app, accessibility-broken, no focus trap, can't be Esc-closed.
- **Recommendation:** Refactor to use the same scaffold as `EditDialog`/`AiPreviewDialog` (`.dialog` class + header + body + footer + `trapFocus`).

### [SEV: High] Theme parity broken: hard-coded purple folder-type badge colors and other palette outliers
- **File:** `src/styles/components.css:314,315,319,320` (`.folder-type-sewing_pattern` purple badge), `4050,4051,4055,4056` (`.mfg-badge-warn`), `4309,4310,4314,4315,4319,4320,4324,4325` (`.mfg-inv-status` warn/low), `4393,4394,4403,4404` (`.mfg-tt-diff-over`), `4524,4525` (`.star-rating .star.filled` `#f5a623`)
- **Description:** Hard-coded hex colors `#f0e6ff`/`#7c3aed` (purple), `#fef3c7`/`#92400e`/`#78350f`/`#fde68a` (amber), `#fee2e2`/`#991b1b`/`#7f1d1d`/`#fca5a5` (red), `#f5a623` (star yellow), `#f59e0b` (warn dot), `rgba(239,68,68,…)` and `rgba(245,158,11,…)` for inventory row tints. These are Tailwind defaults inserted ad-hoc — they don't reference Aurora tokens and don't always have a matching `[data-theme="dunkel"]` override (the star color and the `.mfg-stock-warn` dot have none).
- **Visual impact:** Folder-type badges, inventory warning rows, manufacturing badges and the star rating have a different design language than the rest of the app — Tailwind palette next to Apple-system Aurora palette. The unfilled `.mfg-stock-warn` dot (`#f59e0b`) stays orange in dark mode while everything else flips to a tokenised color, looking visually disconnected.
- **Recommendation:** Add Aurora tokens for "purple/secondary" (e.g. `--color-secondary`, `--color-secondary-bg`) and reuse `--color-warning`, `--color-error`, `--color-success` (and their `-bg`/`-text` variants) for everything else. Remove every hex literal from `components.css`.

### [SEV: High] Hard-coded `#fff` text on accent backgrounds and `background: white` on canvases
- **File:** `src/styles/components.css:1571` (`.metadata-view-btn:hover`), `3268` (`.dv-canvas`), `3507` (`.image-viewer-header`), `3526` (`.image-viewer-close`), `3551` (`.image-viewer-nav`), `3599,3606` (`.image-viewer-controls .dv-btn`), `3749` (`.pp-preview-canvas`), `3835` (`.pp-print-btn`), `4255` (`.mfg-bom-remove:hover`)
- **Description:** Multiple rules use `color: #fff` / `background: white`. Aurora has `--color-on-status: #ffffff` for text-on-accent and that token should be used. The PDF/print canvases hard-code white background, which is correct for printing but looks wrong inside the dark-mode app (a hard white panel in a dark UI). The image-viewer overlay uses `rgba(0,0,0,0.6/0.75/0.9)` and `rgba(255,255,255,…)` with no theme equivalents — same look in light and dark modes.
- **Visual impact:** Image viewer is the only screen that is dark-themed in light mode (acceptable convention for media viewers, but the styling should still be tokenised). PDF preview canvas glows pure-white in dark mode when the surrounding chrome is dark grey — visually harsh.
- **Recommendation:** Replace `#fff` text with `var(--color-on-status)`, replace `background: white` for canvases with a `--color-canvas` token (white in both themes — but explicit), and tokenise the viewer overlay colors (e.g. `--color-overlay-strong: rgba(0,0,0,…)`).

### [SEV: Medium] Two parallel input class systems with near-identical rules
- **File:** `src/styles/components.css:1813` (`.metadata-form-input`), `2519` (`.settings-input`), `3808` (`.pp-setting-input`), `4168` (`.mfg-input`), `1905` (`.tag-input`)
- **Description:** Five different input classes with essentially the same body (`width:100%; padding:1-2 spacing-2/3; border:1px solid border; radius:radius-button|radius-input; background:bg|surface; color:text; font-size:body|caption; font-family:family`) and the same focus rule (`border-color: accent`). They differ only in micro-details (padding step `var(--spacing-2)` vs `var(--spacing-1)`, background `--color-bg` vs `--color-surface`, radius `--radius-button` vs `--radius-input`).
- **Visual impact:** Inputs across panels (Metadata, Settings, PrintPreview, Manufacturing, Tag chips) have subtly different padding heights and different radii — most users won't notice in isolation but it shows when two are visible together (e.g. Manufacturing dialog shows `mfg-input` next to a `dialog-textarea` and a `pp-setting-input` in different sub-panels).
- **Recommendation:** Define one canonical `.aurora-input` (or just `.input`) and use modifiers (`.input--sm`, `.input--surface`) for variants. Replace all five.

### [SEV: Medium] Two parallel sort-direction button styles
- **File:** `src/styles/components.css:753` (`.sort-dir-btn`) and `1637` (`.search-sort-dir-btn`)
- **Description:** Same UI control (a button that flips a sort arrow) implemented twice with nearly identical CSS but slightly different hover (`background: var(--color-accent-10); color: var(--color-accent)` vs `background: var(--color-accent-10); color: var(--color-accent)`).
- **Visual impact:** Sort buttons in `SortControl` and in `SearchBar` look almost identical but not exactly — easy for a designer to spot inconsistency.
- **Recommendation:** Collapse to one rule.

### [SEV: Medium] Inconsistent dialog title element (`h2` vs `h3`) for the same role
- **File:** `src/components/PatternUploadDialog.ts:74` (`h2`), `ManufacturingDialog.ts:134` (`h2`), `ProjectListDialog.ts:102` (`h2`); `EditDialog.ts:34`, `FolderDialog.ts:40`, `SmartFolderDialog.ts:36`, `FolderMoveDialog.ts:44`, `ImportPreviewDialog.ts:63`, `MetadataPanel.ts:1796` (`h3`); `AiPreviewDialog.ts:55-56`, `AiResultDialog.ts:59-60`, `SettingsDialog.ts:80-81` use `<span class="dialog-title">` (no heading element at all)
- **Description:** Three different markup choices for the dialog title: `<h2>`, `<h3>`, and `<span>`. Headings are also assigned different classes (`mfg-title`, `pl-title`, `dialog-title`, `dialog-edit-title`).
- **Visual impact:** Mostly accessibility/document-outline impact, but also screen-reader users hear different heading levels for the same UI affordance. CSS-wise the visual title size is set per-class so it ends up looking inconsistent (e.g. PatternUpload h2 inherits `font-size: 1.5em` from the UA stylesheet whereas MetadataPanel h3 gets only the `dialog-edit-title` rule).
- **Recommendation:** Pick one (preferably `<h2>` since dialogs interrupt the document flow) and one class `.dialog-title`. Remove `mfg-title`, `pl-title`, `dialog-edit-title` aliases.

### [SEV: Medium] Inconsistent close icon character (× vs ✖)
- **File:** Most dialogs use `\u00D7` (`×`); `ManufacturingDialog.ts:720,950` use `\u2716` (`✖`)
- **Description:** Two different Unicode glyphs for the same "remove/close" affordance.
- **Visual impact:** Heavy black ✖ inside the manufacturing tables looks visually heavier than the × used elsewhere — different stroke weight, different vertical centering.
- **Recommendation:** Standardise on one glyph (recommend `\u00D7` since it's used in 28+ places).

### [SEV: Medium] Magic z-index numbers without a token scale
- **File:** `src/styles/components.css` lines 543 (1000), 886 (90), 1065 (51), 1935 (10), 1986 (90), 2124 (100), 2728 (200), 2834 (2), 3102 (100), 3134 (110), 3487 (110), 3555 (1), 3621 (120), 3852 (115), 3976 (115)
- **Description:** Z-indexes scattered between `1` and `1000` with no central scale. `100` is used for both `.dialog-overlay` and `.drop-zone-overlay`. `110` is used for `.document-viewer-overlay` and `.image-viewer-overlay`. `115` for both `.project-list-overlay` and `.mfg-overlay`. `200` for toasts. `1000` for the folder context menu — by far the highest.
- **Visual impact:** Stacking can be off in edge cases. The folder context menu (`z-index:1000`) sits **above** the toast container (`z-index:200`), which is wrong — toasts should always be the top-most layer. The drop-zone overlay (`z-index:100`) sits at the same level as standard dialogs and could occlude or be occluded unpredictably.
- **Recommendation:** Add a z-index token scale to `aurora.css`: `--z-base:0; --z-popover:50; --z-overlay:100; --z-dialog:110; --z-dialog-fullscreen:120; --z-toast:1000;`. Use them consistently.

### [SEV: Medium] Hardcoded font-sizes in CSS bypass the typography scale
- **File:** `src/styles/components.css:34,95,110,125,243,260,299,523,871,1243,1477,1527,1535,1613,1654,1968,2232,2607,2843,2979,2993,3045,3057,3120,3174,3420,3527,3552,3771,4523`
- **Description:** Aurora defines the type scale as `--font-size-display:20`, `--font-size-heading:15`, `--font-size-body:13`, `--font-size-label:13`, `--font-size-caption:11`, `--font-size-section:10`. Many rules use bare `8px`, `9px`, `10px`, `11px`, `12px`, `14px`, `16px`, `18px`, `20px`, `24px`, `32px`, `48px`, `0.65rem`, `0.75rem`, `1.25rem`, `1.4rem`, `1.5rem` instead. Some are icon-button sizes (legitimate), but font-sizes for badges (`8/9/10/11px`), info row code (`11px`), copy buttons (`12px`), attachment delete (`14px`), text-popup pre (`0.75rem`) etc. are off-scale.
- **Visual impact:** Inconsistent label/caption sizes across components. Settings legend (`code` 11px) vs caption tokens (11px) — same outcome but un-tokenised. Star rating uses `1.4rem` (~22px) — a unique size.
- **Recommendation:** Map every hard-coded size to the nearest token, and add new tokens (`--font-size-micro: 9px` or `--font-size-badge: 10px`, `--font-size-emoji: 24px`) only when none of the existing 6 levels fits.

### [SEV: Medium] Hardcoded paddings/margins/gaps that bypass the spacing scale
- **File:** `src/styles/components.css:302,464,629,1012,1093,1209,1322,1468,1488,1780,1878,2051,2420,2461,2605,2636,2645,2670,2677,2693,2944,2969,2971,2977,2986,3007,3008,3014,3015,3405,4112,4249,4528,4530`
- **Description:** Aurora spacing scale is `4/8/12/16/20/24/32/48`. Many rules use ad-hoc `1px 4px`, `1px 6px`, `2px 8px`, `2px 6px`, `2px 4px`, `4px 8px`, `4px`, `6px`, `8px 12px`, `padding: 8px 12px` (settings legend), `margin-bottom: 12px`, `gap: 6px`, `gap: 3px`. Some are intentional 1-2px values for badges (acceptable but should be a `--spacing-half: 2px` token); others (`gap: 6px`, `8px 12px`) are mid-scale values that just don't match.
- **Visual impact:** Subtle mis-alignment of buttons, badges and rows across screens. The Settings legend's `padding: 8px 12px` is `--spacing-2 var(--spacing-3)` already, just not written as such — invisible bug today, breaks if the scale ever shifts.
- **Recommendation:** Rewrite all spacing values to use tokens, add `--spacing-0: 2px` (or `--spacing-half`) for the badge case.

### [SEV: Medium] Components do not extend the `Component` base class — lifecycle inconsistency
- **File:** All dialog files (`EditDialog`, `ImageViewerDialog`, `SettingsDialog`, `AiPreviewDialog`, `FolderDialog`, `PrintPreviewDialog`, `SmartFolderDialog`, `Splitter`, `AiResultDialog`, `FolderMoveDialog`, `PatternUploadDialog`, `ImportPreviewDialog`, `Toast` (`ToastContainer`), `BatchDialog`, `DocumentViewer`, `ManufacturingDialog`, `ProjectListDialog`, `ImagePreviewDialog`)
- **Description:** Only 10 components extend `Component` (Toolbar, TagInput, FilterChips, StatusBar, SortControl, Dashboard, FileList, SearchBar, MetadataPanel, Sidebar). All 18 dialog/popup/overlay components implement their own `show()`/`close()`/`destroy()` lifecycle, their own focus trap acquisition, their own subscription tracking. This is documented in `CLAUDE.md` as the canonical base for "all UI components". The base class also `el.innerHTML = ""` on destroy — dialogs do `overlay.remove()` instead and get away with it because they live outside `el`, so the abstraction doesn't fit dialogs.
- **Visual impact:** Not directly visual, but the design-system implication is real: each dialog re-implements the open/close protocol slightly differently (some use `static instance`, some don't; some re-attach Esc listeners, some rely on `dialog-dismiss` events; some use `releaseFocusTrap`, some don't). This produces inconsistent dismiss behaviour (e.g. `ImportPreviewDialog` does or doesn't react to Esc depending on focus position), inconsistent re-open guards.
- **Recommendation:** Either (a) introduce a `Dialog` base class with `open()`/`close()`/`overlay`/`releaseFocusTrap`/Esc-handler/backdrop-click that every dialog extends, or (b) factor a `createDialog({title, body, footer, onClose})` helper that returns a uniform structure. The current copy-paste of dialog scaffolding is the root cause of every other dialog inconsistency in this report.

### [SEV: Medium] PatternUploadDialog cancel button missing variant class
- **File:** `src/components/PatternUploadDialog.ts:303`
- **Description:** `cancelBtn.className = "dialog-btn"` — no `dialog-btn-secondary` modifier.
- **Visual impact:** Cancel button renders with the default `.dialog-btn` border but no background fill rule, making it look subtly different from cancel buttons in every other dialog (which all use `.dialog-btn-secondary`).
- **Recommendation:** Append `dialog-btn-secondary`.

### [SEV: Medium] ProjectListDialog uses viewer-button (`dv-btn`) class for primary actions
- **File:** `src/components/ProjectListDialog.ts:109,349,395,778,975,1117`
- **Description:** "Neues Projekt", "Abbrechen", "Zurück zur Detailansicht", "Bestellung erstellen", "Setup", "Audit anzeigen" all use `dv-btn` (the document viewer toolbar button). That class is designed for compact icon-style toolbar buttons, not page-level primary/secondary actions.
- **Visual impact:** Primary actions in the project dialog render as small dim toolbar buttons next to a `dialog-overlay`-style content area, looking subordinate when they are the main action. Visual hierarchy is wrong.
- **Recommendation:** Use `dialog-btn dialog-btn-primary` for "Neues Projekt"/"Bestellung erstellen", `dialog-btn dialog-btn-secondary` for "Abbrechen"/"Zurück".

### [SEV: Medium] Status badge classes duplicated (`metadata-project-status` vs `pl-status-badge`)
- **File:** `src/styles/components.css:1612-1629` (metadata-project-status) and `3889-3905` (pl-status-badge); `ProjectListDialog.ts:223` mixes the two: it uses `metadata-project-status` for list items but `pl-status-badge` for the dashboard
- **Description:** Two near-identical badge implementations for project status. The metadata variant has `font-size:10px` and `padding: 1px var(--spacing-1)`; the project-list variant has `font-size: var(--font-size-caption)` (11px) and `padding: 2px var(--spacing-2)`. Status mapping (in_progress/completed) is duplicated.
- **Visual impact:** Same project status renders at slightly different sizes in the project list vs the metadata panel.
- **Recommendation:** Single `.status-badge` class with theme-aware variants (`.status-badge--in-progress`, `.status-badge--completed`, etc.).

### [SEV: Low] Inline `style.opacity = "0.7"` etc. used as a substitute for the muted-color token
- **File:** `src/components/ProjectListDialog.ts:444,676,703,713`; `src/components/ManufacturingDialog.ts:1640,1832`
- **Description:** Hint texts set inline `el.style.opacity = "0.6"` or `"0.7"` and `el.style.fontSize = "0.85em"` instead of `color: var(--color-muted); font-size: var(--font-size-caption)`.
- **Visual impact:** Hint text is rendered with reduced opacity rather than reduced contrast, which violates the design language and produces washed-out text on dark backgrounds.
- **Recommendation:** Use `color: var(--color-muted)` + `font-size: var(--font-size-caption)` via a CSS class.

### [SEV: Low] Border-radius for the settings-legend code uses `3px` (off-scale)
- **File:** `src/styles/components.css:2978`
- **Description:** `border-radius: 3px;` — Aurora has `--radius-input:6px`, `--radius-card:8px`, `--radius-pill:999px`, `--radius-swatch:4px`. `3px` exists nowhere else.
- **Visual impact:** Inline `<code>` corners in settings have a unique radius.
- **Recommendation:** Use `var(--radius-swatch)` (4px) or add `--radius-tag: 3px` if intentional.

### [SEV: Low] Two right-rail panel paddings (`--spacing-3` vs `--spacing-4`)
- **File:** `src/styles/layout.css:67` (sidebar `padding: var(--spacing-3)`) and `83` (right `padding: var(--spacing-4)`)
- **Description:** Left sidebar uses 12 px padding, right metadata panel uses 16 px padding. Center column uses 12 px. Visually the right panel inset deeper than the left.
- **Visual impact:** Asymmetric three-column layout — content in the right panel is nudged 4 px further from the splitter than content in the left panel.
- **Recommendation:** Pick one (recommend `--spacing-3` to match center+sidebar) unless the asymmetry is intentional, in which case document it.

### [SEV: Low] Image-viewer footer/header use raw `rgba(0,0,0,0.6)` instead of an overlay token
- **File:** `src/styles/components.css:3506,3549,3560,3595` (image viewer); `2120` (`.dialog-overlay` `rgba(0,0,0,0.5)`); `2813` (image preview overlay `rgba(0,0,0,0.75)`); `3488` (image viewer overlay `rgba(0,0,0,0.9)`)
- **Description:** Four different overlay opacities (`0.5`, `0.6`, `0.75`, `0.8`, `0.9`). No token. No theme adaptation.
- **Visual impact:** Different "modal darkness" in different dialogs — image preview is 50 % darker than standard dialog.
- **Recommendation:** Define `--color-scrim-light: rgba(0,0,0,0.5)`, `--color-scrim-strong: rgba(0,0,0,0.85)` and use them consistently.

### [SEV: Low] Multiple ad-hoc icon button sizes (28×28 vs 32×32)
- **File:** `src/styles/components.css:400,1962` (28×28 sidebar add / burger), `2835` (32×32 image preview close), `2878` (32×32 image preview button), `834` (32×32 search filter toggle)
- **Description:** Two button-icon sizes used interchangeably with no documented rule on when each applies.
- **Visual impact:** Header tools (burger 28 px, search filter 32 px) are visibly different sizes despite sitting in the same toolbar.
- **Recommendation:** Define `--btn-size-sm: 28px`, `--btn-size-md: 32px` tokens and document their use.

### [SEV: Low] `font-family: "Consolas", "Monaco", "Courier New", monospace` appears only once with no token
- **File:** `src/styles/components.css:259`
- **Description:** Text-popup `<pre>` content uses an inline mono font stack. No `--font-family-mono` token.
- **Visual impact:** Monospace block in About/License popup uses a different font stack than any code/path display elsewhere (settings legend's `<code>` uses the default `--font-family`).
- **Recommendation:** Add `--font-family-mono` to `aurora.css` and reuse for `code`, `pre`, file-path displays.

### [SEV: Low] Duplicate settings-legend `font-size: 11px` literal vs `--font-size-caption` (11px)
- **File:** `src/styles/components.css:2979`
- **Description:** Hard-coded `11px` where `var(--font-size-caption)` would work and would respect any future scale change.
- **Recommendation:** Replace with the token.

### [SEV: Low] Iconography mixes Unicode glyphs and emoji
- **File:** `src/components/Toolbar.ts:62-237` (large emoji icon set: 📁🔍📥💾📍📄🔄📤✂📋✨✏📂🔧⚙ ℹ ⌘ ⛁ 🗑 ✖); other components use abstract Unicode geometric arrows (`\u2194`, `\u2195`, `\u21BA`, `\u2605`, `\u2630`, `\u2192`, `\u2715`, `\u2716`)
- **Description:** No single icon library. Toolbar uses color-emoji glyphs that render with the OS color emoji font; other places use monochrome Unicode that renders in `currentColor`. Mixing these inside the same dialog (e.g. Manufacturing tab labels are plain text; the burger menu uses colored emoji; close buttons use monochrome ×) gives an inconsistent visual style.
- **Visual impact:** Color emoji introduces yellow/blue/red into a monochrome design and reads as noise alongside Aurora's calm palette.
- **Recommendation:** Pick one of (a) a real SVG/icon-font library, (b) all monochrome Unicode glyphs, (c) all color emoji. Recommend (a) or (b).

### [SEV: Low] Cancel-then-primary button order is inconsistent in `AiResultDialog`
- **File:** `src/components/AiResultDialog.ts:174-194`
- **Description:** Footer order is "Ablehnen" (danger) → "Alle akzeptieren" (secondary) → "Akzeptieren" (primary). Other footers use Cancel (secondary) → Save (primary). Here a third action is wedged between cancel/danger and primary, breaking the convention.
- **Visual impact:** Users habituated to "Cancel on the left, Save on the right" find the primary action displaced.
- **Recommendation:** Either Cancel-first (secondary) → Reject (danger) → Accept-all (secondary) → Accept (primary), or move "Alle akzeptieren" out of the footer (e.g. as a checkbox in the body or a split-button).

### [SEV: Low] DocumentViewer `.dv-canvas { background: white }` not theme-aware
- **File:** `src/styles/components.css:3268`
- **Description:** PDF page background is hard-white. Acceptable for "this is a paper page" semantics, but the surrounding `.dv-canvas-container` uses `var(--color-bg)` — in dark mode this produces a glaring white rectangle in a dark panel.
- **Visual impact:** PDF preview is the most visually jarring screen in dark mode.
- **Recommendation:** Either keep white intentionally and add a subtle outer glow/border to soften the contrast, or expose a "dark page" toggle that inverts the canvas (already common in PDF viewers).

### [SEV: Low] `.app-status` uses `--color-border-light` while `.app-menu` uses `--color-border` — inconsistent border weight
- **File:** `src/styles/layout.css:50` (menu `border-bottom: 1px solid var(--color-border)`) and `102` (status `border-top: 1px solid var(--color-border-light)`)
- **Description:** Top menu bar gets a heavier border than the bottom status bar.
- **Visual impact:** Asymmetric framing of the main app — top edge looks more "structural" than bottom.
- **Recommendation:** Use the same token on both top and bottom (recommend `--color-border-light` for both — softer; the two app chrome bars feel more like a single frame).
