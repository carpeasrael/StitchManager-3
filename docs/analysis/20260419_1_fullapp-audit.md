# Full-App Audit — 2026-04-19

> Phase 1 analysis document for a full-codebase audit of StitchManager
> against the four review dimensions defined in CLAUDE.md:
> **Security · Performance · Usability · Design Consistency**.
>
> Source review reports (full text per finding):
> - `docs/reviews/fullapp_20260419_claude_review_security.md`
> - `docs/reviews/fullapp_20260419_claude_review_performance.md`
> - `docs/reviews/fullapp_20260419_claude_review_usability.md`
> - `docs/reviews/fullapp_20260419_claude_review_design.md`

---

## 1. Problem description

A holistic audit of the StitchManager desktop app (Tauri v2 + TypeScript + Rust + SQLite) was requested. Four independent review agents — one per dimension defined in `CLAUDE.md` — examined the entire codebase and reported back. The acceptance gate (zero findings per dimension) **failed** in all four dimensions:

| Dimension | Findings | Critical | High | Medium | Low |
|---|---:|---:|---:|---:|---:|
| Security | 19 | 0 | 6 | 7 | 6 |
| Performance | 23 | 4 | 9 | 8 | 2 |
| Usability | 30 | 4 | 7 | 9 | 10 |
| Design Consistency | 26 | 2 | 6 | 11 | 7 |
| **Total** | **98** | **10** | **28** | **35** | **25** |

The top-line risks are:

- **Security** — a chain of Tauri commands (`attach_file` / `open_attachment` / `delete_attachment` / `convert_file`) trusts caller- or DB-supplied paths without containment checks, which after restoring a malicious backup turns the attachment workflow into an arbitrary-file-write/delete/launch surface. A hand-rolled HTML sanitizer feeds `innerHTML`. CSP keeps `script-src blob:` and `style-src 'unsafe-inline'`. `tauri-plugin-sql` is registered without a matching capability (latent unrestricted-SQL surface).
- **Performance** — `AppState.get()` deep-copies on every read across 237 call sites; per-row UPDATEs outside transactions in mass-import paths; `Sidebar` does a full DOM rebuild + recursive CTE on every selection; FTS5 update triggers fire on every `updated_at` touch.
- **Usability** — widespread ASCII transliterations of umlauts (`Schliessen`, `loeschen`, `auswaehlen`); three full-screen viewers without focus traps or modal ARIA; native `confirm()` / `prompt()` for destructive actions; long-running batch operations cannot be cancelled.
- **Design Consistency** — undefined CSS variables with hard-coded hex fallbacks; two parallel button class systems where one is undefined; three different dialog close-button class names; 100+ inline `style="…"` assignments bypassing the design system; theme parity broken by hard-coded badge colors.

---

## 2. Affected components

### Backend (Rust)
- `src-tauri/src/commands/files.rs` — attachment commands, sanitizer, batched thumbnails, FTS probe
- `src-tauri/src/commands/convert.rs` — output directory containment
- `src-tauri/src/commands/scanner.rs` — `mass_import`, `import_files`, `watcher_auto_import` per-row UPDATE loops
- `src-tauri/src/commands/folders.rs` — `delete_folder` sequential thumbnail unlink, recursive CTE for counts
- `src-tauri/src/commands/backup.rs` — `import_library`, `relink_batch`, `import_metadata_json`, `archive_files_batch`
- `src-tauri/src/commands/viewer.rs` — `read_file_bytes` allow-list
- `src-tauri/src/commands/print.rs` — Windows PowerShell command construction
- `src-tauri/src/commands/ai.rs` — prompt construction (injection), `ai_url` scheme validation
- `src-tauri/src/commands/migration.rs` — 2stitch hash interpolation
- `src-tauri/src/commands/statistics.rs` — dashboard 9-query stack
- `src-tauri/src/services/file_watcher.rs` — unbounded mpsc channel
- `src-tauri/src/services/ai_client.rs` — bearer-token over plain HTTP
- `src-tauri/src/parsers/{pes,dst,jef,vp3}.rs` — double-walk + per-segment vec allocation
- `src-tauri/src/db/migrations.rs` — missing indices, FTS triggers without `WHEN`
- `src-tauri/src/db/queries.rs` — 42-column `FILE_SELECT` for list view
- `src-tauri/src/lib.rs` — `tauri-plugin-sql` registration without capability
- `src-tauri/tauri.conf.json` — CSP `script-src blob:` and `style-src 'unsafe-inline'`
- `src-tauri/capabilities/default.json` — missing `sql:*` capability mismatch

