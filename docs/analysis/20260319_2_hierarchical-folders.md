# Analysis: Hierarchical Folders (Issue #126, Phase 2)

**Date:** 2026-03-19
**Issue:** #126 — Proposals 3 & 4

---

## Problem Description

### Proposal 3: Tree Rendering in Sidebar

The Sidebar (`src/components/Sidebar.ts`) renders all folders as a flat `<ul>` list (lines 76–212), iterating `folders` from `AppState` with a simple `for (const folder of folders)` loop. Although the database schema already supports hierarchical relationships via `folders.parent_id` (confirmed in `src-tauri/src/db/migrations.rs` line 170: `parent_id INTEGER REFERENCES folders(id) ON DELETE CASCADE`), the frontend completely ignores parent-child relationships:

- No indentation or visual nesting is applied.
- No expand/collapse toggle exists for parent folders.
- No expand/collapse state is tracked in `AppState` or anywhere else.
- File counts are flat per-folder: `get_all_folder_file_counts` (folders.rs lines 258–273) counts only direct files per `folder_id` — a parent never shows the sum of its own + descendant files.
- Drag-and-drop reorder (`reorderFolder`, lines 311–343) treats all folders as a flat list of siblings — `sort_order` is globally scoped, not per-parent.

Users with 20+ folders cannot organize them into logical groups (e.g., "Weihnachten" containing "Sterne" and "Engel").

### Proposal 4: Move / Reparent Folders

Once a folder is created with a `parent_id`, there is no way to change it. The `update_folder` command (folders.rs lines 92–149) accepts only `name` and `folder_type` — it has no `parent_id` parameter. There is no context menu in the Sidebar (confirmed: no `contextmenu` event handlers exist anywhere in `src/`). Users cannot move a folder into or out of a parent after creation.

---

## Affected Components

### Frontend

| File | Role | Changes Required |
|------|------|-----------------|
| `src/components/Sidebar.ts` | Flat folder list rendering, drag-and-drop reorder | Tree rendering, expand/collapse, tree-aware drag-and-drop, context menu for "Move to..." |
| `src/services/FolderService.ts` | Tauri invoke wrappers for folder ops | New `moveFolder()` wrapper, updated `updateSortOrders()` signature |
| `src/state/AppState.ts` | Reactive state singleton | New `expandedFolderIds: Set<number>` (or `number[]`) state key |
| `src/types/index.ts` | `Folder` interface, `State` interface | Add `expandedFolderIds` to `State` |
| `src/styles/components.css` | Sidebar and folder styles (lines 386–513) | Tree indent styles, expand/collapse toggle, context menu styles, drop-zone indicators |
| `src/components/FolderDialog.ts` | Folder creation dialog with flat parent dropdown | Update parent dropdown to show indented tree structure |

### Backend

| File | Role | Changes Required |
|------|------|-----------------|
| `src-tauri/src/commands/folders.rs` | Folder CRUD, sort orders, file counts | `move_folder` command, updated `create_folder` for sibling-scoped sort_order, recursive `get_all_folder_file_counts` |
| `src-tauri/src/lib.rs` | Tauri command registration (line 118+) | Register `move_folder` command |
| `src-tauri/src/db/models.rs` | `Folder` struct (lines 16–27) | No change needed — `parent_id: Option<i64>` already exists |
| `src-tauri/src/db/migrations.rs` | Schema definition | No migration needed — `parent_id` FK and index already exist (lines 170–175) |

---

## Root Cause / Rationale

1. **No tree builder:** The frontend receives a flat `Vec<Folder>` ordered by `sort_order, name` (folders.rs line 29) and renders it verbatim. No code exists to transform the flat list into a tree structure using `parentId`.

2. **Global sort_order:** `create_folder` computes `MAX(sort_order)` globally (line 71: `SELECT COALESCE(MAX(sort_order), 0) FROM folders`) rather than scoping to siblings (`WHERE parent_id IS ?` or `WHERE parent_id = ?`). `update_folder_sort_orders` applies changes globally without considering parent grouping. `reorderFolder` in Sidebar.ts (lines 321–343) reorders the entire flat array.

