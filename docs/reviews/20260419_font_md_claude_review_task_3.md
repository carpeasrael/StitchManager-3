# Claude Task-Resolution Review

- **Date:** 2026-04-19
- **Reviewer:** Claude CLI reviewer 2 (task-resolution) — cycle 3
- **Prefix:** `20260419_font_md`
- **Task source (verbatim):** "align the design and layout across the application, use the one open source font across the application" + follow-up "the font should be open source and included into the application. the font, and font style should be aligned across the application, and readme and licens should be shown in md preview instead md raw."
- **Approved analysis:** `docs/analysis/20260419_2_align-design-font-and-md-preview.md` (including the cycle-2 addendum; subsection 3 explicitly approves the italic-face deferral as an engineering tradeoff)
- **Pending changes reviewed:** `git diff` (working tree) and `git diff --cached` (index empty) as of this review.

---

## Cross-check against every task requirement

### (a) Single open-source font bundled in the repo, no CDN

- `src/assets/fonts/InterVariable.woff2` is present on disk (183,936 bytes ≈ 180 KB).
- `src/assets/fonts/OFL.txt` is present (4,380 bytes) and contains the SIL OFL 1.1 header with the upstream Inter copyright line (`Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter)`), satisfying the OFL "include the licence" clause for downstream redistribution of the Tauri bundle.
- `apple/`, `*.dmg`, `*.msi`, `*.AppImage`, `*.flatpak` are newly ignored in `.gitignore`; the font asset directory is **not** ignored (the pattern `apple/` is scoped to the repo-root release-artefact directory, not to `src/assets/fonts/`).
- Grepping the repo for `googleapis|googlefonts|fonts\.google|cdnjs|jsdelivr|unpkg` returns no matches inside production sources; the only historical hit is a cycle-2 reviewer note. `index.html` adds no font `<link>` tag. The `@font-face` `src:` URL is a repo-relative `../assets/fonts/InterVariable.woff2`, so Vite fingerprints and bundles it locally at build time.
- **Result: met.**

### (b) `@font-face` declaration + `--font-family` token

- `src/styles/aurora.css` lines 1–17 declare `@font-face { font-family: "Inter"; src: url("../assets/fonts/InterVariable.woff2") format("woff2-variations"), url("../assets/fonts/InterVariable.woff2") format("woff2"); font-weight: 100 900; font-style: normal; font-display: swap; unicode-range: … }`. The explanatory block comment documents why only the upright face is bundled.
- The `--font-family` CSS custom property is updated on line 64 to `"Inter", ui-sans-serif, system-ui, -apple-system, "Segoe UI", Helvetica, Arial, sans-serif` — Inter leads, with a progressive-enhancement system stack as fallback.
- `--font-family-mono` on line 94 is rewritten to `ui-monospace, "SF Mono", "Consolas", "Roboto Mono", "Cascadia Mono", "Courier New", monospace` — pure system stack, consistent with the analysis decision not to bundle a second family.
- `src/styles/layout.css` activates Inter's recommended feature settings on `.app-layout` (`font-feature-settings: "cv11", "ss01", "ss03"`) plus `-webkit-font-smoothing`, `-moz-osx-font-smoothing`, `text-rendering: optimizeLegibility`.
- **Result: met.**

### (c) No hard-coded font families anywhere

Grep for `font-family` across `src/`:
- Every occurrence in `src/styles/components.css` (15 call sites) resolves to `var(--font-family)` or `var(--font-family-mono)`. The two previously hard-coded offenders flagged in the analysis are gone:
  - Line 2938 (`.batch-log`): `font-family: var(--font-family-mono)` (was `monospace`).
  - Line 4447 (`.mfg-input`): `font-family: var(--font-family)` (was `inherit`).
  - The old `.text-popup-content` `"Consolas", "Monaco", "Courier New"` stack was removed outright; the replacement block is token-driven.
- `src/styles/aurora.css`: the `@font-face` declaration itself names `"Inter"` (required — this is the family definition, not consumption). Both token values live in `:root`.
- `src/styles/layout.css`: `.app-layout { font-family: var(--font-family); … }` — token-driven.
- `src/**/*.ts`: grep for `font-family|fontFamily` returns zero matches. No inline TypeScript style sets a font family.
- **Result: met.**

