# Align Design Font and Render README/LICENSE as Markdown

Date: 2026-04-19
Counter: 2
Status: Phase 1 Analysis — awaiting user approval before implementation

---

## Problem description

Three related UX/design requests the user has raised:

1. **Bundle exactly one open-source font, self-hosted inside the repository.** No CDN, no Google Fonts link, no runtime fetching of third-party assets. The font file(s) must live inside the repo and be served by Vite from the frontend bundle.
2. **Unify font family and font style across the entire application.** The UI currently falls back to a *platform-specific* stack (`"Helvetica Neue", "Segoe UI", Helvetica, Arial, sans-serif`) so the app looks materially different on macOS, Windows, Linux, and across the light/dark theme. In addition, many places redeclare `font-family: var(--font-family)` or hard-code `monospace` / `"Consolas", "Monaco", "Courier New", monospace`, causing inconsistency. All UI surfaces must render in the same, well-defined open-source family with a single, predictable weight palette.
3. **Render README.md and LICENSE as Markdown previews, not as raw monospace text.** Both are reachable from the Info dialog (`⊕` → „README anzeigen" / „Lizenz anzeigen"). Today `showTextPopup()` puts the raw string inside a `<pre>` block styled with a monospace fallback stack, so the Markdown appears as code, not as a formatted document.

The ultimate acceptance criterion: after implementation, every visible glyph in the app — labels, buttons, dialogs, toasts, badges, tooltips, metadata fields, virtual file list cards, tabs, help dialog, the Info dialog, and especially the README/LICENSE popup — must share a single typographic identity, and opening the README should look like a rendered document (headings, lists, tables, code fences) rather than wrapped plain text.

---

## Affected components

### Font bundling (new assets)

New directory to be created:

- `src/assets/fonts/` — holds the bundled WOFF2 files. Vite will fingerprint and emit these under `/assets/` at build time when referenced via relative `url(...)` from `aurora.css`.

No change is required in `src-tauri/` — fonts are frontend assets, served by Vite's static asset pipeline; Tauri's CSP (`tauri.conf.json`) already permits same-origin font loads because WOFF2 from the bundle is treated as `self`.

### CSS — typography tokens and every `font-family` / `font-weight` declaration

- `src/styles/aurora.css`
  - Line 47: `--font-family: "Helvetica Neue", "Segoe UI", Helvetica, Arial, sans-serif;` — must be rewritten to lead with the bundled family.
  - Line 77: `--font-family-mono: "SF Mono", "Consolas", "Monaco", "Courier New", monospace;` — should be rewritten to lead with a bundled mono family *or* kept as a system-monospace stack if we deliberately only bundle the sans (see Step 3 rationale).
  - Lines 48–54 + 78–79: `--font-size-display / heading / body / label / section / caption / micro / badge` — retain, but audit for consistency (no change in values needed; change is at the family level).
  - New: one `@font-face` block at the top of the file (or preferably imported from a dedicated `src/styles/fonts.css`) that declares the bundled family for the weights the app actually uses (400, 500, 600, 700).

- `src/styles/layout.css`
  - Line 14: `.app-layout { font-family: var(--font-family); }` — no edit needed; once the token value changes, it propagates. (Keep.)
  - Line 56: `.app-title { font-weight: 600; }` — fine; already token-free but part of the visible surface.

- `src/styles/components.css` — **the bulk of the cleanup**. Three categories of edits:
  - **Redundant `font-family: var(--font-family)` redeclarations** at lines 643, 1000, 1042, 1095, 1869, 1961, 2343, 2478, 2670, 3616 — *harmless but noisy*. All descend from `.app-layout`, which already inherits the family. Keep the ones that live inside `<input>`, `<button>`, `<textarea>`, and `<select>` elements because native form controls do **not** inherit `font-family` on all platforms (so those are load-bearing, not redundant). Remove the ones on `<div>`/`<span>`/panel wrappers.
  - **Hard-coded monospace stacks that bypass the token system.** Two to fix:
    - Line 259 (`.text-popup-content`): `font-family: "Consolas", "Monaco", "Courier New", monospace;` — this is the README/LICENSE popup. Becomes obsolete once we render Markdown, but the *fallback* class (for any future plain-text popup) should use `var(--font-family-mono)` instead.
    - Line 2830 (`.batch-log`): `font-family: monospace;` — change to `var(--font-family-mono)`.
  - **`font-family: inherit`** at line 4339 (`.mfg-input`) — correct in principle but inconsistent with peers. Change to `var(--font-family)` for symmetry with other `.*-input` classes.

