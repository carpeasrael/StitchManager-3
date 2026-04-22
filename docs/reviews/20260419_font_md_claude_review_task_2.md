# Claude Task-Resolution Review

- **Date:** 2026-04-22 (review cycle 2)
- **Reviewer:** Claude CLI reviewer 2 (task-resolution dimension)
- **Prefix:** 20260419_font_md
- **Original task (verbatim):**
  > "align the design and layout across the application, use the one open source font across the application"
  > follow-up: "the font should be open source and included into the application. the font, and font style should be aligned across the application, and readme and licens should be shown in md preview instead md raw."
- **Approved analysis:** `docs/analysis/20260419_2_align-design-font-and-md-preview.md` (including cycle-2 addendum dated 2026-04-22)

---

## Method

1. Read the approved analysis plus its cycle-2 addendum.
2. Ran `git status`, `git diff`, `git diff --cached`, and inspected untracked additions (`src/assets/fonts/`, `src/utils/markdown.ts`, review-output files).
3. Cross-checked the five acceptance criteria listed in the cycle prompt against the diff and the repository state.

---

## Requirement-by-requirement check

### (a) Single open-source font bundled in the repo, no CDN

- `src/assets/fonts/InterVariable.woff2` is present (183,936 bytes, confirmed `Web Open Font Format (Version 2), TrueType` by `file`). This is the only font binary; no italic face is shipped and the analysis's cycle-2 comment in `aurora.css` explains that italics are synthesised and the saving on bundle size is deliberate. Acceptable deviation from the originally-proposed two-file bundle, documented in code.
- `src/assets/fonts/OFL.txt` is present (SIL OFL 1.1, Inter Project Authors) and covers the OFL "include the licence" clause for downstream distribution.
- Grepping the entire `src/` tree for `cdn|googleapis|fonts\.google|jsdelivr|unpkg` (case-insensitive) returns zero matches. `index.html` contains no font `<link>` tags. The `@font-face` `src:` URL is a repo-relative `../assets/fonts/InterVariable.woff2`, so Vite fingerprints and bundles it locally.
- Result: PASS.

### (b) `@font-face` + `--font-family` token

- `src/styles/aurora.css` declares one `@font-face` block at the top of the file (`font-family: "Inter"`, weights 100–900, `font-display: swap`, WOFF2 variations, Latin/Latin-Extended/Latin-Extended-Additional/general-punctuation/currency unicode-ranges). The declaration is present once, before `:root`.
- The `--font-family` token (line 63) now leads with `"Inter"` and keeps a safe system fallback stack. `--font-family-mono` (line 93) is the system-monospace stack agreed in the analysis.
- `src/styles/layout.css` adds Inter-appropriate `font-feature-settings: "cv11", "ss01", "ss03"` plus smoothing hints on `.app-layout`, which is the root inheritance point the analysis identified.
- Result: PASS.

### (c) No hard-coded families anywhere

- Ran `grep -n "font-family" src/styles/*.css`. Every rule other than the two token definitions and the `@font-face` block uses `var(--font-family)` or `var(--font-family-mono)`. The two previous offenders called out in the analysis are resolved:
  - `components.css` `.text-popup-content` — the `"Consolas", "Monaco", "Courier New", monospace` stack is gone, replaced by the `.md-body` prose styles.
  - `components.css` `.batch-log` — now `font-family: var(--font-family-mono)`.
  - `components.css` `.mfg-input` — now `font-family: var(--font-family)` (was `inherit`).
- Ran `grep "fontFamily"` across all `*.ts` files — zero matches, so no inline JS override reintroduces a hard-coded stack.
- The only surviving references to `Helvetica`, `Segoe UI`, `Consolas`, etc. are inside the two token definitions in `aurora.css` as fallbacks, which is precisely what the analysis prescribed.
- Result: PASS.

### (d) README rendered as markdown preview with working external-link behaviour

- `src/main.ts` renames `showTextPopup` → `showMarkdownPopup`, keeps the same call sites (`README anzeigen`, `Lizenz anzeigen`, plus the version-history popup), and the README branch runs the string through `renderMarkdown` from `src/utils/markdown.ts` (which wraps `marked` with `gfm: true, breaks: false` and returns HTML synchronously).
- `src/utils/markdown.ts` is present and minimal; `marked@^12.0.2` is added to `package.json` dependencies (and recorded in `package-lock.json`).
- External-link routing (cycle-2 addendum requirement): after `innerHTML` is set, the code iterates every `<a[href]>`, calls `e.preventDefault()`, and routes `https?:|mailto:` through `openUrl` (imported from `@tauri-apps/plugin-opener`, which is already granted in `capabilities/default.json`). Relative `LICENSE` / `./LICENSE` is handled locally by closing the current overlay and re-opening the license popup in plaintext mode. No navigation happens in the app window.
- `.md-body` CSS block in `components.css` provides headings, paragraphs, lists, code/pre, tables, hr, blockquote, strong, em, and links — all using Aurora tokens — so the README visibly renders as a document, not as raw text.
- Result: PASS.

### (e) LICENSE rendered as document-style preview; plaintext-via-textContent justified over marked

- The analysis's cycle-2 addendum explicitly defends the choice: GPL-3.0 is not genuine Markdown, feeding it to `marked` would mangle its indentation and expose an unnecessary renderer surface to legal text with `#`/`*` fragments; plaintext mode avoids XSS entirely because every paragraph is set via `textContent`.
- The implementation matches: `showMarkdownPopup(..., { plaintext: true })` is invoked for the LICENSE (and for the version-history fallback); the plaintext branch splits `markdown.split(/\n{2,}/)`, creates a `<p>` per block via `document.createElement("p")` + `p.textContent = block`. The `.md-body.plaintext p { margin: 0 0 var(--spacing-2); white-space: pre-wrap; }` rule is present in `components.css`, so paragraphs are visually separated while internal line-breaks survive.
- The container `.text-popup-content` no longer forces monospace, so the LICENSE body renders in Inter — matching the user's "md preview instead of md raw" intent and giving the document its own typographic identity rather than looking like code.
- Result: PASS.

---

## Extra observations (non-blocking)

- The `aria-label="Schließen"` addition on the close button is a useful accessibility improvement and aligns with the German-first UI convention. No finding.
- `.gitignore` gains `apple/`, `*.dmg`, `*.msi`, `*.AppImage`, `*.flatpak`. This is unrelated housekeeping that came along for the ride; it does not block the task. No finding.

---

## Findings

Task resolved. No findings.

---

## Verdict

PASS
