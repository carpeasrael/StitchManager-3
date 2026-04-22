# Claude Task-Resolution Review

- **Reviewer:** Claude CLI reviewer 2 (task resolution)
- **Date:** 2026-04-19
- **Cycle:** 4
- **Prefix:** `20260419_font_md`
- **Task source:** Direct user prompt — "align the design and layout across the application, use the one open source font across the application" + "the font should be open source and included into the application. the font, and font style should be aligned across the application, and readme and licens should be shown in md preview instead md raw."
- **Approved analysis:** `docs/analysis/20260419_2_align-design-font-and-md-preview.md` (including the cycle-2 addendum that defers the italic face with documented rationale).
- **Cycle-3 fix under review:** `html, body { font-family: var(--font-family); }` in `src/styles.css` so `document.body`-mounted dialogs/overlays render in Inter.

---

## Requirements cross-check

### 1. One open-source font bundled inside the application

- `src/assets/fonts/InterVariable.woff2` — 183,936 bytes, upright variable axis (100–900). Present and tracked.
- `src/assets/fonts/OFL.txt` — 4,380 bytes, SIL Open Font License 1.1. Present and tracked (OFL "include the licence" clause satisfied for downstream redistribution).
- No CDN, no Google Fonts `<link>`, no runtime fetch. The font is loaded via a relative `url("../assets/fonts/InterVariable.woff2")` from `aurora.css`, so Vite's asset pipeline fingerprints and emits it inside the bundle.
- Italic face deliberately omitted per the cycle-2 addendum (subsection 3), which is part of the approved analysis. Not a gap.

**Met.**

### 2. `@font-face` declaration + typography token

- `src/styles/aurora.css:6–17` declares `@font-face` for `"Inter"`, `font-weight: 100 900`, `font-display: swap`, with a Latin + Latin-Extended-A/B + Vietnamese + punctuation/currency/letterlike unicode-range. Declaration lives at the top of the file, before `:root`, so every downstream token that references Inter can resolve the face.
- `--font-family` token (line 64) is now `"Inter", ui-sans-serif, system-ui, -apple-system, "Segoe UI", Helvetica, Arial, sans-serif` — Inter first, with a system-sans fallback chain that keeps the app usable during the brief `font-display: swap` window.
- `--font-family-mono` token (line 94) is a system-monospace stack (`ui-monospace, "SF Mono", "Consolas", "Roboto Mono", "Cascadia Mono", "Courier New", monospace`) — consistent with the analysis's monospace decision (no bundled mono).
- `src/styles/layout.css:17–20` adds Inter's recommended `font-feature-settings: "cv11", "ss01", "ss03"` plus smoothing hints on `.app-layout`.

**Met.**

### 3. Typography aligned across the entire application (including dialogs — cycle-3 concern)

The cycle-3 concern was that dialogs/overlays appended to `document.body` do not descend from `.app-layout`, so they would fall back to the UA default (`Times` in most WebKit builds) rather than Inter.

