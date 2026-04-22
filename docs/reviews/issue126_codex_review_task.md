# Codex Task-Resolution Review

Date: 2026-04-19
Reviewer: Codex CLI reviewer 2
Task reference: PR #126 Phases 1-4

1. Smart-folder selection is still additive with the user's current ad-hoc filters instead of acting as a saved view on its own. In [src/components/FileList.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/components/FileList.ts#L62) and again in [src/components/FileList.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/components/FileList.ts#L101), the code parses `sf.filterJson` and merges it into the live `searchParams` object (`searchParams = { ...searchParams, ...parsed }`). The approved Phase 4 analysis called for loading the smart folder's saved filter and passing that as the active query; with the current merge behavior, any leftover search bar / chip / advanced-filter state silently narrows the smart folder result set, so saved smart folders are not reliable reusable views.

2. The watcher follow-up behavior from Phase 3 is still missing. The approved import-preview analysis required watcher auto-import to stay silent but surface a toast with a review action that can reopen the imported files for metadata cleanup. In [src/main.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/main.ts#L897), the implementation only shows plain text (`"Metadaten im Panel bearbeiten"`) and reloads counts; there is no link, callback, event, or preview-opening path. That leaves the "review retroactively" acceptance criterion unresolved.

3. Drag-and-drop still bypasses the preview dialog for small drops, which directly contradicts the unified-import requirement for Phase 3. In [src/main.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/main.ts#L1133), drops of 3 files or fewer still call `ScannerService.importFiles(...)` immediately and never open `ImportPreviewDialog`. The approved analysis required dropped files to go through preview instead of direct import, so this is still partial.

4. Smart folders are not editable from the UI, so the Phase 4 CRUD workflow is incomplete. The backend exposes `update_smart_folder`, but the frontend only offers create and delete: the sidebar renders a `+` button and delete button in [src/components/Sidebar.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/components/Sidebar.ts#L407), and [src/components/SmartFolderDialog.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/components/SmartFolderDialog.ts#L10) is create-only (`open(): void`, fixed "Neuer intelligenter Ordner" title, save path always calls `SmartFolderService.create(...)` at [src/components/SmartFolderDialog.ts](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src/components/SmartFolderDialog.ts#L234)). The approved analysis explicitly called for a create/edit smart-folder dialog and frontend CRUD.

5. Migration v26 does not match the approved default smart-folder set. The analysis doc specified presets for "Nicht analysiert", "5 Sterne", and "Kuerzlich importiert", but [src-tauri/src/db/migrations.rs](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/src-tauri/src/db/migrations.rs#L1353) seeds "Nicht analysiert", "5 Sterne", and "Favoriten" instead. That is a requirements mismatch against the documented Phase 4 plan.

Final verdict: FAIL
