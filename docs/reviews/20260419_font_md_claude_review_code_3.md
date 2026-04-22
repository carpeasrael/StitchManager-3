# Claude Code Review

- Reviewer: Claude CLI reviewer 1 (code review, cycle 3)
- Date: 2026-04-19
- Prefix: `20260419_font_md`
- Scope: uncommitted diff for "align design/font across app and render README/LICENSE as markdown preview"
- Files inspected:
  - `src/main.ts`
  - `src/utils/markdown.ts` (new, untracked)
  - `src/styles/aurora.css`
  - `src/styles/layout.css`
  - `src/styles/components.css`
  - `src/assets/fonts/InterVariable.woff2`, `OFL.txt` (new assets, untracked)
  - `package.json` + `package-lock.json` (marked@12.0.2 added)
  - `.gitignore` (unrelated release-artifact ignore additions)

## Methodology

- Ran `git diff` and `git diff --cached` on the working tree.
- Re-reviewed every changed line from scratch against the criteria in `.claude/commands/review-code.md` (correctness, security, type safety, architecture, performance, edge cases, conventions).
- Verified TypeScript strict build (`npx tsc --noEmit` exit 0) and full production build (`npm run build` succeeds; font emitted as `dist/assets/InterVariable-*.woff2` at 183.94 KB).
- Confirmed the Marked 12.x API surface (`new Marked(MarkedExtension)`, instance `parse(src, MarkedOptions): string | Promise<string>`) matches the usage in `src/utils/markdown.ts`.
- Confirmed the `opener:default` permission set (`/System/Volumes/Data/Users/carpeasrael/.cargo/registry/src/index.crates.io-.../tauri-plugin-opener-2.5.3/permissions/default.toml`) already grants `allow-open-url`, so the `openUrl(href)` call requires no new capability.
- Confirmed `ToastContainer.show` signature (`(level: ToastLevel, message, duration?)`) and that `"error"` is a valid `ToastLevel` union member (`src/types/index.ts:704`).
- Confirmed the README contains a `[LICENSE](LICENSE)` link (`README.md:154`) that the LICENSE regex `/^\.?\/?LICENSE(\.md)?$/i` catches, and no `#`-anchor links exist in the source, so the hash-bypass branch is defensive rather than load-bearing.
- Confirmed the `@font-face` relative URL `../assets/fonts/InterVariable.woff2` (from `src/styles/aurora.css`) resolves to the committed file; the file header `wOF2` is a valid WOFF2 magic number.
- Confirmed `dialog-dismiss` listener continuity with the Escape key handler at `src/main.ts:1090` (`overlay.dispatchEvent(new CustomEvent("dialog-dismiss"))`).

## Targeted re-verification of cycle-2 fixes

1. **Plaintext paragraph split skips empty blocks** (`src/main.ts:242–247`): the guard `if (block === "") continue;` handles the leading empty chunk that `split(/\n{2,}/)` yields when the input begins with blank lines. Behaviour verified against both the LICENSE (leading whitespace only, no empty leading block) and a hypothetical leading-blank-line input (filtered correctly).
2. **Link interceptor**: hash-only hrefs bypass the handler via an early `return` from the per-anchor forEach callback (`src/main.ts:259`); the LICENSE regex accepts `LICENSE`, `./LICENSE`, `/LICENSE`, `LICENSE.md` (case-insensitive); unhandled hrefs fall through to `console.debug`. No attached click handler on hash anchors means they preserve native behaviour — in practice the README has no `#` anchors so this is dead code today but correct defensively.
3. **`openUrl(...).catch` now surfaces a toast** (`src/main.ts:262–266`): `ToastContainer.show("error", "Link konnte nicht geöffnet werden")` uses an imported symbol (`src/main.ts:17`) with the correct `ToastLevel`; `openUrl` is imported from `@tauri-apps/plugin-opener` (`src/main.ts:37`); the German error string is consistent with the app's UX voice.
6. **`new Marked({ gfm, breaks })`** (`src/utils/markdown.ts:3`): matches `class Marked { constructor(...args: MarkedExtension[]) }` (marked.d.ts:591) where `MarkedExtension` exposes both `gfm?: boolean` and `breaks?: boolean`. The `md.parse(src, { async: false })` return narrowing via `typeof html !== "string"` is defensive for the union type; correct.
7. **`@font-face` `src:` two-format list** (`src/styles/aurora.css:8–17`): `url(...) format("woff2-variations"), url(...) format("woff2")` restored; both point at the same file so browsers that reject the unknown `woff2-variations` hint fall back to the generic `woff2` format. Matches the approved analysis.
8. **Italic face deferred** — the analysis cycle-2 addendum (subsection 3) documents and defends the trade-off; the CSS still declares `.md-body em { font-style: italic; }` and lets the browser synthesise oblique at the WebKit/Chromium standard ~12° slant. This is an analysis-level decision, not a code defect.

