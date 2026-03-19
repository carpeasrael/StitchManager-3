import { Component } from "./Component";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { ToastContainer } from "./Toast";
import { FolderDialog } from "./FolderDialog";
import * as FolderService from "../services/FolderService";
import * as ProjectService from "../services/ProjectService";
import type { Collection } from "../types";

export class Sidebar extends Component {
  private folderCounts = new Map<number, number>();
  private collections: Collection[] = [];
  private dragSrcId: number | null = null;
  private reordering = false;

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("folders", () => this.loadCounts())
    );
    this.subscribe(
      appState.on("selectedFolderId", () => this.render())
    );
    this.loadFolders();
    this.loadCollections();
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
    for (const c of this.folderCounts.values()) totalCount += c;
    allCount.textContent = String(totalCount);
    allLi.appendChild(allName);
    allLi.appendChild(allCount);
    allLi.addEventListener("click", () => {
      appState.set("selectedFileIds", []);
      appState.set("selectedFileId", null);
      appState.set("selectedFolderId", null);
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

    for (const folder of folders) {
      const li = document.createElement("li");
      li.className = "folder-item";
      li.setAttribute("draggable", "true");
      li.dataset.folderId = String(folder.id);
      if (folder.id === selectedId) {
        li.classList.add("selected");
      }

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

      // Drag-and-drop reorder
      li.addEventListener("dragstart", (e) => {
        this.dragSrcId = folder.id;
        li.classList.add("dragging");
        e.dataTransfer?.setData("text/plain", String(folder.id));
      });
      li.addEventListener("dragend", () => {
        this.dragSrcId = null;
        li.classList.remove("dragging");
        list.querySelectorAll(".drag-over").forEach((el) => el.classList.remove("drag-over"));
      });
      li.addEventListener("dragover", (e) => {
        e.preventDefault();
        if (this.dragSrcId !== null && this.dragSrcId !== folder.id) {
          li.classList.add("drag-over");
        }
      });
      li.addEventListener("dragleave", () => {
        li.classList.remove("drag-over");
      });
      li.addEventListener("drop", (e) => {
        e.preventDefault();
        li.classList.remove("drag-over");
        if (this.dragSrcId !== null && this.dragSrcId !== folder.id) {
          this.reorderFolder(this.dragSrcId, folder.id);
        }
      });

      // Keyboard reorder: Alt+Up/Down
      li.tabIndex = 0;
      li.addEventListener("keydown", (e) => {
        if (!e.altKey) return;
        const idx = folders.findIndex((f) => f.id === folder.id);
        if (e.key === "ArrowUp" && idx > 0) {
          e.preventDefault();
          this.reorderFolder(folder.id, folders[idx - 1].id);
        } else if (e.key === "ArrowDown" && idx < folders.length - 1) {
          e.preventDefault();
          this.reorderFolder(folder.id, folders[idx + 1].id);
        }
      });

      list.appendChild(li);
    }

    this.el.appendChild(list);

    // Collections section
    this.renderCollections();
  }

  private async loadCollections(): Promise<void> {
    try {
      this.collections = await ProjectService.getCollections();
      this.render();
    } catch {
      // Silently continue without collections
    }
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
    const srcIdx = folders.findIndex((f) => f.id === srcId);
    const targetIdx = folders.findIndex((f) => f.id === targetId);
    if (srcIdx === -1 || targetIdx === -1) return;

    // Move src to target position
    const reordered = [...folders];
    const [moved] = reordered.splice(srcIdx, 1);
    reordered.splice(targetIdx, 0, moved);

    // Assign sort_order with gaps of 10
    const orders: [number, number][] = reordered.map((f, i) => [f.id, (i + 1) * 10]);

    try {
      await FolderService.updateSortOrders(orders);
      // Reload to get fresh order from backend
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
}