3. **Flat file counts:** `get_all_folder_file_counts` (folders.rs lines 258–273) uses `GROUP BY folder_id` — a parent folder sees only files directly in it, never descendant totals. The recursive CTE pattern already exists in `delete_folder` (lines 157–164) but isn't used for counts.

4. **No `parent_id` in `update_folder`:** The command signature (line 93–98) only accepts `name` and `folder_type`. Moving a folder requires changing its `parent_id`, which is not exposed.

5. **No context menu infrastructure:** The Sidebar has no `contextmenu` event handler. A right-click menu is needed for the "Move to..." action and future folder-level operations.

6. **No expand/collapse state:** `State` (types/index.ts lines 668–681) has no field for tracking which folders are expanded/collapsed. The UI has no toggle mechanism.

---

## Proposed Approach

### Step 1: Add `expandedFolderIds` to AppState

**Files:** `src/types/index.ts`, `src/state/AppState.ts`

- Add `expandedFolderIds: number[]` to the `State` interface (after line 669).
- Initialize to `[]` in `AppState` initial state (after line 6 of AppState.ts).
- By default all folders start collapsed; root-level folders (parentId = null) are always visible.

### Step 2: Build tree utility function

**File:** New utility in `src/utils/tree.ts` (or inline in Sidebar)

Create a pure function to transform the flat folder list into a tree:

```typescript
interface FolderTreeNode {
  folder: Folder;
  children: FolderTreeNode[];
  depth: number;
}

function buildFolderTree(folders: Folder[]): FolderTreeNode[]
```

- Group folders by `parentId`.
- Recursively build tree starting from roots (`parentId === null`).
- Sort siblings by `sortOrder` then `name` (matching backend ORDER BY).
- Return array of root-level `FolderTreeNode`s.

Also create a `flattenVisibleTree()` function that takes the tree and a `Set<number>` of expanded IDs, and returns a flat array of `{ folder, depth }` for only the visible nodes (root nodes + children of expanded nodes).

### Step 3: Refactor Sidebar rendering to tree

**File:** `src/components/Sidebar.ts`

- Subscribe to `expandedFolderIds` state changes (in constructor, alongside existing subscriptions).
- In `render()`, replace the flat `for (const folder of folders)` loop (lines 111–212) with:
  1. Call `buildFolderTree(folders)`.
  2. Call `flattenVisibleTree(tree, expandedIds)`.
  3. Render each visible node with:
     - Indent via `padding-left: (depth * 16 + 8)px` (or a CSS class per depth).
     - An expand/collapse toggle chevron (`>` / `v`) for nodes that have children.
     - Click on chevron toggles the folder's ID in `expandedFolderIds` state.
     - Existing click, drag, delete behavior preserved.

- "Alle Ordner" remains at the top, always visible, showing total count across all folders.

### Step 4: Recursive file counts

**File:** `src-tauri/src/commands/folders.rs`

Replace `get_all_folder_file_counts` (lines 258–273) with a recursive count query:

```sql
WITH RECURSIVE folder_tree(id, root_id) AS (
    SELECT id, id FROM folders
    UNION ALL
    SELECT f.id, ft.root_id FROM folders f JOIN folder_tree ft ON f.parent_id = ft.id
)
SELECT ft.root_id AS folder_id, COUNT(*) AS cnt
FROM embroidery_files e
JOIN folder_tree ft ON e.folder_id = ft.id
WHERE e.deleted_at IS NULL
GROUP BY ft.root_id
```

This gives each folder a count including all descendants. The CTE pattern is already proven in `delete_folder` (lines 157–164).

Keep `get_folder_file_count` (single folder) unchanged — it only needs the direct count for its own display.

### Step 5: Sibling-scoped sort_order in `create_folder`

**File:** `src-tauri/src/commands/folders.rs`