## Additional checks

- **XSS**: `content.innerHTML = renderMarkdown(markdown)` is only entered in the non-plaintext branch. The only caller that reaches the non-plaintext branch is `showMarkdownPopup("README", README_TEXT)` (`src/main.ts:317`), i.e. a build-time literal from `src/utils/app-texts.ts`. Marked 12 escapes HTML by default (no `sanitize`/`html` overrides in `markdown.ts`). All three untrusted or verbatim sources (LICENSE, Versionshistorie from DB, any future non-markdown string) explicitly route through the plaintext branch, which writes via `textContent` only. XSS surface is closed.
- **Navigation**: `e.preventDefault()` runs before any branch decision for non-hash hrefs. External URLs go through `openUrl`; the LICENSE self-link closes the overlay and re-enters via the same entry point. No code path leaves the webview anchoring to a non-existent URL.
- **Recursive `showMarkdownPopup` call** for LICENSE: the outer overlay is removed before the inner call (`src/main.ts:268–269`), so only one popup is stacked at any time. The inner popup's plaintext branch does not register any link handler, so there is no listener leak from the recursion.
- **Event-listener per anchor** vs. a single delegated listener: the README renders ~6 anchors (`| Runtime | [Tauri v2](…)`, …), so the per-anchor attachment cost is trivial. When the overlay is removed, the anchors are garbage-collected with their handlers. No leak risk.
- **TypeScript strict**: `a as HTMLAnchorElement` is a narrowing cast (could use `querySelectorAll<HTMLAnchorElement>("a[href]")` for a tighter type but the current form is correct and idiomatic). `tsc --noEmit` passes.
- **CSS specificity**: `.md-body.plaintext p { margin: 0 0 var(--spacing-2); white-space: pre-wrap; }` has the same specificity (0,0,2,1) as `.md-body p { margin: 0 0 var(--spacing-3); }` but appears later in source order, so the plaintext margin wins. Intentional and correct.
- **Font-feature settings** `"cv11", "ss01", "ss03"` at `src/styles/layout.css:17` are valid Inter OpenType features (straight ä/ö/ü, stylistic set for single-storey a, tabular alt). No runtime cost; ignored by system fallback fonts during FOIT/FOUT. Safe.
- **Monospace token**: `--font-family-mono: ui-monospace, "SF Mono", "Consolas", "Roboto Mono", "Cascadia Mono", "Courier New", monospace;` at `src/styles/aurora.css:94` — the `ui-monospace` keyword is supported in Safari and iOS WKWebView (Tauri macOS target) and is silently ignored in Chromium/WebView2 (Tauri Windows target), which correctly falls through to `"Consolas"`. Token is now token-consistent with `.batch-log` (`src/styles/components.css:2938`) and `.md-body code/pre`.
- **Unicode range**: `U+1E00–1EFF` (Latin Extended Additional) is now included alongside the prior Latin-Extended ranges. This covers the GPL-3.0 English text plus German umlauts/ß and is a superset of the ranges the app actually renders; the `@font-face` scope is appropriate.
- **Versionshistorie rendering**: `lines.join("\n")` (single newlines) plus plaintext mode means the whole list renders as a single `<p>` whose `textContent` preserves newlines through `white-space: pre-wrap`. This matches the pre-refactor `<pre>`-tag behaviour (list items on their own visual lines) — intentional, not a regression.
- **`setAttribute("aria-label", "Schließen")`** on the close button, and `role="dialog"` / `aria-modal="true"` / `aria-label={title}` on the dialog: good for screen-reader discoverability; German close label matches the project convention.
- **`.gitignore`** additions (`apple/`, `*.dmg`, `*.msi`, `*.AppImage`, `*.flatpak`): unrelated release-artifact hygiene; harmless to the task under review.

## Findings

Code review passed. No findings.

## Verdict

PASS
