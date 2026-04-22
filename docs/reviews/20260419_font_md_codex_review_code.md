# Codex Code Review

Date: 2026-04-22
Reviewer: Codex CLI reviewer 1
Scope: Review cycle `20260419_font_md`
Task: align design/font across app and render README/LICENSE as markdown preview

1. `major` — [src/main.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/main.ts:246), [src/utils/markdown.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/utils/markdown.ts:5): the new README popup writes `marked` output straight into `innerHTML`, but there is no follow-up handling for the links embedded in `README_TEXT`. The bundled README contains both external links (`https://...`) and a relative `[LICENSE](LICENSE)` link. In the popup these become live anchors inside the Tauri webview, so clicking them will navigate the app window to an external/unknown URL or a non-existent in-app route instead of opening safely via the opener plugin or mapping the relative LICENSE link back to the existing license dialog. That is a user-visible regression introduced by the markdown preview.

2. `major` — [src/main.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/main.ts:297): the LICENSE button still calls `showMarkdownPopup(..., { plaintext: true })`, so the LICENSE remains a wrapped plain-text dump rather than a markdown/document preview. The approved scope for this cycle was to render both README and LICENSE as markdown previews; with this branch only README changed behavior.

Validation notes: inspected `git diff` and `git diff --cached` (no staged changes), reviewed the approved analysis, and ran `npm run build` successfully.

FAIL
