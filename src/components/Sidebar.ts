import { Component } from "./Component";
import { appState } from "../state/AppState";
import { ToastContainer } from "./Toast";
import * as FolderService from "../services/FolderService";

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
        if (folder.id === appState.get("selectedFolderId")) {
          // Re-click: deselect → go to "Alle Ordner"
          appState.set("selectedFileIds", []);
          appState.set("selectedFileId", null);
          appState.set("selectedFolderId", null);
        } else {
          appState.set("selectedFileIds", []);
          appState.set("selectedFileId", null);
          appState.set("selectedFolderId", folder.id);
        }
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
