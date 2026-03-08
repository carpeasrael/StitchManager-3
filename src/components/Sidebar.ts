import { Component } from "./Component";
import { appState } from "../state/AppState";
import * as FolderService from "../services/FolderService";
import type { Folder } from "../types/index";

export class Sidebar extends Component {
  private folderCounts = new Map<number, number>();

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("folders", () => this.render())
    );
    this.subscribe(
      appState.on("selectedFolderId", () => this.render())
    );
    this.loadFolders();
  }

  private async loadFolders(): Promise<void> {
    try {
      const folders = await FolderService.getAll();
      appState.set("folders", folders);
      await this.loadCounts(folders);
    } catch (e) {
      console.warn("Failed to load folders:", e);
    }
  }

  private async loadCounts(folders: Folder[]): Promise<void> {
    const results = await Promise.all(
      folders.map(async (folder) => {
        try {
          const count = await FolderService.getFileCount(folder.id);
          return [folder.id, count] as const;
        } catch {
          return [folder.id, 0] as const;
        }
      })
    );
    for (const [id, count] of results) {
      this.folderCounts.set(id, count);
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
    addBtn.addEventListener("click", () => this.createFolder());
    header.appendChild(addBtn);

    this.el.appendChild(header);

    if (folders.length === 0) {
      const empty = document.createElement("div");
      empty.className = "sidebar-empty";
      empty.textContent = "Keine Ordner vorhanden";
      this.el.appendChild(empty);
      return;
    }

    const list = document.createElement("ul");
    list.className = "folder-list";

    for (const folder of folders) {
      const li = document.createElement("li");
      li.className = "folder-item";
      if (folder.id === selectedId) {
        li.classList.add("selected");
      }

      const nameSpan = document.createElement("span");
      nameSpan.className = "folder-name";
      nameSpan.textContent = folder.name;

      const countSpan = document.createElement("span");
      countSpan.className = "folder-count";
      countSpan.textContent = String(this.folderCounts.get(folder.id) ?? 0);

      li.appendChild(nameSpan);
      li.appendChild(countSpan);

      li.addEventListener("click", () => {
        appState.set("selectedFolderId", folder.id);
      });

      list.appendChild(li);
    }

    this.el.appendChild(list);
  }

  private async createFolder(): Promise<void> {
    const name = prompt("Ordnername:");
    if (!name || !name.trim()) return;

    const path = prompt("Pfad zum Ordner:");
    if (!path || !path.trim()) return;

    try {
      await FolderService.create(name.trim(), path.trim());
      await this.loadFolders();
    } catch (e) {
      console.warn("Failed to create folder:", e);
      alert(`Fehler: ${e}`);
    }
  }
}
