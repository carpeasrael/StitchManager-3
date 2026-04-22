# Codex Code Review

Date: 2026-04-19
Reviewer: Codex CLI reviewer 1
Scope: PR #126 Phases 1-4, commit range a208f9b..3a6dbbd

1. `src-tauri/src/commands/statistics.rs:79` — major — The new dashboard aggregates for "Top Ordner" and "Speicherverbrauch" only join files whose `folder_id = f.id`. Phase 2 made folders hierarchical and the sidebar now counts descendants recursively, so any parent folder whose files live in subfolders is reported as empty or undercounted on the dashboard. That makes the new dashboard stats incorrect for nested trees introduced in the same PR.
2. `src/components/ImportPreviewDialog.ts:491` — major — After a successful import, the dialog sets `selectedFolderId` and refreshes files, but it never clears `selectedSmartFolderId`. `FileList.loadFiles()` explicitly ignores the folder filter whenever a smart folder is selected, so starting an import while a smart folder is active can leave the UI showing the old smart-folder result set instead of the folder that was just imported into.
3. `src/components/SmartFolderDialog.ts:184` — minor — "Aus aktuellem Filter uebernehmen" serializes the AI filter with `String(sp.aiAnalyzed)`. If the current search is "confirmed only" (`aiAnalyzed = true` and `aiConfirmed = true`), the dialog writes `"true"` instead of `"confirmed"`, and the saved smart folder broadens from confirmed files to all analyzed files. That is a behavior change in the stored filter.
4. `src/components/Sidebar.ts:103` — minor — The "Alle Ordner" row is marked selected whenever `selectedFolderId === null`. Smart-folder selection intentionally clears `selectedFolderId`, so selecting a smart folder now highlights both the smart folder and "Alle Ordner" at the same time, which misrepresents the active scope.

FAIL