### Frontend (TypeScript)
- `src/state/AppState.ts` — deep-copy on every read
- `src/components/FileList.ts` — full re-render + thumb cache eviction on `files` change
- `src/components/Sidebar.ts` — full DOM rebuild on selection; loadCounts on every folders mutation
- `src/components/MetadataPanel.ts` — XSS sink (`innerHTML`); 8-roundtrip metadata fetch; per-keystroke checkDirty; ad-hoc dialog without focus trap
- `src/components/Toast.ts` — no manual dismiss; uniform 4 s lifetime; max-5 cap drops unread errors
- `src/components/{DocumentViewer,ImageViewerDialog,PrintPreviewDialog}.ts` — missing focus trap and modal ARIA
- `src/components/Splitter.ts` — no keyboard support, no ARIA, no width persistence
- `src/components/{FolderDialog,FolderMoveDialog,SmartFolderDialog,ImportPreviewDialog}.ts` — orphan `btn`/`btn-primary` classes; orphan `.dialog-close-btn`
- `src/components/{Manufacturing,ProjectList,Edit,PatternUpload,Settings}Dialog.ts` — inline styles, mixed close-button classes, native `confirm`/`prompt`
- `src/components/AiResultDialog.ts` — non-standard footer button order
- `src/components/Dashboard.ts` — non-clickable stat cards
- `src/components/EditDialog.ts` — destructive transforms with no preview
- `src/main.ts` — 7 native `confirm()`/`prompt()` calls; drop-zone offered without folder selection
- `src/shortcuts.ts` — advertised Ctrl+K not implemented; missing common shortcuts (Ctrl+A, Ctrl+N, ?)
- `src/utils/app-texts.ts` — README mentions Ctrl+K but it's unbound
- `src/styles/aurora.css` — missing tokens (`--color-text-muted`, `--radius-sm`, `--color-bg-hover`, `--color-danger*`, `--color-accent-rgb`, font-mono, z-index scale)
- `src/styles/components.css` — undefined-var fallbacks; hard-coded badge palette; magic z-indices; off-scale font sizes and spacings
- `src/styles/layout.css` — asymmetric sidebar/right padding

---

## 3. Root cause / rationale

The findings cluster into a small number of underlying root causes:

### R1 — Trust model on filesystem paths is "validate `..` only"
The backend repeatedly relies on `validate_no_traversal` (rejects `..` components) to sanitise paths, then writes/reads/launches the result. That check is necessary but not sufficient: it does not enforce that the resolved path lives under an allow-listed root (library root, attachment dir, USB mount). Because `file_attachments.file_path` and `embroidery_files.filepath` are persisted DB strings, any code path that can write to those tables — including `restore_backup` (user-picked ZIP) and `import_library` — can stage absolute paths the rest of the app then trusts. This is the structural cause of the four High security findings (`delete_attachment`, `open_attachment`, `attach_file`, `convert_file`).

### R2 — Hand-rolled abstractions where vetted libraries exist
- HTML sanitization is scanner-based and Rust-side, then assigned to `innerHTML` on the frontend (Rust crate `ammonia` is the canonical fix).
- AI prompts are assembled by string concatenation of untrusted metadata.
- The Windows print path interpolates a path into a PowerShell `-Command` string.

Each is a footgun where a maintained library or a well-known pattern (separate `Command::args`, delimited prompt segments) eliminates the class of bugs.

### R3 — `AppState.get()` over-defensive copy semantics
`AppState.get()` always deep-copies arrays + spreads each object, on the assumption that callers might mutate. With 237 call sites on hot paths (`Sidebar.render`, `FileList.handleClick`, per-keystroke checks), every UI interaction allocates thousands of objects. The `getRef()` escape hatch already exists but is rarely used.

### R4 — Per-row writes outside transactions
`mass_import`, `import_files`, `watcher_auto_import`, `import_metadata_json`, `archive_files_batch`, `unarchive_files_batch`, `relink_batch` all loop with auto-committing UPDATEs. Each WAL fsync is ~3 ms on macOS APFS — for 10K files this is 30 seconds of pure fsync cost.

