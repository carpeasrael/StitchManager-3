import { Component } from "./Component";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { ToastContainer } from "./Toast";
import { FolderDialog } from "./FolderDialog";
import { FolderMoveDialog } from "./FolderMoveDialog";
import { SmartFolderDialog } from "./SmartFolderDialog";
import { buildFolderTree, flattenVisibleTree } from "../utils/tree";
import * as FolderService from "../services/FolderService";
import * as ProjectService from "../services/ProjectService";
import * as SmartFolderService from "../services/SmartFolderService";
import type { Collection, SmartFolder } from "../types";

export class Sidebar extends Component {
  private folderCounts = new Map<number, number>();
  private collections: Collection[] = [];
  private smartFoldersList: SmartFolder[] = [];
  private dragSrcId: number | null = null;
  private reordering = false;
  private contextMenu: HTMLElement | null = null;
  private contextMenuCloseHandler: ((e: Event) => void) | null = null;

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("folders", () => this.loadCounts())
    );
    this.subscribe(
      appState.on("selectedFolderId", () => this.render())
    );
    this.subscribe(
      appState.on("expandedFolderIds", () => this.render())
    );
    this.subscribe(
      appState.on("smartFolders", () => {
        this.smartFoldersList = appState.get("smartFolders");
        this.render();
      })
    );
    this.subscribe(
      appState.on("selectedSmartFolderId", () => this.render())
    );
    this.loadFolders();
    this.loadCollections();
    this.loadSmartFolders();
  }

  private async loadFolders(): Promise<void> {
    try {
      const folders = await FolderService.getAll();
      appState.set("folders", folders);
      await this.loadCounts();
    } catch (e) {
      console.warn("Failed to load folders:", e);
      ToastContainer.show("error", "Ordner konnten nicht geladen werden");
    }
  }

  private async loadCounts(): Promise<void> {
    try {
      const counts = await FolderService.getAllFileCounts();
      this.folderCounts.clear();
      for (const [id, count] of Object.entries(counts)) {
        this.folderCounts.set(Number(id), count);
      }
    } catch {
      // Fall back to zero counts on error
    }
    this.render();
  }

  render(): void {
    const folders = appState.get("folders");
    const selectedId = appState.get("selectedFolderId");
    const expandedIds = new Set(appState.get("expandedFolderIds"));

    this.el.innerHTML = "";

    const header = document.createElement("div");
    header.className = "sidebar-header";

    const title = document.createElement("span");
    title.className = "sidebar-title";
    title.textContent = "Ordner";
    header.appendChild(title);

    const addBtn = document.createElement("button");
    addBtn.className = "sidebar-add-btn";
    addBtn.textContent = "+";
    addBtn.title = "Neuer Ordner";
    addBtn.setAttribute("aria-label", "Neuer Ordner");
    addBtn.addEventListener("click", () => this.createFolder());
    header.appendChild(addBtn);

    this.el.appendChild(header);

    const list = document.createElement("ul");
    list.className = "folder-list";

    // "Alle Ordner" entry — shows files across all folders
    const allLi = document.createElement("li");
    allLi.className = "folder-item";
    if (selectedId === null) {
      allLi.classList.add("selected");
    }
    const allName = document.createElement("span");
    allName.className = "folder-name";
    allName.textContent = "Alle Ordner";
    const allCount = document.createElement("span");
    allCount.className = "folder-count";
    let totalCount = 0;
    // Sum only root-level counts (recursive counts already include descendants)
    const tree = buildFolderTree(folders);
    for (const node of tree) {
      totalCount += this.folderCounts.get(node.folder.id) ?? 0;
    }
    allCount.textContent = String(totalCount);
    allLi.appendChild(allName);
    allLi.appendChild(allCount);
    allLi.addEventListener("click", () => {
      appState.set("selectedFileIds", []);
      appState.set("selectedFileId", null);
      appState.set("selectedFolderId", null);
      appState.set("selectedSmartFolderId", null);
    });
    list.appendChild(allLi);

    if (folders.length === 0) {
      const empty = document.createElement("div");
      empty.className = "sidebar-empty";
      empty.textContent = "Keine Ordner vorhanden";
      this.el.appendChild(list);
      this.el.appendChild(empty);
      return;
    }

    // Build tree and flatten visible nodes
    const visible = flattenVisibleTree(tree, expandedIds);

    for (const entry of visible) {
      const { folder, depth, hasChildren } = entry;
      const li = document.createElement("li");
      li.className = "folder-item";
      li.setAttribute("draggable", "true");
      li.dataset.folderId = String(folder.id);
      li.dataset.depth = String(depth);
      li.style.paddingLeft = `${depth * 16 + 8}px`;
      if (folder.id === selectedId) {
        li.classList.add("selected");
      }

      // Expand/collapse toggle
      const toggle = document.createElement("span");
      toggle.className = "folder-toggle";
      if (hasChildren) {
        toggle.textContent = "\u25B6";
        if (expandedIds.has(folder.id)) {
          toggle.classList.add("expanded");
        }
        toggle.addEventListener("click", (e) => {
          e.stopPropagation();
          this.toggleExpand(folder.id);
        });
      } else {
        toggle.classList.add("leaf");
        toggle.textContent = "\u25B6";
      }
      li.appendChild(toggle);

      const nameSpan = document.createElement("span");
      nameSpan.className = "folder-name";
      nameSpan.textContent = folder.name;

      // Folder type badge
      if (folder.folderType) {
        const badge = document.createElement("span");
        badge.className = `folder-type-badge folder-type-${folder.folderType}`;
        const typeLabels: Record<string, { short: string; full: string }> = {
          embroidery: { short: "S", full: "Stickmuster" },
          sewing_pattern: { short: "N", full: "Schnittmuster" },
          mixed: { short: "G", full: "Gemischt" },
        };
        const info = typeLabels[folder.folderType] ?? { short: "?", full: folder.folderType };
        badge.textContent = info.short;
        badge.title = info.full;
        nameSpan.appendChild(badge);
      }

      const countSpan = document.createElement("span");
      countSpan.className = "folder-count";
      countSpan.textContent = String(this.folderCounts.get(folder.id) ?? 0);

      const deleteBtn = document.createElement("button");
      deleteBtn.className = "folder-delete-btn";
      deleteBtn.textContent = "\u00D7";
      deleteBtn.title = "Ordner l\u00F6schen";
      deleteBtn.setAttribute("aria-label", `Ordner ${folder.name} l\u00F6schen`);
      deleteBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        this.deleteFolder(folder.id);
      });

      li.appendChild(nameSpan);
      li.appendChild(countSpan);
      li.appendChild(deleteBtn);

      li.addEventListener("click", () => {
        appState.set("selectedSmartFolderId", null);
        if (folder.id === appState.get("selectedFolderId")) {
          appState.set("selectedFileIds", []);
          appState.set("selectedFileId", null);
          appState.set("selectedFolderId", null);
        } else {
          appState.set("selectedFileIds", []);
          appState.set("selectedFileId", null);
          appState.set("selectedFolderId", folder.id);
        }
      });

      // Context menu
      li.addEventListener("contextmenu", (e) => {
        e.preventDefault();
        e.stopPropagation();
        this.showContextMenu(e.clientX, e.clientY, folder.id);
      });

      // Drag-and-drop: sibling reorder (drag between) or reparent (drag onto)
      li.addEventListener("dragstart", (e) => {
        this.dragSrcId = folder.id;
        li.classList.add("dragging");
        e.dataTransfer?.setData("text/plain", String(folder.id));
      });
      li.addEventListener("dragend", () => {
        this.dragSrcId = null;
        li.classList.remove("dragging");
        list.querySelectorAll(".drag-over, .drop-into").forEach((el) => {
          el.classList.remove("drag-over", "drop-into");
        });
      });
      li.addEventListener("dragover", (e) => {
        e.preventDefault();
        if (this.dragSrcId === null || this.dragSrcId === folder.id) return;
        // Determine drop position: top half = reorder, bottom half = reparent
        const rect = li.getBoundingClientRect();
        const midY = rect.top + rect.height / 2;
        if (e.clientY < midY) {
          li.classList.add("drag-over");
          li.classList.remove("drop-into");
        } else {
          li.classList.add("drop-into");
          li.classList.remove("drag-over");
        }
      });
      li.addEventListener("dragleave", () => {
        li.classList.remove("drag-over", "drop-into");
      });
      li.addEventListener("drop", (e) => {
        e.preventDefault();
        const wasDropInto = li.classList.contains("drop-into");
        li.classList.remove("drag-over", "drop-into");
        if (this.dragSrcId === null || this.dragSrcId === folder.id) return;

        if (wasDropInto) {
          // Reparent: make dragged folder a child of this folder
          this.moveFolderTo(this.dragSrcId, folder.id);
        } else {
          // Sibling reorder — only if same parent
          const srcFolder = folders.find((f) => f.id === this.dragSrcId);
          if (srcFolder && srcFolder.parentId === folder.parentId) {
            this.reorderFolder(this.dragSrcId, folder.id);
          }
          // Non-siblings in reorder zone: ignore (use drop-into or context menu to reparent)
        }
      });

      // Keyboard reorder: Alt+Up/Down among siblings
      li.tabIndex = 0;
      li.addEventListener("keydown", (e) => {
        if (!e.altKey) return;
        const siblings = visible.filter((v) => v.folder.parentId === folder.parentId);
        const idx = siblings.findIndex((s) => s.folder.id === folder.id);
        if (e.key === "ArrowUp" && idx > 0) {
          e.preventDefault();
          this.reorderFolder(folder.id, siblings[idx - 1].folder.id);
        } else if (e.key === "ArrowDown" && idx < siblings.length - 1) {
          e.preventDefault();
          this.reorderFolder(folder.id, siblings[idx + 1].folder.id);
        }
      });

      list.appendChild(li);
    }

    this.el.appendChild(list);

    // Collections section
    this.renderCollections();

    // Smart folders section
    this.renderSmartFolders();
  }

  private toggleExpand(folderId: number): void {
    const current = appState.get("expandedFolderIds");
    const idx = current.indexOf(folderId);
    if (idx >= 0) {
      appState.set("expandedFolderIds", current.filter((id) => id !== folderId));
    } else {
      appState.set("expandedFolderIds", [...current, folderId]);
    }
  }

  private showContextMenu(x: number, y: number, folderId: number): void {
    this.closeContextMenu();

    const menu = document.createElement("div");
    menu.className = "folder-context-menu";
    document.body.appendChild(menu);

    // Build content first so we can measure dimensions
    const moveItem = document.createElement("div");
    moveItem.className = "folder-context-menu-item";
    moveItem.textContent = "Verschieben nach\u2026";
    moveItem.addEventListener("click", () => {
      this.closeContextMenu();
      FolderMoveDialog.open(folderId);
    });
    menu.appendChild(moveItem);

    // Clamp to viewport bounds
    const menuRect = menu.getBoundingClientRect();
    const clampedX = Math.min(x, window.innerWidth - menuRect.width - 4);
    const clampedY = Math.min(y, window.innerHeight - menuRect.height - 4);
    menu.style.left = `${Math.max(0, clampedX)}px`;
    menu.style.top = `${Math.max(0, clampedY)}px`;

    this.contextMenu = menu;

    // Close on click or Escape
    this.contextMenuCloseHandler = (e: Event) => {
      if (e.type === "keydown" && (e as KeyboardEvent).key !== "Escape") return;
      this.closeContextMenu();
    };
    requestAnimationFrame(() => {
      if (this.contextMenuCloseHandler) {
        document.addEventListener("click", this.contextMenuCloseHandler);
        document.addEventListener("keydown", this.contextMenuCloseHandler);
      }
    });
  }

  private closeContextMenu(): void {
    if (this.contextMenuCloseHandler) {
      document.removeEventListener("click", this.contextMenuCloseHandler);
      document.removeEventListener("keydown", this.contextMenuCloseHandler);
      this.contextMenuCloseHandler = null;
    }
    if (this.contextMenu) {
      this.contextMenu.remove();
      this.contextMenu = null;
    }
  }

  private async moveFolderTo(srcId: number, targetParentId: number | null): Promise<void> {
    if (this.reordering) return;
    this.reordering = true;
    try {
      await FolderService.moveFolder(srcId, targetParentId);
      const updated = await FolderService.getAll();
      appState.set("folders", updated);
      // Auto-expand the target parent so moved folder is visible
      if (targetParentId !== null) {
        const expanded = appState.get("expandedFolderIds");
        if (!expanded.includes(targetParentId)) {
          appState.set("expandedFolderIds", [...expanded, targetParentId]);
        }
      }
    } catch (e) {
      const msg =
        e && typeof e === "object" && "message" in e
          ? (e as { message: string }).message
          : String(e);
      ToastContainer.show("error", `Verschieben fehlgeschlagen: ${msg}`);
    } finally {
      this.reordering = false;
    }
  }

  private async loadCollections(): Promise<void> {
    try {
      this.collections = await ProjectService.getCollections();
      this.render();
    } catch {
      // Silently continue without collections
    }
  }

  private async loadSmartFolders(): Promise<void> {
    try {
      const sf = await SmartFolderService.getAll();
      appState.set("smartFolders", sf);
    } catch {
      // Silently continue
    }
  }

  private renderSmartFolders(): void {
    const section = document.createElement("div");
    section.className = "sidebar-smart-folders";

    const header = document.createElement("div");
    header.className = "sidebar-header";
    const title = document.createElement("span");
    title.className = "sidebar-title";
    title.textContent = "Intelligente Ordner";
    header.appendChild(title);

    const addBtn = document.createElement("button");
    addBtn.className = "sidebar-add-btn";
    addBtn.textContent = "+";
    addBtn.title = "Neuer intelligenter Ordner";
    addBtn.setAttribute("aria-label", "Neuer intelligenter Ordner");
    addBtn.addEventListener("click", () => SmartFolderDialog.open());
    header.appendChild(addBtn);
    section.appendChild(header);

    const selectedSmartId = appState.get("selectedSmartFolderId");

    if (this.smartFoldersList.length > 0) {
      const list = document.createElement("ul");
      list.className = "folder-list";

      for (const sf of this.smartFoldersList) {
        const li = document.createElement("li");
        li.className = "folder-item smart-folder-item";
        if (sf.id === selectedSmartId) {
          li.classList.add("selected");
        }

        const iconSpan = document.createElement("span");
        iconSpan.className = "smart-folder-icon";
        iconSpan.textContent = sf.icon;
        li.appendChild(iconSpan);

        const nameSpan = document.createElement("span");
        nameSpan.className = "folder-name";
        nameSpan.textContent = sf.name;
        li.appendChild(nameSpan);

        const delBtn = document.createElement("button");
        delBtn.className = "folder-delete-btn";
        delBtn.textContent = "\u00D7";
        delBtn.title = "Loeschen";
        delBtn.setAttribute("aria-label", `${sf.name} loeschen`);
        delBtn.addEventListener("click", async (e) => {
          e.stopPropagation();
          try {
            await SmartFolderService.remove(sf.id);
            const updated = await SmartFolderService.getAll();
            appState.set("smartFolders", updated);
            if (appState.get("selectedSmartFolderId") === sf.id) {
              appState.set("selectedSmartFolderId", null);
            }
          } catch {
            ToastContainer.show("error", "Konnte nicht geloescht werden");
          }
        });
        li.appendChild(delBtn);

        li.addEventListener("click", () => {
          if (sf.id === appState.get("selectedSmartFolderId")) {
            appState.set("selectedSmartFolderId", null);
          } else {
            // Mutual exclusion: clear folder selection when selecting smart folder
            appState.set("selectedFileIds", []);
            appState.set("selectedFileId", null);
            appState.set("selectedFolderId", null);
            appState.set("selectedSmartFolderId", sf.id);
          }
        });

        list.appendChild(li);
      }
      section.appendChild(list);
    }

    this.el.appendChild(section);
  }

  private renderCollections(): void {
    const section = document.createElement("div");
    section.className = "sidebar-collections";

    const header = document.createElement("div");
    header.className = "sidebar-header";

    const title = document.createElement("span");
    title.className = "sidebar-title";
    title.textContent = "Sammlungen";
    header.appendChild(title);

    const addBtn = document.createElement("button");
    addBtn.className = "sidebar-add-btn";
    addBtn.textContent = "+";
    addBtn.title = "Neue Sammlung";
    addBtn.setAttribute("aria-label", "Neue Sammlung");
    addBtn.addEventListener("click", async () => {
      const name = prompt("Sammlungsname:");
      if (!name?.trim()) return;
      try {
        await ProjectService.createCollection(name.trim());
        await this.loadCollections();
      } catch {
        ToastContainer.show("error", "Sammlung konnte nicht erstellt werden");
      }
    });
    header.appendChild(addBtn);

    const uploadBtn = document.createElement("button");
    uploadBtn.className = "sidebar-add-btn";
    uploadBtn.textContent = "\u2191";
    uploadBtn.title = "Schnittmuster hochladen";
    uploadBtn.setAttribute("aria-label", "Schnittmuster hochladen");
    uploadBtn.addEventListener("click", () => {
      EventBus.emit("pattern:upload");
    });
    header.appendChild(uploadBtn);
    section.appendChild(header);

    if (this.collections.length > 0) {
      const list = document.createElement("ul");
      list.className = "folder-list";

      for (const col of this.collections) {
        const li = document.createElement("li");
        li.className = "folder-item collection-item";

        const nameSpan = document.createElement("span");
        nameSpan.className = "folder-name";
        nameSpan.textContent = col.name;
        li.appendChild(nameSpan);

        const delBtn = document.createElement("button");
        delBtn.className = "folder-delete-btn";
        delBtn.textContent = "\u00D7";
        delBtn.title = "Sammlung loeschen";
        delBtn.setAttribute("aria-label", `Sammlung ${col.name} loeschen`);
        delBtn.addEventListener("click", async (e) => {
          e.stopPropagation();
          try {
            await ProjectService.deleteCollection(col.id);
            await this.loadCollections();
          } catch {
            ToastContainer.show("error", "Sammlung konnte nicht geloescht werden");
          }
        });
        li.appendChild(delBtn);

        li.addEventListener("click", () => {
          EventBus.emit("collection:selected", { collectionId: col.id, collectionName: col.name });
        });

        list.appendChild(li);
      }

      section.appendChild(list);
    }

    this.el.appendChild(section);
  }

  private async reorderFolder(srcId: number, targetId: number): Promise<void> {
    if (this.reordering) return;
    this.reordering = true;
    try {
      await this.reorderFolderInner(srcId, targetId);
    } finally {
      this.reordering = false;
    }
  }

  private async reorderFolderInner(srcId: number, targetId: number): Promise<void> {
    const folders = appState.get("folders");
    const src = folders.find((f) => f.id === srcId);
    const target = folders.find((f) => f.id === targetId);
    if (!src || !target) return;

    // Only reorder among siblings (same parentId)
    if (src.parentId !== target.parentId) return;

    const siblings = folders.filter((f) => f.parentId === target.parentId);
    const srcIdx = siblings.findIndex((f) => f.id === srcId);
    const targetIdx = siblings.findIndex((f) => f.id === targetId);
    if (srcIdx === -1 || targetIdx === -1) return;

    // Reorder siblings
    const reordered = [...siblings];
    const [moved] = reordered.splice(srcIdx, 1);
    reordered.splice(targetIdx, 0, moved);

    const orders: [number, number][] = reordered.map((f, i) => [f.id, (i + 1) * 10]);

    try {
      await FolderService.updateSortOrders(orders);
      const updated = await FolderService.getAll();
      appState.set("folders", updated);
    } catch (e) {
      console.warn("Failed to reorder folders:", e);
      ToastContainer.show("error", "Ordner konnten nicht umsortiert werden");
    }
  }

  private deleteFolder(folderId: number): void {
    // Select the folder first so the central handler knows which one to delete
    appState.set("selectedFileIds", []);
    appState.set("selectedFileId", null);
    appState.set("selectedFolderId", folderId);
    EventBus.emit("toolbar:delete-folder");
  }

  private createFolder(): void {
    FolderDialog.open();
  }

  destroy(): void {
    this.closeContextMenu();
    super.destroy();
  }
}
