# Code Review: Issue #125

## Findings

No findings.

Verified:
- Condition gate removed: "Projekt starten" now shows for all file types (not just sewing_pattern/PDF)
- Dropdown filters `p.status !== "completed" && p.status !== "archived"` — correct
- Uses `ProjectService.addFileToProject(projectId, fileId, "pattern")` — valid role per backend validation
- `INSERT OR IGNORE` in backend prevents duplicate link errors
- Dropdown loads asynchronously, disabled while loading, shows fallback on error/empty
- Dropdown resets after selection (`addSelect.value = ""`)
- No backend changes needed — all required commands already exist
- `npm run build` passes, `cargo test` 204/204 passes