Change the `MAX(sort_order)` query in `create_folder` (line 71) to scope by parent:

```sql
-- For root folders:
SELECT COALESCE(MAX(sort_order), 0) FROM folders WHERE parent_id IS NULL
-- For child folders:
SELECT COALESCE(MAX(sort_order), 0) FROM folders WHERE parent_id = ?1
```

This ensures new folders are appended at the end of their sibling group, not at the global end.

### Step 6: Tree-aware drag-and-drop reorder

**File:** `src/components/Sidebar.ts`

Modify drag-and-drop to respect the tree:

- **Sibling reorder:** Dragging between siblings at the same level reorders within that parent group. `reorderFolderInner` must filter to siblings (same `parentId`) before computing new sort orders.
- **Reparent via drop-on-folder:** Dragging onto a folder (not between) makes the dragged folder a child. This triggers the `move_folder` backend command.
- Visual indicators:
  - Drag between: `border-top` (existing `.drag-over` style, line 511) for reorder.
  - Drag onto: `background` highlight for reparent.
- The `updateSortOrders` call after reorder must send only sibling orders, not all folders.

### Step 7: Backend `move_folder` command

**File:** `src-tauri/src/commands/folders.rs`

Add a new Tauri command:

```rust
#[tauri::command]
pub fn move_folder(
    db: State<'_, DbState>,
    folder_id: i64,
    new_parent_id: Option<i64>,
) -> Result<Folder, AppError>
```

Implementation:
1. **Existence check:** Verify `folder_id` exists.
2. **Circular reference check:** If `new_parent_id` is Some, walk ancestors using a recursive CTE to ensure `folder_id` is not an ancestor of `new_parent_id`. Use:
   ```sql
   WITH RECURSIVE ancestors(id) AS (
       SELECT parent_id FROM folders WHERE id = ?1  -- new_parent_id
       UNION ALL
       SELECT f.parent_id FROM folders f JOIN ancestors a ON f.id = a.id
   )
   SELECT 1 FROM ancestors WHERE id = ?2  -- folder_id
   ```
   If a row is found, return `AppError::Validation("Zirkulaere Referenz: Ordner kann nicht in einen eigenen Unterordner verschoben werden")`.
3. **Self-reference check:** `folder_id` cannot equal `new_parent_id`.
4. **Update `parent_id`:** `UPDATE folders SET parent_id = ?1, updated_at = datetime('now') WHERE id = ?2`.
5. **Recalculate sort_order:** Place the moved folder at the end of the new parent's children: `SELECT COALESCE(MAX(sort_order), 0) FROM folders WHERE parent_id IS ?1` (handling NULL for root).
6. Return updated folder.

Register in `src-tauri/src/lib.rs` invoke_handler (after line 125).

### Step 8: Frontend `moveFolder` service wrapper

**File:** `src/services/FolderService.ts`

Add:
```typescript
export async function moveFolder(
  folderId: number,
  newParentId: number | null
): Promise<Folder> {
  return invoke<Folder>("move_folder", { folderId, newParentId: newParentId ?? null });
}
```

### Step 9: Context menu with "Move to..." action

**File:** `src/components/Sidebar.ts`

Add a `contextmenu` event handler to each folder `<li>`:
1. On right-click, show a custom context menu at cursor position.
2. Menu items: "Verschieben nach..." (Move to...).
3. Clicking "Verschieben nach..." opens a folder-tree picker dialog.

**Folder-tree picker dialog:** New static method `FolderMoveDialog.open(folderId: number)`:
- Modal overlay with a rendered folder tree (reuse `buildFolderTree` from Step 2).
- The current folder and all its descendants are disabled/grayed (cannot move into self or descendants).
- A "Stammverzeichnis" (Root) option at the top to move to root level.
- On selection, call `FolderService.moveFolder(folderId, selectedParentId)`, reload folders, and show toast.

### Step 10: Update FolderDialog parent selector

**File:** `src/components/FolderDialog.ts`