### R5 — Render-on-every-state-change without diffing
`FileList` and `Sidebar` subscribe to `appState.on("files"|"folders", () => this.render())` and rebuild the entire DOM (and event listeners). `loadMoreFiles` writes a new array → renders → re-fetches all visible thumbnails over IPC. Selection changes that should be a CSS class toggle become full DOM rebuilds.

### R6 — Drift between Aurora design system and component implementations
`aurora.css` defines a clean token system. Component CSS and TS have drifted away from it through five mechanisms:
1. References to **undefined** CSS variables that silently fall back to hard-coded colors (`--color-text-muted`, `--color-bg-hover`, `--color-danger`, `--color-accent-rgb`, `--radius-sm`).
2. **Two parallel button class systems** (`.btn`/`.btn-primary` vs `.dialog-btn`) where the first is entirely undefined and used in 4 dialogs.
3. **Three close-button class names** (`.dialog-close`, `.dialog-close-btn`, `.dv-close-btn`) and three more bespoke variants, each with different visuals.
4. **100+ inline `el.style.*`** assignments in components — even when they reference Aurora tokens, the rule lives in TS and bypasses the cascade and `[data-theme="dunkel"]` overrides.
5. **Hard-coded Tailwind-era hex palettes** for badges, status colors, star ratings — no dark-mode override on several.

### R7 — Frontend bypasses its own dialog system for important interactions
19 call sites use native `confirm()` / `prompt()` for destructive actions (delete file/folder, purge trash, delete custom field, delete material/supplier/product/license/inspection) **and** primary input flows (collection name, format conversion, machine selection). These dialogs are unstyled, not theme-aware, not focus-trapped within the app, locale-dependent, and `prompt()` provides no validation feedback at all.

### R8 — German UI written without umlauts
~80 user-facing strings across 22 components use ASCII transliterations (`Schliessen`, `loeschen`, `auswaehlen`, `hinzufuegen`, `Groesse`, `Hoehe`, `Ueberlappung`, etc.). The TS files are saved as UTF-8 and umlauts work in the same files elsewhere — the transliterations are a deliberate stylistic choice that breaks German orthography.

### R9 — Modal accessibility and focus-management gaps
Three full-screen viewers (`DocumentViewer`, `ImageViewerDialog`, `PrintPreviewDialog`) don't call `trapFocus()`. The ad-hoc modal in `MetadataPanel.showAttachmentTypeSelector` has no header, no footer, no focus trap, no Escape handler. The `Splitter` has no keyboard support at all.

### R10 — No persisted user preferences for layout
`Splitter` widths are written to CSS variables on the document element only; never persisted. Every restart resets to the defaults.

---

## 4. Proposed approach

The 98 findings should be tackled in **five waves**, ordered by risk and by how each wave unblocks the next.

### Wave 1 — Security hardening (blocking all other work)
Goal: close the file-path trust chain and remove the latent SQL surface.

- Enforce containment checks (canonicalise + ancestor-prefix match) for `attach_file`, `open_attachment`, `delete_attachment`, `convert_file`, `relink_batch`, `import_library`. Build the expected ancestor from `library_root` / `<library>/.stichman/attachments/<id>/` / USB mount.
- Add an extension allowlist for `attach_file` and `read_file_bytes` matching the supported viewer/attachment formats.
- Replace `sanitize_html` with `ammonia`. Re-render `instructions_html` via the sanitizer's serialisation (do not round-trip through `innerHTML`); consider a sandboxed `<iframe sandbox>` instead.
- Either remove the `tauri_plugin_sql::Builder::default()` registration, or scope it via a narrow capability and document the intent.
- Drop `script-src blob:` from CSP; move `script-src` to `'self'`. If `pdfjs-dist` needs blob workers, scope to `worker-src 'self' blob:` only. Drop `'unsafe-inline'` from `style-src` after moving the handful of inline styles to `components.css`.
- Validate `ai_url`: when `api_key` is set, reject schemes other than `https://` (allow `http://localhost`/`127.0.0.1`).
- Delimit untrusted segments in AI prompts (`<UNTRUSTED>…</UNTRUSTED>`); strip control characters; cap length.
- Pass paths via separate `Command::args` for PowerShell instead of `-Command` interpolation.
- Validate `library_root` on save (reject `/`, `~`, `/Users/<user>`, `C:\Users\<user>`).
- Add format validation on `update_file` fields (`pattern_date`, `purchase_link` scheme allowlist, length caps).
- Add `^[A-Fa-f0-9]{32,64}$` validation on the 2stitch `content_hash`.

