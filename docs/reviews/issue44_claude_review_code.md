Code review passed. No findings.

## Verification of Previously Reported Findings

All five previously identified issues have been properly resolved:

1. **Duplicate logic**: Sidebar.deleteFolder() delegates to EventBus ("toolbar:delete-folder"); single handler in main.ts. No duplication.
2. **Stale files**: main.ts handler clears files via `appState.set("files", [])` after folder deletion.
3. **Subfolder warning**: Confirmation message includes "und Unterordner" when `folders.some(f => f.parentId === folderId)` is true.
4. **Accessibility**: Delete button uses `opacity: 0` / `pointer-events: none` (hidden), revealed on hover/selected/focus-visible with `opacity: 1` / `pointer-events: auto`. Button has `aria-label`. Color changes to `--color-error` on hover/focus-visible.
5. **Stale counts**: Sidebar subscribes to `appState.on("folders", () => this.loadCounts())`, which calls `FolderService.getAllFileCounts()` and re-renders.

No new issues found in the reviewed files.