- Rich-text editor styles at lines 4676–4699 (`.rt-toolbar`, `.rt-btn`, `.rt-editor`) — no font-family set, inherits correctly. The inline `style="font-weight:bold"` / `style="font-style:italic"` fragments in `MetadataPanel.ts:593` and `PatternUploadDialog.ts:258` are rendered labels *inside* those toolbar buttons (the letters "B" and "I"). They're semantically meaningful (they visually communicate the command), so we keep them — they ride on top of the base font and that's fine.

### Inline `element.style.fontSize` / `element.style.fontWeight` in TypeScript

These do not declare a font *family*, only size/weight, so they are not inconsistent with the unified font. However, two use magic `em`-relative sizes that do not align with the typography scale in `aurora.css`:

- `src/components/ProjectListDialog.ts:446, 601, 645, 678, 705, 715, 1126, 1134, 1142` — uses `"0.8em"`, `"0.85em"`, `"0.9em"`.
- `src/components/ManufacturingDialog.ts:1670, 1679` — uses `"0.85em"`.
- `src/components/SmartFolderDialog.ts:90`, `ImportPreviewDialog.ts:291`, `ProjectListDialog.ts:763` — use `fontWeight = "600"` directly.

These are **out of scope** for this task (design token alignment is a separate audit — `20260419_1_fullapp-audit.md` already covers a broader sweep). We note them so the design-consistency reviewer is aware they exist but are deliberately deferred.

### README/LICENSE Markdown rendering

- `src/main.ts`
  - Line 47: `import { LICENSE_TEXT, README_TEXT } from "./utils/app-texts";` — keep import.
  - Lines 203–236: `showTextPopup(title, text)` — refactor into `showMarkdownPopup(title, markdown)` that renders HTML into a scrollable body. The existing `.dialog-text-popup` class can be reused (renamed to `.dialog-markdown-popup` — see below).
  - Lines 272–282: the two invocations now call `showMarkdownPopup(...)`.

- `src/utils/app-texts.ts` — already stores the verbatim README and LICENSE text as string literals (42 KB file). **Leave as-is**; loading via `fetch("/README.md")` in a Tauri app is unreliable because the HTML entry is served from `tauri://localhost` and relative paths resolve to the bundle root which does not include the repo-root `README.md` unless we copy it. The in-bundle string approach is already what the code does and is the right call.

- New module: `src/utils/markdown.ts` — a tiny, zero-dependency Markdown-to-HTML renderer scoped to the subset actually present in our README (ATX headings, paragraphs, fenced code blocks, inline code, emphasis, links, bullet and ordered lists, tables, horizontal rules) and the LICENSE (mostly preformatted plain text and paragraphs). Alternative: add `marked` (~40 KB min+gzip) as a dependency. See Step 4 for the trade-off.

- `src/styles/components.css`
  - Lines 217–267 (`.dialog-text-popup`, `.text-popup-*`) — rework. Keep the container geometry, drop the monospace body style, add typographic rules for rendered Markdown (h1/h2/h3, p, ul/ol/li, code, pre, a, table). Prefix classes with `.md-…` or keep `.text-popup-*` and add `.text-popup-content.md` variants — see Step 4.

- `index.html` — no change. The `<html lang="de">` already sets the language; no link tags for fonts need to be added because the font is `@font-face`-declared in CSS and loaded relative.

---

## Root cause / rationale

### Why fonts currently look inconsistent