Acceptance: all 19 security findings resolved; security reviewer re-runs with zero findings.

### Wave 2 — Performance hot paths
Goal: eliminate the four Critical performance findings + the highest-frequency High findings.

- **AppState** — make `get()` return the live reference (rename current `getRef` semantics). Audit the codebase for in-place mutations and convert them to immutable patterns. Keep an explicit `clone(key)` for the rare defensive case.
- **FileList** — emit a `files:append` event on `loadMoreFiles` instead of replacing the whole `files` array; keep `thumbCache` and `renderedCards` across appends. Treat selection changes as class-toggle updates, not re-renders.
- **Sidebar** — split into "selection update" (class toggle on existing `<li>`) and "structural update" (DOM rebuild). Cache `get_all_folder_file_counts` with TTL or invalidate only on actual file inserts/deletes.
- **DB write paths** — wrap every per-row UPDATE loop in `lock_db` + `unchecked_transaction` + batched UPDATE + `commit` (`mass_import`, `import_files`, `watcher_auto_import`, `import_metadata_json`, `archive_files_batch`, `unarchive_files_batch`, `relink_batch`).
- **FTS triggers** — add `OF column_list` to the AFTER UPDATE triggers so the FTS row only rebuilds when an indexed column actually changed.
- **Indices** — add `idx_embroidery_files_created_at`; replace `idx_file_thread_colors_file_id` with composite `(file_id, sort_order)`.
- **`get_thumbnails_batch`** — convert to async; use `tokio::fs::read` + `spawn_blocking` for base64; consider streaming individual `thumb:ready` events instead of one big response, or serve via `asset://` allowlisted thumbnail directory.
- **`pre_parse_file` loop** — parallelise with `rayon` (`par_iter`).
- **`delete_folder`** — parallel thumbnail unlink in `spawn_blocking`; emit progress event.
- **FTS5 existence probe** — cache the flag in `DbState` from schema version.
- **Tag SELECT-then-INSERT** — switch to `INSERT … ON CONFLICT … RETURNING id` (SQLite ≥ 3.35) or pre-load a name→id `HashMap`.
- **`FILE_LIST_SELECT`** — define a slim 10-column SELECT for paginated list view; reserve the 42-column `FILE_SELECT` for `getFile(id)`.
- **MetadataPanel** — collapse the 8-roundtrip `Promise.all` in `onSelectionChanged` into a single `get_file_with_metadata(file_id)` command; cache field DOM refs; drop redundant `JSON.stringify` in `checkDirty`.
- **File watcher** — bounded `sync_channel(N)`; cap HashSet at 500 entries before flushing.
- **PES/DST parsers** — pre-allocate Vecs from header counts; combine count + segment passes.
- **Dashboard stats** — combine 3 ai_status counts and 3 missing_metadata counts into single conditional-aggregate queries.

Acceptance: all 23 performance findings resolved; performance reviewer re-runs with zero findings.

### Wave 3 — Usability foundations
Goal: fix the four Critical usability findings and the High-severity items that block daily use.

