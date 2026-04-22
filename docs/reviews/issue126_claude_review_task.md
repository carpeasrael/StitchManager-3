# Claude Task-Resolution Review

- Date: 2026-04-19
- Reviewer: Claude (task-resolution review)
- Task reference: PR #126 Phases 1–4 (commits a9e02f0 → 3a6dbbd)
- Scope: committed range `a208f9b..3a6dbbd` (not uncommitted changes)

## Method

Read issue #126 in full, reviewed all commit comments reporting phase completion, then read the diff for every phase commit across backend (Rust), frontend (TypeScript), DB migrations, services, and UI components. Cross-checked each of the eight proposals in the issue against the delivered code.

## Findings

The bulk of the eight proposals are implemented correctly. However, five concrete gaps were found where the acceptance criteria stated in issue #126 are not fully met by the implementation.

### 1. Phase 1 / Proposal #2 — Split per-folder file counts are missing

Issue requirement (verbatim):

> Split file counts: show `12 Stickdateien · 3 Schnittmuster` instead of just `15`

Implementation: `src/components/Sidebar.ts` renders a single numeric `folder-count` span per folder (`countSpan.textContent = String(this.folderCounts.get(folder.id) ?? 0)`) and a single-valued type badge (`S` / `N` / `G`). The backend `get_all_folder_file_counts` only returns a total via the recursive CTE — it does not split by `file_type` (embroidery vs sewing_pattern).

Impact: the "Stickdateien · Schnittmuster" split display demanded by Phase 1 Proposal #2 is not delivered. Users see only the total.

### 2. Phase 3 / Proposal #5 — Watcher Toast does not open the preview retroactively

Issue requirement (verbatim):

> For watcher auto-import: skip preview (maintain current silent behavior), but show a Toast with "3 neue Dateien importiert — Überprüfen" link that opens the preview retroactively.

Implementation: `src/main.ts` (the `"fs:auto-imported"` handler) shows a plain informational toast:

```
`${imported} neue Datei(en) importiert — Metadaten im Panel bearbeiten`
```

There is no clickable "Überprüfen" action and no path to open `ImportPreviewDialog` retroactively for the imported files. `ToastContainer.show()` is invoked with only `type` and `message` — no action callback is provided, and `Toast.ts` is not extended for linkable toasts.

Impact: the "Überprüfen" / review-after-the-fact UX explicitly called for in Phase 3 Proposal #5 is missing.

### 3. Phase 4 / Proposal #7 — Smart folder "edit" CRUD is not exposed in UI

Issue requirement (verbatim):

> CRUD: create from current filter state, edit, delete

Implementation status:
- Create: `SmartFolderDialog.open()` → backend `create_smart_folder` ✅
- Delete: sidebar per-row `×` button → `SmartFolderService.remove` ✅
- Edit: `update_smart_folder` backend command and `SmartFolderService.update` exist, but **no UI surface invokes them**. `SmartFolderDialog` has only a create path (no edit mode / no pre-populated form). Right-click / double-click / pencil-icon editing is absent on the sidebar smart-folder entries.

Grep confirmation: `SmartFolderService.update` has zero call sites in `src/`.

Impact: users cannot modify an existing smart folder's name, icon, or filter criteria without deleting and recreating it. The "edit" leg of CRUD is not delivered.

### 4. Phase 4 / Proposal #7 — Preset smart folder differs from spec

Issue requirement (verbatim):

> Provide preset smart folders on first run: "Nicht analysiert", "5 Sterne", "Kürzlich importiert"

Implementation (`src-tauri/src/db/migrations.rs`, `apply_v26`):

```
'Nicht analysiert', '🔬', '{"aiAnalyzed": false}', 10
'5 Sterne',        '⭐', '{"ratingMin": 5}',       20
'Favoriten',        '❤️','{"isFavorite": true}',   30
```

The third preset is "Favoriten" instead of "Kürzlich importiert". The specified third preset is not shipped, and there is no "Favoriten" preset in the issue text.

Impact: a first-run user does not see a "Kürzlich importiert" shortcut. This also means the date-based recency filter mode is not exercised by any preset.

### 5. Phase 4 / Proposal #8 — No Toolbar button for the Dashboard / "Bibliothek-Übersicht"

Issue requirement (verbatim):

> Add a "Bibliothek-Übersicht" (Library Overview) view accessible from the Toolbar
> Affected components:
>   …
>   `src/components/Toolbar.ts` — add dashboard button

Implementation: the enhanced dashboard is rendered whenever no folder and no smart folder is selected (`Dashboard.ts::checkVisibility`). No button was added to `Toolbar.ts` in this PR — the diff on `src/components/Toolbar.ts` only rewires `addFolder` and `scanFolder`. There is no new menu item or button to surface the "Bibliothek-Übersicht".

Impact: the view is discoverable only by deselecting/clicking "Alle Ordner" — the explicit Toolbar entry point asked for in Phase 4 Proposal #8 is not present.

## Phases/Proposals passing without findings

- Phase 1 #1 Native Folder Picker Dialog (FolderDialog with browse, parent dropdown, type selector) — implemented as specified.
- Phase 2 #3 Tree Rendering (`buildFolderTree`, `flattenVisibleTree`, expanded-state persistence in `AppState.expandedFolderIds`, recursive file counts via CTE, drag-into-reparent) — implemented.
- Phase 2 #4 Move / Reparent (`move_folder` command, recursive-CTE circular reference check, self-ref guard, sibling-scoped sort_order, `FolderMoveDialog` tree picker with disabled self/descendants, transaction wrap) — implemented.
- Phase 3 #5 Import Preview Dialog core (sortable checkbox list, summary bar, select-all/none, bulk-metadata fields, target folder select with "+ Neuer Ordner", `scan_only` backend with batch already-imported detection) — implemented (aside from Finding 2).
- Phase 3 #6 Unified Import for Stickmuster & Schnittmuster (BulkMetadata with `author` / `skill_level` applied only to sewing_pattern rows, rating range validation early-exit, conditional designer/difficulty fields in preview dialog) — implemented.
- Phase 4 #7 Smart folders: schema v26, backend CRUD commands, `SmartFolderService`, `selectedSmartFolderId` + mutual-exclusion state, filter JSON merged into `SearchParams` in both `loadFiles` and `loadMoreFiles`, extended `SearchParams` with `ratingMin`/`ratingMax`/`isFavorite` — implemented (aside from Findings 3 & 4).
- Phase 4 #8 Dashboard enrichment (files by type, AI status incl. confirmed, top folders, missing metadata counts, storage by folder, recent imports) — implemented (aside from Finding 5).

## Verdict

FAIL

Five requirements from issue #126 are not fully delivered:

1. Split per-folder counts (Stickdateien · Schnittmuster) — not rendered.
2. Watcher toast "Überprüfen" clickable link to re-open the import preview — not implemented.
3. Smart folder edit UI — backend ready, UI missing.
4. Preset smart folder "Kürzlich importiert" — replaced with "Favoriten".
5. Toolbar button for the Bibliothek-Übersicht dashboard — not added.