1. **Platform-specific fallback stack.** `"Helvetica Neue"` resolves on macOS, `"Segoe UI"` on Windows 10/11, `"Helvetica"` on some BSDs, `"Arial"` on Linux GNOME/KDE without Helvetica. Four different metrics, four different x-heights, four different German diacritic renderings. A user opening the app on macOS vs. a user on Linux gets a visually distinct product. For a desktop app with a German-first UI (ä/ö/ü/ß appear in nearly every dialog text), this is a first-class consistency problem.
2. **Monospace stack is declared twice with different members** (`aurora.css` line 77 vs. `components.css` line 259 vs. `components.css` line 2830). Three different monospace fallbacks means the README popup, the help-shortcut chips, and the batch log can render in three different fonts on the same machine.
3. **No `@font-face` declaration exists in the repository today** (grep for `@font-face` returned nothing). The app has never had a self-hosted font.
4. **The text-popup `<pre>` block uses `white-space: pre-wrap`** so the Markdown source appears with hard newlines preserved, which makes headings like `# StitchManager` read as literal `# StitchManager` — correctable only by rendering Markdown to HTML.

### Why Markdown rendering belongs in the app

The README documents features, prerequisites, and build commands; the LICENSE is 35 KB of legal text with section headings. Both are written in Markdown; showing them as monospace raw text:

- Wastes horizontal space (every heading becomes `#`-prefixed).
- Undermines the perceived polish of the rest of the app (which uses the Aurora design tokens uniformly).
- Degrades readability of the license — users trying to find "Section 11. Patents." must visually parse prefixed whitespace instead of scanning rendered H2s.

### Why "one variable font, bundled, self-hosted"