- **German orthography** — sweep ~80 inline strings; replace ASCII transliterations with the correct umlaut/ß characters. Centralise user-visible strings in `src/utils/app-texts.ts` for future audits.
- **Focus trap + modal ARIA** — add `role="dialog"`, `aria-modal="true"`, `aria-label`, and `trapFocus(dialog)` to `DocumentViewer`, `ImageViewerDialog`, `PrintPreviewDialog`. Save and restore `previousFocus` in the close path.
- **Replace native dialogs** — build `ConfirmDialog` and `InputDialog` components (focus-trapped, theme-aware, Esc-closable, German copy). Replace all 19 `confirm()`/`prompt()` call sites. For "Format wählen" and "Maschine auswählen", use proper select/list dialogs.
- **Cancellable batch ops** — wire `BatchDialog`'s cancel button to a real `batch:cancel` event consumed by the backend (or `AbortController` signal). Drop the 2 s auto-close on errors. Surface a non-blocking status indicator in the StatusBar when the dialog is closed mid-operation.
- **Toast** — add a "×" close button; differentiate lifetimes by level (`success`/`info` 3-4 s, `error` persistent or 10 s+); add `role="alert"` for errors; prefer dropping non-error toasts when capped.
- **Backend error mapping** — frontend maps `AppError.code` → German user-friendly message (`Validation` → "Ungültige Eingabe: …", `NotFound` → "Datei oder Ordner nicht gefunden", `Io` → "Dateisystemfehler", `Database` → "Datenbankfehler — bitte erneut versuchen").
- **Splitter** — add `role="separator"`, `aria-orientation`, `aria-valuemin/max/now`, `tabindex="0"`, Arrow-key handlers, Home/End snaps. Persist final value to `settings` on `mouseup` (`sidebar_width`, `center_width`, `right_width`).
- **Shortcuts** — implement Ctrl+K (open AI on selected file) or remove the README claim. Add Ctrl+A (select all), Ctrl+N (new folder), `?` (help). Add a "Tastaturkürzel" entry in the burger menu opening a help dialog.
- **EditDialog** — preview-first flow: small canvas updates per transform; single explicit "Speichern" opens the file picker.
- **Du/Sie register** — pick one (recommend Du) and align all strings.
- Plus the medium-severity items: clickable Dashboard stat cards, `label.htmlFor` on form helpers, image `alt`, deprecated `execCommand` plan, empty/loading/error states for lists, `aria-live` on StatusBar/scan progress, keyboard equivalent for sidebar context menu, drop-zone gating when no folder selected.

Acceptance: all 30 usability findings resolved; usability reviewer re-runs with zero findings.

### Wave 4 — Design system reconciliation
Goal: re-converge on Aurora.

- **Tokens** — add the missing CSS variables to `aurora.css`: `--color-text-muted` (alias `--color-muted`), `--radius-sm`, `--color-bg-hover` (alias `--color-accent-10`), `--color-danger`/`--color-danger-bg` (alias `--color-error`/`--color-error-bg`), `--color-accent-rgb`, `--color-on-status`, `--color-canvas`, `--color-overlay-strong`/`--color-scrim-light`/`--color-scrim-strong`, `--font-family-mono`, `--font-size-micro` (or `--font-size-badge`), `--btn-size-sm`/`--btn-size-md`, z-index scale (`--z-base/popover/overlay/dialog/dialog-fullscreen/toast`).
- **Buttons** — replace `btn`/`btn-primary`/`btn-secondary`/`btn-small` with `dialog-btn`/`dialog-btn-primary`/`dialog-btn-secondary` family in 4 dialogs (`FolderDialog`, `FolderMoveDialog`, `SmartFolderDialog`, `ImportPreviewDialog`). Delete the orphan `.btn-small` rule or re-home it under the canonical family.
- **Close buttons** — standardise on `.dialog-close`. Remove `.dialog-close-btn`, `.dv-close-btn`, `.edit-close-btn`, `.text-popup-close-x`, `.image-preview-close`. One CSS rule, used in every modal.
- **Inline styles** — move every static inline `el.style.*` to a CSS class in `components.css`. Reserve inline `style` only for genuinely dynamic values (drag transforms, virtual-scroll positions, computed tree-depth padding).
- **Theme parity** — replace hard-coded Tailwind hex palette in `.folder-type-sewing_pattern`, `.mfg-badge-warn`, `.mfg-inv-status`, `.mfg-tt-diff-over`, `.star-rating .star.filled`, `.mfg-stock-warn` with Aurora tokens; ensure every color rule has a `[data-theme="dunkel"]` override.
- **Hard-coded `#fff` / `white`** — replace with `var(--color-on-status)` or `var(--color-canvas)`; tokenise viewer overlay colors.
- **Inputs** — collapse the five parallel input class systems (`.metadata-form-input`, `.settings-input`, `.pp-setting-input`, `.mfg-input`, `.tag-input`) into one `.input` with modifiers.
- **Sort buttons** — collapse `.sort-dir-btn` and `.search-sort-dir-btn` to one rule.
- **Dialog title** — pick `<h2 class="dialog-title">` for every dialog. Remove `.mfg-title`, `.pl-title`, `.dialog-edit-title`.
- **Close icon glyph** — standardise on `\u00D7`.
- **Status badge** — single `.status-badge` with `--in-progress`, `--completed` modifiers.
- **MetadataPanel ad-hoc dialog** — refactor `showAttachmentTypeSelector` to use the same scaffold as `EditDialog` / `AiPreviewDialog`.
- **`PatternUploadDialog` cancel** — add `dialog-btn-secondary`.
- **`ProjectListDialog` primary actions** — switch `dv-btn` → `dialog-btn dialog-btn-primary`/`secondary`.
- **`AiResultDialog` footer order** — Cancel (secondary) → Reject (danger) → Accept-all (secondary) → Accept (primary), or move "Alle akzeptieren" out of the footer.
- **Off-scale font sizes / paddings** — map every hard-coded `Npx` to the nearest Aurora token; add new tokens only when no existing level fits.
- **Magic z-index** — replace every literal with the new token scale; ensure toasts are highest.
- **Iconography** — pick one of (a) SVG/icon-font library, (b) all monochrome Unicode, (c) all color emoji. Mixing creates visual noise.
- **Layout** — make sidebar/right padding symmetric (`--spacing-3` on both).
- **Border weights** — use the same border token on top menu and bottom status bar.
- **`Component` base** — introduce a `Dialog` base class with `open()`/`close()`/`overlay`/`releaseFocusTrap`/Esc-handler/backdrop-click that all 18 dialog components extend; or factor a `createDialog(...)` helper.
- **`Splitter` width persistence** — covered in Wave 3 (overlaps with usability).

