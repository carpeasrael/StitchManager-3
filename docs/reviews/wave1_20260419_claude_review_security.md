# Wave 1 Security Review (Cycle 2) — 2026-04-19

## Summary
**PASS.** No new security findings. The two follow-up changes in
`src/components/MetadataPanel.ts` (the `extractBackendMessage` helper, the
save-handler now surfacing the backend message via `ToastContainer.show`,
and the `addAttachment` `open()` `filters:` allow-list) do not introduce
any new attack surface, and every original Wave 1 security fix is still
present in the uncommitted diff.

## Verification of original 19 findings
Spot-checked the diff for the major Wave 1 fixes — all present and unchanged
in this cycle:

- `src-tauri/src/commands/mod.rs` — `expand_home`, `canonicalize_or_self`,
  `ensure_under`, `library_root`, `validate_library_root`,
  `ATTACHMENT_EXTENSIONS`, `VIEWER_EXTENSIONS`, `lower_ext` all intact.
- `src-tauri/src/commands/files.rs` — `MAX_TEXT_FIELD`/`MAX_LINK_FIELD`
  caps, `is_valid_iso_date`, http(s)-only `purchase_link`, ammonia-backed
  `sanitize_html`, `attach_file` extension allow-list,
  `delete_attachment` containment check, `open_attachment` enforcing
  `ensure_under(...)?` + extension allow-list.
- `src-tauri/src/commands/ai.rs` — `validate_ai_url` (https-only when an
  API key is set, loopback exception), `<UNTRUSTED>` prompt fences,
  `sanitize_prompt_segment` stripping `<`, `>`, control chars, capping at
  512.
- `src-tauri/src/commands/settings.rs` — `library_root` value validated.
- `src-tauri/src/commands/backup.rs` — `relink_batch` validates both
  prefixes, checks each rewritten path is under `library_root`,
  `auto_purge_trash` clamps `retention_days` to `[1, 3650]`,
  `import_library` validates the new root.
- `src-tauri/src/commands/convert.rs`, `migration.rs`, `print.rs`,
  `viewer.rs`, `lib.rs`, `tauri.conf.json`, `Cargo.toml` — Wave 1 changes
  for findings #4, #6, #7, #9, #12, #15, #17 still present in this diff.
- Frontend defence-in-depth `sanitizeRichText` (DOMParser tree walker) at
  `src/components/MetadataPanel.ts` is still applied to
  `instrEditor.innerHTML`.

## Verification of follow-up changes

**`extractBackendMessage(e, fallback)`** — narrowing read of `e.message`
when `e` is an object with a string `message`, then `e.message` for
`Error` instances, otherwise the German fallback. No DOM access, no
`eval`, no template injection, no dangerous prototype walking; even with
a poisoned input the function falls through to the static fallback
string. No new surface.

**Save-handler toast** (`ToastContainer.show("error", msg)` in the catch
path) — verified `src/components/Toast.ts:65-67`: the message is
rendered via `textContent`, never `innerHTML`. There is no `innerHTML`
in `Toast.ts` at all. A backend Validation message containing
`<script>`, `<img onerror=...>`, or any other HTML would render as
literal characters. No XSS surface introduced. Backend
`AppError::Validation` payloads are German strings authored in the Rust
source (e.g. "Sprache zu lang", "Musterdatum muss im Format YYYY-MM-DD
vorliegen") — they do not echo arbitrary attacker-controlled content
verbatim, only field labels plus user-controlled values when needed,
and even those would be safely text-rendered.

**`addAttachment` `filters` array** — purely a UX filter on the Tauri
`open()` dialog. The backend `attach_file` command independently
rejects any extension outside `ATTACHMENT_EXTENSIONS` (verified at
`src-tauri/src/commands/files.rs` `ext_lower` allow-list check).
Bypassing the dialog filter (e.g. by typing a path or by dropping the
filter entirely on a misconfigured platform) still hits the backend
allow-list, so this is a pure ergonomic improvement with no weakening
of the security boundary. The frontend `extensions: ["pdf","png","jpg",
"jpeg","txt","md"]` array exactly matches the backend
`ATTACHMENT_EXTENSIONS` constant.

## New findings

No new findings.
