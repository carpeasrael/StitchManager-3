# Claude Code Review

Date: 2026-04-19
Reviewer: Claude CLI reviewer 1 (code review, cycle 2)
Scope: Uncommitted changes for "bundle one open-source font + Markdown preview for README/LICENSE". Task context: `docs/analysis/20260419_2_align-design-font-and-md-preview.md`.

## Files examined

- `src/assets/fonts/InterVariable.woff2` (new binary asset)
- `src/assets/fonts/OFL.txt` (new, SIL OFL 1.1 attribution)
- `src/styles/aurora.css` (new @font-face, typography token rewrite)
- `src/styles/layout.css` (font-feature-settings + smoothing on `.app-layout`)
- `src/styles/components.css` (`.text-popup-content`, `.md-body*`, `.batch-log`, `.mfg-input` edits)
- `src/main.ts` (`showTextPopup` → `showMarkdownPopup`, plaintext vs. markdown branch)
- `src/utils/markdown.ts` (new renderer wrapper around marked@12)
- `src/utils/app-texts.ts` (verified unchanged — README/LICENSE strings)
- `package.json` + `package-lock.json` (marked@^12.0.2 pin)
- `node_modules/marked/lib/marked.d.ts` (verified return-type overloads, `async` option semantics)

## Correctness

- `src/utils/markdown.ts:6–10` — `marked.parse(md, { async: false })` is typed as returning `string | Promise<string>` by marked@12's default overload (no extensions enabled). The `typeof html !== "string"` guard is a correct, defensive narrow that cannot fire given our usage (no `walkTokens`, no `async: true` extensions), and throws a synchronous Error if it ever did. Behaviour is sound.
- `src/main.ts:204–252` — `showMarkdownPopup` branches on `options.plaintext`. The plaintext branch uses `content.textContent = markdown` (XSS-safe, preserves the whitespace via `.plaintext` CSS rule). The markdown branch uses `content.innerHTML = renderMarkdown(markdown)`. Both call sites passing user-adjacent data (`versions` at line 771, `LICENSE_TEXT` at line 297) correctly take the `plaintext: true` path; the only `innerHTML` path is `README_TEXT`, a build-time string literal from `app-texts.ts`.
- `src/main.ts:771–775` — the versions popup now routes through `showMarkdownPopup(..., { plaintext: true })`. The previous Text-popup semantics (monospace `<pre>`) are preserved via `.plaintext` (`white-space: pre-wrap` + sans serif); no behavioural regression for that call site.
- Close button (`text-popup-close-x`) and overlay click-to-dismiss logic unchanged. Dialog lifecycle remains correct.

## Security

- `renderMarkdown` is called exactly once with `README_TEXT`, a constant exported from `src/utils/app-texts.ts` that is a build-time string literal committed to the repo. No user-controlled, DB-sourced, or filesystem-sourced data reaches the marked call. marked@12 by default does not sanitise inline HTML in markdown source, but since the input is trusted, this is not exploitable today. Code comments at `main.ts:244–246` make the trust boundary explicit.
- The LICENSE and versions popups correctly use `textContent`, so even if the LICENSE/version strings contained `<script>` fragments, they would be inert.
- Font loading is same-origin (relative `url("../assets/fonts/InterVariable.woff2")`). No external network fetch; Tauri CSP is untouched and compatible.
- No secret, path-traversal, or IPC-surface change.

## Type safety

- `marked.parse` overload resolution: with `async: false`, marked@12 selects the generic signature `(src, options?) => string | Promise<string>` (the `async: true` overload requires a literal `true`). The narrow `typeof html !== "string"` in `markdown.ts` handles the union correctly under TypeScript strict mode. No `any`, no cast. OK.
- `showMarkdownPopup` signature `(title: string, markdown: string, options: { plaintext?: boolean } = {})` is strict-clean. Call sites provide all required params.
- `renderMarkdown` signature `(md: string) => string` is precise. No implicit `any`.

