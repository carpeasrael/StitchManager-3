# Claude Code Review

- Project: StitchManager (Tauri v2 + TypeScript + Rust + SQLite)
- Reviewer: Claude CLI reviewer 1 (code review)
- Cycle: 4
- Date: 2026-04-19
- Scope: uncommitted diff (`git diff` + `git diff --cached`) and the untracked files that belong to this task (`src/utils/markdown.ts`, `src/assets/fonts/InterVariable.woff2`, `src/assets/fonts/OFL.txt`).
- Prior-cycle finding under examination: cycle 3 reported that Inter was only applied on `.app-layout`, so dialogs appended to `document.body` fell back to the system default. The fix for cycle 4 is the added `html, body { font-family: var(--font-family); }` declaration in `src/styles.css`.

---

## Verification of the cycle 3 fix

### (a) Does `html, body { font-family: var(--font-family) }` conflict with `.app-layout { font-family: var(--font-family) }`?

No conflict.

- Both declarations resolve to the identical token value. Even if one rule "wins" on specificity, the computed `font-family` is byte-identical, so there is no possibility of divergent rendering.
- The `html, body` selector has specificity `0,0,0,1` each (so `body` effectively `0,0,0,1`); `.app-layout` has specificity `0,0,1,0`. The class rule is more specific, but it only applies inside the main layout subtree. Dialogs and toasts appended to `document.body` (outside `.app-layout`) now inherit from `body`, which is exactly the intent.
- CSS ordering is preserved: `src/styles.css` imports `aurora.css` → `layout.css` → `components.css` first, and the `html, body` block sits after all `@import` rules, which satisfies the CSS spec's rule that `@import` must precede other rules.
- The `@font-face` declaration is the first rule inside `aurora.css`, which is valid (only `@charset` must precede `@font-face`, and no `@charset` is in use).

### (b) Are all dialog-mounted surfaces now correctly inheriting Inter?

Yes. The new `body { font-family: var(--font-family) }` rule is inherited by every descendant of `<body>`, which includes:

- `.dialog-overlay` elements appended via `document.body.appendChild(overlay)` (confirmed for `showMarkdownPopup`, `showInfoDialog`, and every `*Dialog.ts` that uses the same pattern).
- Toast containers.
- Any future popover/overlay created with `document.body.appendChild(...)`.

I spot-checked the dialog components that use `document.body.appendChild` (AiPreviewDialog, AiResultDialog, BatchDialog, ConfirmDialog, InputDialog, SettingsDialog, ImageViewerDialog, PrintPreviewDialog, ProjectListDialog, ManufacturingDialog, ImportPreviewDialog, HelpDialog, FolderDialog, EditDialog, DocumentViewer, ImagePreviewDialog, SmartFolderDialog, FolderMoveDialog, PatternUploadDialog). None of them set an opposing `font-family`, so they all inherit Inter via the new `body` rule.

Form controls (`<input>`, `<button>`, `<textarea>`, `<select>`) still need explicit `font-family: var(--font-family)` because native form controls do not inherit font-family on some platforms; the existing rules in `components.css` that target those elements are preserved, and the `.mfg-input` change from `inherit` → `var(--font-family)` is the correct direction.

### (c) Anything else in the diff

I examined every changed line for correctness, security, type safety, architecture, performance, edge cases, and conventions. No findings.

---

## Examined files and rationale