The parent folder dropdown (lines 124–156) currently renders a flat `<select>` with all folders. Update to show indented folder names reflecting the tree hierarchy:

- Build tree using `buildFolderTree()`.
- Flatten tree in DFS order.
- Set `<option>` text to `"  ".repeat(depth) + folder.name` for visual indentation.

### Step 11: CSS for tree rendering

**File:** `src/styles/components.css`

Add styles after line 513:

```css
/* Tree indent */
.folder-item[data-depth="1"] { padding-left: calc(var(--spacing-3) + 16px); }
.folder-item[data-depth="2"] { padding-left: calc(var(--spacing-3) + 32px); }
.folder-item[data-depth="3"] { padding-left: calc(var(--spacing-3) + 48px); }

/* Expand/collapse toggle */
.folder-toggle {
  width: 16px;
  font-size: 10px;
  text-align: center;
  cursor: pointer;
  color: var(--color-muted);
  flex-shrink: 0;
  transition: transform 0.15s;
}
.folder-toggle.expanded {
  transform: rotate(90deg);
}
.folder-toggle.leaf {
  visibility: hidden;
}

/* Reparent drop zone (drag onto) */
.folder-item.drop-into {
  background: var(--color-accent-20);
  border: 1px dashed var(--color-accent);
}

/* Context menu */
.folder-context-menu {
  position: fixed;
  z-index: 1000;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-card);
  box-shadow: var(--shadow-dropdown);
  min-width: 160px;
  padding: var(--spacing-1) 0;
}
.folder-context-menu-item {
  padding: var(--spacing-2) var(--spacing-3);
  cursor: pointer;
  font-size: var(--font-size-body);
  color: var(--color-text);
}
.folder-context-menu-item:hover {
  background: var(--color-accent-10);
}
```

### Step 12: Persist expand/collapse state

Expand/collapse state is stored in `AppState.expandedFolderIds` (in-memory). This is sufficient since it resets on app restart — users expect folders to start collapsed. If persistence is desired later, it can be saved to the `settings` table.

### Step 13: Update `update_folder_sort_orders` for sibling scope

**File:** `src-tauri/src/commands/folders.rs`

The existing `update_folder_sort_orders` command (lines 203–240) does not need structural changes — it already accepts arbitrary `(folder_id, sort_order)` pairs. However, the frontend `reorderFolderInner` (Sidebar.ts lines 321–343) must be updated to send only sibling folder IDs (those with the same `parentId`), not all folders globally.

### Step 14: Backend tests

**File:** `src-tauri/src/commands/folders.rs` (tests module)

Add tests:
- `test_move_folder_basic` — move a root folder under another folder, verify parent_id changes.
- `test_move_folder_circular_reference` — attempt to move a parent into its own child, verify rejection.
- `test_move_folder_self_reference` — attempt to set parent_id = id, verify rejection.
- `test_move_folder_to_root` — move a child folder to root (parent_id = NULL), verify.
- `test_recursive_file_count` — create parent+child folders with files, verify parent count includes child files.

---

## Summary of Changes

| Category | Count | Details |
|----------|-------|---------|
| New backend command | 1 | `move_folder` |
| Modified backend commands | 2 | `create_folder` (sibling sort_order), `get_all_folder_file_counts` (recursive CTE) |
| New frontend utility | 1 | `src/utils/tree.ts` (buildFolderTree, flattenVisibleTree) |
| New frontend service wrapper | 1 | `FolderService.moveFolder()` |
| New UI component | 1 | Folder move dialog (can be in `FolderDialog.ts` or new file) |
| Modified UI components | 2 | `Sidebar.ts` (tree + context menu), `FolderDialog.ts` (indented parent selector) |
| State changes | 1 | `expandedFolderIds` in AppState |
| CSS additions | ~40 lines | Tree indent, toggle, context menu, drop-into indicator |
| New backend tests | 5 | Circular ref, self ref, move, root move, recursive count |
| Schema migration | 0 | `parent_id` FK and index already exist |