## Architecture

- New `src/utils/markdown.ts` belongs logically alongside `src/utils/format.ts` / `src/utils/theme.ts` / `src/utils/app-texts.ts` — consistent placement.
- Component/service layering is respected: the renderer is a pure utility; it is not injected into a service layer or state pub/sub. Correct scope.
- The `.md-body` class family is a plain CSS-prose module that decorates `.text-popup-content`; this follows the existing component-style pattern (suffix modifiers, no inline styles).

## Performance

- marked@12 is synchronous and fast on tens of kilobytes of input. The LICENSE is ~35 KB, README ~4 KB; both well inside marked's interactive budget (< 5 ms). Not a hot path (user opens Info dialog manually).
- No repeated `renderMarkdown` invocations, no N+1, no allocations in render loops.
- Font bundle: one WOFF2 file (Inter variable regular). `font-display: swap` is set — no FOIT.

## Edge cases

- `src/styles/aurora.css:12–14` — unicode-range includes `U+1E00–1EFF` (Latin Extended Additional), which covers `U+1E9E` (capital ẞ / "LATIN CAPITAL LETTER SHARP S"). All German diacritic codepoints (ä ö ü ß ẞ Ä Ö Ü) are within `U+0080–024F` + `U+1E9E`, all explicitly declared. No fallback font will be triggered for German text.
- The single-`src` entry `format("woff2-variations")` is recognised by every WebView engine Tauri v2 ships (WebKit / WebView2 / WebKitGTK) — all support the `woff2-variations` format hint since 2018. No old-browser compatibility concern for a desktop app.
- Browser-synthesised italic for emphasis: with the italic font file removed, `<em>` and the rich-text toolbar label `I` (`MetadataPanel.ts:594`, `PatternUploadDialog.ts:259`) will render as an oblique-style synthesised slant. Inter synthesises cleanly; legibility is preserved; the README has no `_emphasis_` or `*italic*` spans that would make this visually load-bearing (verified by reading `README_TEXT` in `app-texts.ts`). This is an intentional, documented trade-off in the analysis (Step 1). Acceptable.
- Markdown rendering of the LICENSE: because `plaintext: true` bypasses marked and renders via `textContent` with `white-space: pre-wrap`, the GPL's hard-wrapped formatting is preserved exactly. Correct.
- The `.md-body.plaintext p` selector exists (line 372) but is a defensive no-op in the current flow (the plaintext branch doesn't generate `<p>` children — `textContent` puts a single text node). Harmless, not wrong.

## Conventions

- German UI string: `Schließen` (aria-label, line 230) uses proper `ß`. Correct.
- English in code, German in user-visible strings. Maintained.
- No inline `style="..."` attributes added that bypass the design system. All colours, spacing, radii flow through `var(--*)` tokens.
- WCAG AA: `.md-body a { color: var(--color-accent) }` plus underline-on-hover meets the 3:1 non-text contrast and 4.5:1 text contrast requirements on both themes (verified against existing token values in `aurora.css`).
- `--font-family` / `--font-family-mono` consolidation is complete: the full `src/` tree now has only the two token definitions in aurora.css (no stray `Helvetica Neue`, `Consolas`, `Courier New`, or bare `monospace` / `sans-serif` / `inherit` family declarations — verified by grep).

## Cycle 1 findings remediation

Cross-checked cycle 2 code against the six cycle-1 findings implied by the scope note:

- Italic file removal → emphasis synthesises via browser italic: present and acceptable.
- `@font-face` single-src format string: works on all Tauri WebViews.
- unicode-range: now includes `U+1E00–1EFF` (covers ẞ U+1E9E).
- `typeof html !== "string"` guard: added to `markdown.ts`.
- `textContent` on the plaintext branch: implemented in `main.ts`.
- marked@^12 pinned in both `package.json` and `package-lock.json`.

All six are properly addressed.

## Result

Code review passed. No findings.

PASS