1. `.gitignore` — adds `apple/`, `*.dmg`, `*.msi`, `*.AppImage`, `*.flatpak`. Harmless; prevents accidentally committing release artifacts. No security or build implications.
2. `package.json` / `package-lock.json` — adds `marked@^12.0.2` (MIT, resolved to 12.0.2). Integrity hash present. License-compatible with GPL-3.0 distribution. Node engine `>= 18` satisfied by the project's tooling.
3. `src/utils/markdown.ts` — small wrapper around `new Marked({ gfm: true, breaks: false })`. The explicit `async: false` in `md.parse(src, { async: false })` matches the Marked v12 API which returns `string | Promise<string>` on the type side, and the guard (`if (typeof html !== "string") throw`) correctly narrows at runtime without a `as string` cast. Marked escapes HTML by default (`escape$1` in the compiled code), so no XSS surface is introduced.
4. `src/main.ts`
   - `showTextPopup` → `showMarkdownPopup(title, markdown, options)` — signature change propagated to all three call sites (README, LICENSE, version history). README uses the Markdown branch; LICENSE and version history use the `{ plaintext: true }` branch (XSS-safe via `textContent`).
   - Plaintext branch correctly splits on `\n{2,}`, filters empty blocks, and creates `<p>` elements via `textContent`, which is XSS-safe for GPL-3.0 text (which contains `<`, `>`, and `&` in the patent clause) and for DB-sourced version history.
   - Markdown branch: `content.innerHTML = renderMarkdown(markdown)` is gated to the bundled README string literal, which is a build-time constant in `app-texts.ts`. No user-controlled data path reaches this branch.
   - Link interception: `querySelectorAll("a[href]")` + delegated click handlers. `e.preventDefault()` is called unconditionally, so unhandled schemes (including `javascript:`) cannot navigate. Only `https?:` and `mailto:` flow through `openUrl()`, and only the exact relative path `LICENSE` / `./LICENSE` / `LICENSE.md` (case-insensitive) opens the license dialog. Hash anchors pass through unchanged (handled by `#` early return). The regex anchors (`^`) prevent bypasses like `myhttps:evil`.
   - `overlay.remove()` before opening the follow-up license dialog prevents overlay stacking.
   - New `closeX.setAttribute("aria-label", "Schließen")` improves accessibility; German string is correct.
   - Escape key support is inherited via the existing `dialog-dismiss` CustomEvent mechanism in the global Escape handler (main.ts:1088). The overlay correctly listens for this event.
5. `src/styles.css` — new `html, body { font-family: var(--font-family); }` rule. Correct fix for the cycle 3 finding (see section above).
6. `src/styles/aurora.css`
   - `@font-face` with `src: url("../assets/fonts/InterVariable.woff2")` — path resolves correctly from `src/styles/aurora.css` to `src/assets/fonts/InterVariable.woff2` (file exists, 184 KB). `format("woff2-variations")` first with `format("woff2")` fallback is the standard variable-font pattern. `unicode-range` covers Latin, Latin-Extended-A/B, Latin-Extended-Additional, general punctuation, currency, letterlike symbols, and arrows — sufficient for the German UI and the README/LICENSE content.
   - `font-display: swap` prevents FOIT.
   - `--font-family` and `--font-family-mono` updated to put Inter first and use `ui-monospace` as the system-monospace leader. Both include reasonable cross-platform fallback chains.
7. `src/styles/components.css`
   - `.text-popup-content` rewritten to shared typography; removed the hard-coded `"Consolas", "Monaco", "Courier New", monospace` stack.
   - New `.md-body *` rules use existing Aurora tokens (`--color-border-light`, `--color-muted`, `--color-accent`, `--color-surface`, `--color-bg`, `--color-border`, `--radius-sm`, `--spacing-*`, `--font-family-mono`, `--font-size-body`, `--font-size-caption`). I verified all referenced tokens exist in both `hell` and `dunkel` themes in `aurora.css`. No hard-coded colors.
   - `.md-body.plaintext p { margin: 0 0 var(--spacing-2); white-space: pre-wrap; }` correctly preserves line breaks inside each paragraph while producing real paragraph spacing between blocks.
   - `.batch-log` changed from `font-family: monospace` to `font-family: var(--font-family-mono)` — now consistent with `.help-keys` and uses the token stack.
   - `.mfg-input` changed from `font-family: inherit` to `font-family: var(--font-family)` — correct, because native form controls do not reliably inherit font-family on all platforms.
8. `src/styles/layout.css` — adds Inter's UI feature settings (`cv11, ss01, ss03`) and standard macOS/Linux font-smoothing hints plus `text-rendering: optimizeLegibility` on `.app-layout`. These are additive; they do not conflict with the `html, body` rule because font-feature-settings/font-smoothing are not controlled by `font-family`.
9. `src/assets/fonts/OFL.txt` — SIL OFL 1.1 license text present (4380 bytes), satisfying the OFL's distribution clause.

---

## Findings

Code review passed. No findings.

---

## Verdict

PASS