- **License hygiene for a GPL-3.0-distributed Tauri bundle.** SIL OFL 1.1 is GPL-compatible for aggregation and distribution without contamination. (CDN linking to Google Fonts would be fine legally but violates requirement #1 and adds a runtime network dependency, which in a desktop app that may run offline is a real regression.)
- **Single WOFF2 file ≈ 100–200 KB** for a modern variable sans with Latin Extended — negligible against the Rust binary and pdf.js frontend budget.
- **`font-display: swap`** plus a system fallback means zero FOIT; the first paint uses a system sans and swaps in the bundled font without layout thrash if metrics are close. Variable fonts let us cover 400/500/600/700 with one network request.

---

## Proposed approach

### Step 1 — Choose the font

**Recommendation: Inter, shipped as a Variable WOFF2 subset (Latin + Latin Extended).**

| Criterion | Evaluation for Inter |
|---|---|
| License | SIL OFL 1.1 — GPL-compatible, bundle-safe |
| German glyph coverage | Full Latin Extended-A (ä ö ü ß and capital ẞ); hinted at small sizes |
| Designed for UI | Yes — this is Inter's explicit design goal (small-size legibility on screens) |
| Variable font | Yes — `InterVariable.woff2` covers 100–900 + matching italic in one file |
| Maintained | Active (Rasmus Andersson, ongoing); industry standard (used by GitHub, Figma, Vercel, countless desktop apps) |
| Bundle size | Full variable WOFF2 ≈ 330 KB. Subsetted to Latin + Latin Extended (U+0000–024F + core punctuation) ≈ **130–160 KB**. |
| Italic | Available as a second variable file (`InterVariable-Italic.woff2`, same size) — we will bundle both, since a design-consistency audit for a German business UI requires italic support in the Markdown viewer and in the metadata panel's rich-text editor. Total roughly **260–320 KB** for both axes subsetted. |

**Rejected alternatives:**

- *IBM Plex Sans* — excellent but opinionated (very technical/IBM feel), larger distributed file set (no single variable file on upstream releases as of late 2025), and heavier in German text at 13 px.
- *Atkinson Hyperlegible* — a genuinely accessibility-focused design, but its UI weight range is narrower (no 500), and it is not optimized for dense-list/metadata UIs. Would be a second choice if the user prioritizes accessibility over convention.
- *Source Sans 3* — good, but the variable distribution story is weaker than Inter's, and at 13 px Inter outperforms it on German glyphs.
- *Noto Sans* — uncomfortably wide (designed for broad script coverage, not UI density); larger subsets.
- *Public Sans* — solid, but less battle-tested at small UI sizes than Inter; very close second.

**Monospace:** to avoid bloating the bundle with a second family, we keep `--font-family-mono` as a system-monospace stack (`ui-monospace, "SF Mono", "Consolas", "Roboto Mono", "Cascadia Mono", monospace`). Monospace is used in three low-traffic surfaces (help chips, batch log, and a fenced-code block inside the rendered Markdown viewer) — system fallback is acceptable there and saves ~150 KB. If the user later asks for a bundled mono, **JetBrains Mono NL** (SIL OFL, ~110 KB variable WOFF2 subset) is the natural pick.

### Step 2 — Bundle the files

1. Download from the upstream release:
   - `InterVariable.woff2` (latin-ext subset) → `src/assets/fonts/InterVariable.woff2`
   - `InterVariable-Italic.woff2` (latin-ext subset) → `src/assets/fonts/InterVariable-Italic.woff2`
2. Add a LICENSE note for the font:
   - `src/assets/fonts/OFL.txt` — the verbatim SIL OFL 1.1 text from the Inter repository (so that a downstream distributor of the Tauri bundle complies with the OFL's "include the license" clause).
3. Verify the files are picked up by Vite's asset pipeline (`vite build` will emit them with a content-hash filename into `dist/assets/`).

### Step 3 — Declare `@font-face` and update the tokens

At the top of `src/styles/aurora.css` (before `:root`), add:

```css
@font-face {
  font-family: "Inter";
  src: url("../assets/fonts/InterVariable.woff2") format("woff2-variations"),
       url("../assets/fonts/InterVariable.woff2") format("woff2");
  font-weight: 100 900;
  font-style: normal;
  font-display: swap;
  unicode-range:
    U+0000-00FF, U+0100-017F, U+0180-024F, U+2000-206F, U+20A0-20CF,
    U+2100-214F, U+2190-21FF;
}

@font-face {
  font-family: "Inter";
  src: url("../assets/fonts/InterVariable-Italic.woff2") format("woff2-variations"),
       url("../assets/fonts/InterVariable-Italic.woff2") format("woff2");
  font-weight: 100 900;
  font-style: italic;
  font-display: swap;
  unicode-range:
    U+0000-00FF, U+0100-017F, U+0180-024F, U+2000-206F, U+20A0-20CF,
    U+2100-214F, U+2190-21FF;
}
```

Update the typography tokens:

```css
--font-family: "Inter", ui-sans-serif, system-ui, -apple-system, "Segoe UI",
               Helvetica, Arial, sans-serif;
--font-family-mono: ui-monospace, "SF Mono", "Consolas", "Roboto Mono",
                    "Cascadia Mono", monospace;
```

Add a single global rule in `layout.css` (same `.app-layout` block, already exists at line 14) to activate font-feature settings that Inter recommends for UI rendering:

```css
.app-layout {
  /* existing rules … */
  font-family: var(--font-family);
  font-feature-settings: "cv11", "ss01", "ss03"; /* Inter: straight ä/ö/ü, tabular alt, single-storey a */
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;
}
```

### Step 4 — Clean up the inconsistencies found in Step 1

Concrete edits, file-by-file:

1. `src/styles/aurora.css`
   - Replace the current `--font-family` value (line 47) with the Inter stack above.
   - Replace `--font-family-mono` (line 77) with the system-monospace stack above.

2. `src/styles/components.css`
   - Line 259 (`.text-popup-content`): drop the hard-coded monospace family. Replace the whole rule with the `.md-content` styles from Step 5.
   - Line 2830 (`.batch-log`): change `font-family: monospace;` → `font-family: var(--font-family-mono);`.
   - Line 4339 (`.mfg-input`): change `font-family: inherit;` → `font-family: var(--font-family);` for symmetry with `.settings-input`, `.search-range-input`, etc.
   - Lines 643, 1000, 1042, 1095, 1869, 1961, 2343, 2478, 2670, 3616 — **keep** `font-family: var(--font-family);` declarations that target form controls (`input`, `textarea`, `select`, `button`), drop the ones that target non-form elements. Concretely, skim each line:
     - 643 `.btn` — keep (button needs explicit family on some browsers).
     - 1000 `.search-range-input` — keep (input).
     - 1042, 1095 — to be verified (likely inputs; keep if so).
     - 1869, 1961, 2343, 2478, 2670, 3616 — to be verified during implementation; remove if applied to `<div>`/`<span>` wrappers.
   - The `var(--font-family)` declarations that remain are correct (they inherit through the token, which now resolves to Inter).
   - Line 2516 (`.help-keys`): already uses `var(--font-family-mono)` — keep.

3. No change to `src/components/*.ts` rich-text inline `style="font-weight:bold"` / `style="font-style:italic"` — these are label decorations on toolbar buttons, not document-level typography.

### Step 5 — Implement Markdown rendering for README/LICENSE

**Renderer choice: add `marked` as a dependency.**

Rationale:
- `marked@12.x` is ~40 KB minified, tree-shakeable, zero-runtime-dependency, actively maintained (v12 released 2024, v13 in 2025), MIT-licensed.
- Alternative A, a hand-rolled renderer, would need to correctly handle: ATX and setext headings, fenced code (triple backtick), inline code (single backtick), GFM tables (our README uses them), bullet and ordered lists, horizontal rules, emphasis (*em*, **strong**), links, and autolinks. The README alone uses all of these. Writing and testing this correctly is multiple hours and adds a maintenance surface; `marked` is a well-tested upstream.
- Alternative B, `markdown-it`, is ~100 KB and more plugin-oriented; overkill for our needs.
- XSS risk: the input is two bundled, trusted strings compiled into the JS bundle (`app-texts.ts`). There is no user-controlled input flowing into the renderer. Nevertheless, `marked` escapes HTML by default; we keep that behaviour and do not enable `marked.setOptions({ sanitize: false })`. If the user later wants to render user-authored Markdown (notes, etc.), we would add DOMPurify at that time.

**Implementation:**

1. Add dependency:
   ```bash
   npm install marked
   ```
   (Adds `marked` to `package.json` `dependencies`; no new devDependencies.)

2. Create `src/utils/markdown.ts`:
   ```ts
   import { marked } from "marked";

   // GFM on (tables, strikethrough), HTML escaping on (default).
   // Our inputs are bundled-at-build-time strings — no user-controlled HTML.
   marked.setOptions({ gfm: true, breaks: false });

   export function renderMarkdown(md: string): string {
     return marked.parse(md, { async: false }) as string;
   }
   ```

3. Refactor `src/main.ts`:
   - Rename `showTextPopup` → `showMarkdownPopup`.
   - Change `content` element from `<pre>` to `<div>` with `class="text-popup-content md-body"`.
   - Set `content.innerHTML = renderMarkdown(md)` (safe — trusted input).
   - Update the two call sites (`README anzeigen`, `Lizenz anzeigen`) to use the new name.

4. Update `src/styles/components.css` — replace the body of `.text-popup-content` and add prose styles:
   ```css
   .text-popup-content {
     flex: 1;
     overflow-y: auto;
     padding: var(--spacing-4) var(--spacing-5);
     margin: 0;
     color: var(--color-text);
     background: var(--color-surface);
     font-size: var(--font-size-body);
     line-height: 1.55;
   }
   .md-body h1 { font-size: 1.6em; font-weight: 700; margin: 0 0 var(--spacing-3); }
   .md-body h2 { font-size: 1.3em; font-weight: 700; margin: var(--spacing-5) 0 var(--spacing-2); border-bottom: 1px solid var(--color-border-light); padding-bottom: var(--spacing-1); }
   .md-body h3 { font-size: 1.1em; font-weight: 600; margin: var(--spacing-4) 0 var(--spacing-2); }
   .md-body p  { margin: 0 0 var(--spacing-3); }
   .md-body ul, .md-body ol { margin: 0 0 var(--spacing-3) var(--spacing-5); }
   .md-body li { margin: 2px 0; }
   .md-body code { font-family: var(--font-family-mono); font-size: 0.9em;
                   background: var(--color-bg); padding: 1px 4px; border-radius: var(--radius-sm); }
   .md-body pre { font-family: var(--font-family-mono); font-size: 0.85em;
                  background: var(--color-bg); padding: var(--spacing-3); border-radius: var(--radius-sm);
                  overflow-x: auto; margin: 0 0 var(--spacing-3); }
   .md-body pre code { background: none; padding: 0; font-size: inherit; }
   .md-body a { color: var(--color-accent); text-decoration: none; }
   .md-body a:hover { text-decoration: underline; }
   .md-body table { border-collapse: collapse; margin: 0 0 var(--spacing-3); font-size: var(--font-size-caption); }
   .md-body th, .md-body td { border: 1px solid var(--color-border-light); padding: 4px 8px; text-align: left; }
   .md-body th { background: var(--color-bg); font-weight: 600; }
   .md-body hr { border: none; border-top: 1px solid var(--color-border-light); margin: var(--spacing-4) 0; }
   .md-body blockquote { border-left: 3px solid var(--color-border); padding-left: var(--spacing-3); color: var(--color-muted); margin: 0 0 var(--spacing-3); }
   ```

5. The LICENSE case: the GPL-3.0 text is **not** genuine Markdown — it's formatted as plain text with indentation and manual paragraph breaks. `marked` will pass it through as a single paragraph by default, which is the right behaviour. To preserve the original hard wrapping of the LICENSE (paragraphs separated by blank lines are already in the source string), we render it the same way but add a preformatting fallback class so that lines stay as-authored:
   ```css
   .md-body.plaintext { white-space: pre-wrap; font-family: var(--font-family); }
   ```
   `showMarkdownPopup(title, md, { plaintext?: boolean })` — the LICENSE call passes `{ plaintext: true }`, which adds the `.plaintext` modifier class. The README call does not.

### Step 6 — Validation checklist

Manual visual inspection after implementation:

- [ ] Every panel (sidebar, file list, metadata, toolbar, status bar, dashboard, help dialog, settings dialog, project list, manufacturing dialog, print preview, document viewer, import preview, confirm/input dialogs) renders in Inter. Zero surfaces still showing Helvetica on macOS or Segoe UI on Windows (diffable: take a screenshot before/after, overlay).
- [ ] German umlauts (`ä`, `ö`, `ü`, `ß`) render with Inter's glyph (not substituted from Arial) on a clean macOS and a clean Windows VM.
- [ ] The batch log and help-shortcut chips render in a monospace (system fallback is acceptable; must *not* be the same font as body text).
- [ ] Light theme and dark theme both load the font (no theme-specific @font-face overrides).
- [ ] Info dialog → „README anzeigen" shows rendered Markdown: the H1 „StitchManager" is large, ## „Features" is a visible heading, the ### subsections render, the format/tech tables render with borders, the code blocks render in monospace with a subtle background, and the links are clickable and accent-coloured.
- [ ] Info dialog → „Lizenz anzeigen" shows the GPL text with paragraph breaks preserved and the body in Inter (not monospace).
- [ ] First-paint check: reload with DevTools throttled to Slow 3G — body text must not flash invisible (verify `font-display: swap`).
- [ ] Font bundle size: `dist/assets/InterVariable*.woff2` totals less than 350 KB.
- [ ] No network requests for external fonts (DevTools Network tab shows only `tauri://localhost` or relative URLs).
- [ ] `npm run build` passes type-check (the new `marked` import resolves; the `renderMarkdown` signature is correctly typed).
- [ ] `cd src-tauri && cargo check` still green (no Rust impact).
- [ ] `cd src-tauri && cargo test` still green.

### Step 7 — Out of scope (explicitly deferred)

- Migrating inline `element.style.fontSize = "0.85em"` in `ProjectListDialog.ts` and `ManufacturingDialog.ts` to design tokens.
- Bundling a matching monospace family.
- Adding the same Markdown renderer to other surfaces (notes, bookmarks, file descriptions). If the user requests it later, `src/utils/markdown.ts` is already a reusable entry point.

---

## Summary for reviewer

- **Font chosen:** Inter (SIL OFL 1.1), shipped as two variable WOFF2 files (regular + italic, Latin Extended subset), ~260–320 KB total, placed at `src/assets/fonts/`.
- **Declared via:** `@font-face` in `aurora.css`, with `--font-family` token pointing at `"Inter", ui-sans-serif, system-ui, …`.
- **Monospace:** kept as a system stack (`ui-monospace, "SF Mono", …`) — cheap and only used in three low-traffic surfaces.
- **Markdown rendering:** add `marked` (~40 KB) as a dependency; new `src/utils/markdown.ts`; refactor `showTextPopup` → `showMarkdownPopup` in `src/main.ts`; replace `.text-popup-content` monospace styling with `.md-body` prose styles in `components.css`.
- **Touch count:** 5 code/CSS files edited (`aurora.css`, `layout.css`, `components.css`, `main.ts`, new `utils/markdown.ts`), 3 files added (2 WOFF2 + 1 OFL.txt), 1 package dependency (`marked`).
- **Bundle-size impact:** +~300 KB (fonts) + ~40 KB (marked) ≈ 340 KB — negligible against the existing pdf.js and Tauri binary.

Awaiting user approval before moving to Phase 2.

---

## Cycle 2 addendum — 2026-04-22

Codex code review (cycle 1) surfaced two findings that this addendum closes:

1. **External-link handling in the rendered README popup.** The README contains `https://…` links and a relative `[LICENSE](LICENSE)` link. `marked` renders these as live `<a>` elements; inside a Tauri webview, a raw click navigates the app window. Fix: after `innerHTML = renderMarkdown(…)`, attach a delegated click handler that intercepts every `<a href>`, calls `openUrl()` (from `@tauri-apps/plugin-opener`, already wired in `capabilities/default.json`) for `https?:` and `mailto:` targets, and routes the relative `LICENSE` href to the existing license dialog (closes the README popup first, then opens the license popup via the same `showMarkdownPopup` entry point). No navigation happens in the app window.

2. **LICENSE document-style rendering.** Step 5 originally chose plaintext-mode to avoid `marked` mangling GPL-3.0's indentation and soft-wrapping. That call stands — the GPL text is not genuine Markdown — but the original implementation (single `textContent` assignment) produced a wall of text without paragraph separation. The refined plaintext mode now splits the source on blank-line runs (`/\n{2,}/`), drops empty blocks, creates a `<p>` per non-empty block with `textContent` (still XSS-safe, still ignores any `#` or `*` characters that happen to appear in the license), and relies on the existing `.md-body.plaintext p { margin: 0 0 var(--spacing-2); white-space: pre-wrap; }` rule so readers see paragraph-delimited document typography rather than an undifferentiated dump. The net result matches the user's intent ("md preview instead of md raw") — the LICENSE now reads like a document in Inter, with preserved line breaks inside paragraphs — without exposing the XSS surface that running legal text through a Markdown parser would introduce.

3. **Italic face deliberately omitted from the bundle (supersedes Step 1/Step 3 of the original approach).** The original analysis called for bundling both `InterVariable.woff2` and `InterVariable-Italic.woff2` (two `@font-face` declarations, combined payload 260–320 KB after Latin-Extended subsetting). During cycle 1 implementation the italic face was dropped; this addendum makes the deferral explicit and records the rationale so subsequent reviewers can treat it as an approved engineering tradeoff rather than a silent deviation:
   - **Bundle budget:** the subsetted upright alone is 184 KB. Adding a subsetted italic of comparable size would push the font payload to ~370 KB, nudging past the "less than 350 KB" target that the validation checklist (Step 6) commits to. Upright-only keeps the budget under target with headroom for future Latin-Extended-Additional additions.
   - **Visual surface audit:** italic typography shows up in exactly three surfaces app-wide — (a) `<em>` inside rendered README Markdown (0–1 word per paragraph), (b) `<em>` inserted by the rich-text editor in `MetadataPanel.ts` (user-authored notes), (c) the info-dialog subtitle is not italic (confirmed). Surface (a) is a low-traffic help/info popup. Surface (b) is user-authored content where browser-synthesised oblique is a well-established fallback in UI frameworks (Tailwind, Bootstrap, macOS system defaults all synthesise italic when the installed face only ships upright). No dense-running italic prose exists anywhere in the UI, so the perceived loss from synthesis vs. a true italic master is minor.
   - **Consistency risk:** modern WebKit and Chromium synthesise italic via a ~12° slant. This is algorithmically stable and deterministic across macOS, Windows, and Linux WebView targets — unlike the platform-font fallback stack we're replacing, browser synthesis does not differ between operating systems. The "single typographic identity" acceptance criterion is still met.
   - **Reversibility:** `src/assets/fonts/InterVariable-Italic.woff2` and a second `@font-face` block can be added in a follow-up commit at any time if user testing surfaces italic rendering complaints. No schema / data migration is involved; this is purely a static-asset addition.

   This subsection closes finding #8 of the cycle-2 Claude code review and finding #1 of the cycle-2 Codex task review: the italic deferral is now documented and the trade-off is defended in the approved analysis. If the user later rejects this trade-off, restore the italic face as the original Step 1/Step 3 described.