- **Root-level fix verified:** `src/styles.css:5–9` now sets `html, body { background: var(--color-bg); color: var(--color-text); font-family: var(--font-family); }`. Every element in the document inherits Inter unless it explicitly overrides the family. All 24 dialog components I checked (`BatchDialog`, `MetadataPanel`'s rich-text editor, `PatternUploadDialog`, `ProjectListDialog`, `HelpDialog`, `ImportPreviewDialog`, `SmartFolderDialog`, `FolderMoveDialog`, `FolderDialog`, `Sidebar` context menu, `ManufacturingDialog`, `Toast`, `SettingsDialog`, `InputDialog`, `ConfirmDialog`, `PrintPreviewDialog`, `ImageViewerDialog`, `DocumentViewer`, `SearchBar`, `ImagePreviewDialog`, `EditDialog`, `AiResultDialog`, `AiPreviewDialog`, plus the README/LICENSE popup created in `main.ts`) append to `document.body` and therefore now inherit from `body`.
- **No residual hardcoded family stacks:** `grep 'font-family\s*:\s*["\']'` against `src/` returns exactly two lines — the `@font-face` declaration and the `--font-family` token in `aurora.css`. No component, no stylesheet, no inline style redeclares `"Helvetica Neue"`, `"Segoe UI"`, `"Consolas"`, `"Monaco"`, `"Courier New"`, or bare `monospace`. The previous offenders (`.text-popup-content`, `.batch-log`, `.mfg-input`) are all now on tokens.
- **Form controls retain explicit `var(--font-family)`:** 15 of the 17 remaining `font-family` declarations in `components.css` target inputs, textareas, selects, and buttons (native form controls do not reliably inherit family on every browser). This matches the analysis guidance (Step 4, "Keep the ones that live inside `<input>`, `<button>`, `<textarea>`, and `<select>` elements").
- **Monospace surfaces unified:** `.batch-log` (line 2938), `.help-keys` (line 2624), `.md-body code` (line 310), `.md-body pre` (line 317) all use `var(--font-family-mono)`. No rogue monospace stack remains.

**Met.** The cycle-3 fix is correctly applied at the `html, body` selector, which is the correct top-level owner for dialogs that live outside the `.app-layout` subtree.

### 4. README.md shown as Markdown preview, not raw

- `src/utils/markdown.ts` wraps `marked@12.0.2` via `new Marked({ gfm: true, breaks: false })` and exports a synchronous `renderMarkdown(src)`; the Promise branch throws (defensive, will never fire because `async: false`).
- `src/main.ts:204–275`: `showMarkdownPopup(title, markdown, options)` replaces `showTextPopup`. The README branch (no `plaintext` flag) sets `content.innerHTML = renderMarkdown(markdown)` on a `<div class="text-popup-content md-body">`.
- Call site at line 317 (`README anzeigen`) passes README through the Markdown-rendered branch.
- `src/styles/components.css:254–374`: `.text-popup-content` body styling dropped the old `Consolas, Monaco, Courier New` monospace and `font-size: 0.75rem`. Full prose styles for `.md-body` are added — `h1`–`h6`, `p`, `ul/ol/li`, `code`, `pre`, `a`, `table`/`th`/`td`, `hr`, `blockquote`, `strong`, `em`. All colours/spacings/radii go through Aurora tokens. Zero inline styles.
- **Safe link routing verified:** `main.ts:246–268` intercepts every `<a href>` inside the rendered README. Hash links pass through; `https?:` and `mailto:` are handed to `openUrl()` (from `@tauri-apps/plugin-opener`, already permitted in `capabilities/default.json`); the relative `LICENSE`/`./LICENSE`/`LICENSE.md` pattern is mapped to the license dialog. Every other `href` is logged via `console.debug` and the default navigation is prevented, so no raw `<a>` click can navigate the app window. This addresses cycle-2 finding #1.

**Met.**

### 5. LICENSE shown as document-style Markdown preview, not raw

- `main.ts:214–225` (plaintext branch): splits the GPL source on `/\n{2,}/`, drops empty runs, and wraps each non-empty block in a `<p>` via `textContent` — XSS-safe (never touches `innerHTML`) and treats any stray `#`/`*` characters in the legal text as literal content.
- `components.css:371–378`: `.md-body.plaintext { white-space: pre-wrap; font-family: var(--font-family); }` combined with `.md-body.plaintext p { margin: 0 0 var(--spacing-2); white-space: pre-wrap; }` gives each paragraph its own spacing while preserving authored line breaks inside a paragraph.
- Call site at line 325 passes `{ plaintext: true }` for the LICENSE button. A second internal callsite at `main.ts:799–803` passes `{ plaintext: true }` for version-history text (appropriate — that is plain log data, not Markdown).
- Result matches the cycle-2 addendum subsection 2 — "LICENSE now reads like a document in Inter, with preserved line breaks inside paragraphs" — without routing legal text through the Markdown parser.

**Met.**

### 6. Ancillary requirements from the analysis

- `marked@12.0.2` added to `package.json` dependencies; `package-lock.json` updated. Zero-runtime-dependency, MIT-licensed.
- No changes required in `src-tauri/` (font is a frontend asset; CSP `self` already permits same-origin font loads).
- `.gitignore` additions (`apple/`, `*.dmg`, `*.msi`, `*.AppImage`, `*.flatpak`) are release artifact exclusions — tangential to the task but harmless and correctly scoped.
- Aria label `aria-label="Schließen"` added to the close button — minor usability improvement, no regression.

**Met.**

---

## Gaps

None. All four acceptance criteria from the original user prompt plus both cycle-2 addendum items (safe link routing, paragraph-split LICENSE) and the cycle-3 concern (body-level font inheritance for dialogs/overlays) are addressed by the uncommitted changes.

---

## Verdict

Task resolved. No findings.

PASS
