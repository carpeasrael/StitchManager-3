import { Component } from "./Component";
import { appState } from "../state/AppState";
import * as FileService from "../services/FileService";

export class FileList extends Component {
  private generation = 0;

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("selectedFolderId", () => this.loadFiles())
    );
    this.subscribe(
      appState.on("searchQuery", () => this.loadFiles())
    );
    this.subscribe(
      appState.on("formatFilter", () => this.loadFiles())
    );
    this.subscribe(
      appState.on("files", () => this.render())
    );
    this.subscribe(
      appState.on("selectedFileId", () => this.render())
    );
    this.loadFiles();
  }

  private async loadFiles(): Promise<void> {
    const gen = ++this.generation;
    const folderId = appState.get("selectedFolderId");
    const search = appState.get("searchQuery");
    const formatFilter = appState.get("formatFilter");

    try {
      const files = await FileService.getFiles(folderId, search, formatFilter);
      if (gen !== this.generation) return;
      appState.set("files", files);
    } catch (e) {
      console.warn("Failed to load files:", e);
    }
  }

  render(): void {
    const files = appState.get("files");
    const selectedId = appState.get("selectedFileId");

    this.el.innerHTML = "";

    if (files.length === 0) {
      const empty = document.createElement("div");
      empty.className = "file-list-empty";
      empty.textContent = "Keine Dateien gefunden";
      this.el.appendChild(empty);
      return;
    }

    const list = document.createElement("div");
    list.className = "file-list";

    for (const file of files) {
      const card = document.createElement("div");
      card.className = "file-card";
      if (file.id === selectedId) {
        card.classList.add("selected");
      }

      const thumb = document.createElement("div");
      thumb.className = "file-card-thumb";
      thumb.textContent = this.getFormatLabel(file.filename);
      card.appendChild(thumb);

      const info = document.createElement("div");
      info.className = "file-card-info";

      const nameEl = document.createElement("div");
      nameEl.className = "file-card-name";
      nameEl.textContent = file.name || file.filename;
      info.appendChild(nameEl);

      const meta = document.createElement("div");
      meta.className = "file-card-meta";
      const parts: string[] = [];
      if (file.fileSizeBytes) {
        parts.push(this.formatSize(file.fileSizeBytes));
      }
      const ext = this.getFormatLabel(file.filename);
      if (ext) {
        parts.push(ext);
      }
      meta.textContent = parts.join(" \u00B7 ");
      info.appendChild(meta);

      card.appendChild(info);

      card.addEventListener("click", () => {
        appState.set("selectedFileId", file.id);
      });

      list.appendChild(card);
    }

    this.el.appendChild(list);
  }

  private getFormatLabel(filename: string): string {
    const ext = filename.split(".").pop();
    return ext ? ext.toUpperCase() : "";
  }

  private formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
}