Acceptance: all 26 design-consistency findings resolved; design reviewer re-runs with zero findings.

### Wave 5 — Cross-dimension re-validation
Per CLAUDE.md, after **any** fix, all four reviewers must re-run cleanly in the **same** cycle. Waves 1–4 will likely interact (e.g. moving inline styles into CSS may invite XSS/DOM regressions; replacing `confirm()` introduces new dialogs that need focus-trap parity). After each wave's commit:

1. Run `npm run build` (TypeScript check + Vite build).
2. Run `cd src-tauri && cargo check` and `cargo test`.
3. Re-run the four review agents against the diff (per `CLAUDE.md` Phase 3).
4. Only when all four return zero findings **in the same cycle**, the wave is accepted.
5. If any reviewer reports new findings introduced by the wave, return to the implementation step for that wave and repeat from #1.

---

## 5. Recommended next steps for the user

The 98 findings exceed what a single PR or even a single sprint should bundle. Suggest:

1. **Open a meta-issue** ("Full-app audit 2026-04-19") referencing this analysis and the four review reports.
2. **Spawn one issue per wave** (5 issues), each linked to the meta-issue. Each wave-issue gets its own Phase 1 analysis (this document is the umbrella analysis; per-wave analyses can be brief).
3. **Tackle Wave 1 first** — the security findings are the highest blast-radius and several are exploit-ready under a hostile backup. Land Wave 1, re-run the four reviewers against the diff, commit, close.
4. Proceed through Waves 2–5 in order, with the same Phase 1 → 2 → 3 → 4 cycle each.
5. Once all five waves are merged with clean reviewer cycles, close the meta-issue and the audit ends.

---

## 6. Out of scope

- Behaviour outside the `src/`, `src-tauri/`, `index.html`, `vite.config.ts`, `tsconfig.json`, `package.json`, `Cargo.toml`, `tauri.conf.json`, `capabilities/default.json` set was not reviewed (e.g. CI workflows, release packaging, GitHub Actions).
- Dependency-upgrade audits (npm `audit`, `cargo audit`) were not run by the reviewers — recommend running them separately.
- No automated test was added or executed during the audit. Existing test runs (`cargo test`, `npm run build`) were not invoked.
- The German-language register (Du vs Sie) is a recommendation, not a finding — the user should confirm before mass-editing strings.

---

## 7. Stop condition

Per `CLAUDE.md` § 2.3, do **not** begin implementation until this analysis is reviewed and explicitly approved by the user. The user should also indicate:

- Whether to bundle the audit into 5 waves as proposed, or to triage a subset first (e.g. Critical+High only), or to open per-finding issues.
- Whether to keep Du or switch to Sie for the German rewrite.
- Whether the iconography decision (a/b/c) has a preferred answer.
- Whether `tauri-plugin-sql` should be removed entirely or scoped for a future use.