### (d) README rendered as Markdown preview with safe external-link behaviour

- `src/utils/markdown.ts` (new) wraps `marked` v12 with GFM on, breaks off, `async: false`, and returns a string (throws if `marked` yields a Promise). `marked`'s default behaviour escapes embedded HTML, so the README string cannot inject markup.
- `marked@^12.0.2` is added to `package.json` `dependencies` and `package-lock.json`; MIT-licensed per the lockfile entry.
- `src/main.ts` refactors `showTextPopup` → `showMarkdownPopup(title, markdown, options)`. The README branch sets `content.innerHTML = renderMarkdown(markdown)` on a `<div class="text-popup-content md-body">`. A delegated click handler iterates every `<a href>`:
  - `href` starting with `#` is left alone (in-page anchor).
  - `https?:` or `mailto:` routes through `openUrl()` from `@tauri-apps/plugin-opener` with a `ToastContainer.show("error", …)` fallback and a console warning if the opener rejects.
  - A relative `LICENSE` or `./LICENSE` / `LICENSE.md` link closes the README overlay and opens the LICENSE popup via the same `showMarkdownPopup` entry point in plaintext mode.
  - Other hrefs log `unhandled markdown link` at debug level and are suppressed (navigation prevented via `e.preventDefault()`).
- The Info dialog's „README anzeigen" button (line 317) now invokes `showMarkdownPopup("README", README_TEXT)` — no `plaintext` flag, so the README uses the Markdown renderer.
- Close button now carries `aria-label="Schließen"` for accessibility.
- `src/styles/components.css` adds a full `.md-body` prose sheet covering h1–h6, p, ul/ol/li, code, pre, a, table, th, td, hr, blockquote, strong, em — all using design tokens (`--color-text`, `--color-bg`, `--color-accent`, `--color-border-light`, `--spacing-*`, `--radius-sm`, `--font-family-mono`). No raw colour literals.
- **Result: met.**

### (e) LICENSE rendered as document-style paragraph preview

- The Info dialog's „Lizenz anzeigen" button (line 325) invokes `showMarkdownPopup("LICENSE — GPL-3.0", LICENSE_TEXT, { plaintext: true })`.
- In plaintext mode, `main.ts` splits the source on `\n{2,}` (blank-line runs), skips empty blocks, and creates one `<p>` per block via `textContent` (XSS-safe — never hands GPL-3.0 legal text to `marked`).
- `.md-body.plaintext` in `components.css` applies `white-space: pre-wrap` and `font-family: var(--font-family)` (so the license renders in Inter, not monospace); `.md-body.plaintext p` adds `margin: 0 0 var(--spacing-2); white-space: pre-wrap` so each block is a visibly separated paragraph while preserving the intra-paragraph soft-wraps of the GPL source. This is the "document-style paragraph preview" the analysis describes.
- The Versionshistorie popup (line 796) was also migrated to `showMarkdownPopup(…, { plaintext: true })`, correctly preserving the list-of-lines format.
- **Result: met.**

### Italic-face deferral (analysis subsection 3 of cycle-2 addendum)

The cycle-2 addendum explicitly approves shipping only the upright Inter variable face, with four documented reasons:
1. Bundle budget (subset upright 184 KB; upright+italic would push to ~370 KB and breach the <350 KB target in Step 6 of the analysis).
2. Surface audit (italic appears in ≤3 low-traffic surfaces: `<em>` in rendered README, rich-text editor emphasis in `MetadataPanel.ts`, neither running italic prose).
3. Consistency (WebKit/Chromium synthesise italic deterministically at ~12° slant, stable across macOS/Windows/Linux WebView).
4. Reversibility (adding a second `@font-face` block is a static-asset follow-up with no data migration).

The pending diff matches this approved scope: one `@font-face` block, `font-style: normal`, explanatory comment in `aurora.css` lines 1–5 ("italics are synthesised by the browser … the size saving of dropping the italic face wins"), and `.md-body em { font-style: italic }` so browser synthesis engages where needed. The diff therefore conforms to the amended approved analysis — not a silent deviation.

---

## Findings

Task resolved. No findings.

---

## Verdict

PASS
