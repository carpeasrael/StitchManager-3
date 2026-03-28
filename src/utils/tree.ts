import type { Folder } from "../types/index";

export interface FolderTreeNode {
  folder: Folder;
  children: FolderTreeNode[];
  depth: number;
}

export interface FlatTreeEntry {
  folder: Folder;
  depth: number;
  hasChildren: boolean;
}

/** Build a tree from a flat folder list using parentId. */
export function buildFolderTree(folders: Folder[]): FolderTreeNode[] {
  const byParent = new Map<number | null, Folder[]>();
  for (const f of folders) {
    const key = f.parentId;
    if (!byParent.has(key)) byParent.set(key, []);
    byParent.get(key)!.push(f);
  }

  function buildLevel(parentId: number | null, depth: number): FolderTreeNode[] {
    const children = byParent.get(parentId) ?? [];
    return children
      .sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name))
      .map((f) => ({
        folder: f,
        children: buildLevel(f.id, depth + 1),
        depth,
      }));
  }

  return buildLevel(null, 0);
}

/** Flatten visible tree nodes based on expanded folder IDs. */
export function flattenVisibleTree(
  tree: FolderTreeNode[],
  expandedIds: Set<number>
): FlatTreeEntry[] {
  const result: FlatTreeEntry[] = [];

  function walk(nodes: FolderTreeNode[]): void {
    for (const node of nodes) {
      result.push({
        folder: node.folder,
        depth: node.depth,
        hasChildren: node.children.length > 0,
      });
      if (node.children.length > 0 && expandedIds.has(node.folder.id)) {
        walk(node.children);
      }
    }
  }

  walk(tree);
  return result;
}

/** Get all descendant IDs of a folder (for disabling in move dialog). */
export function getDescendantIds(folders: Folder[], folderId: number): Set<number> {
  const result = new Set<number>();
  function collect(parentId: number): void {
    for (const f of folders) {
      if (f.parentId === parentId && !result.has(f.id)) {
        result.add(f.id);
        collect(f.id);
      }
    }
  }
  collect(folderId);
  return result;
}
