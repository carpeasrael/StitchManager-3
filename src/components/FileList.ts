import { Component } from "./Component";
import { appState } from "../state/AppState";
import * as FileService from "../services/FileService";

export class FileList extends Component {
  private generation = 0;
  private lastClickedIndex: number | null = null;

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
    this.subscribe(
      appState.on("selectedFileIds", () => this.render())
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
    const selectedIds = appState.get("selectedFileIds");

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

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const card = document.createElement("div");
      card.className = "file-card";

      const isMultiSelected = selectedIds.includes(file.id);
      const isSingleSelected = file.id === selectedId && selectedIds.length === 0;
      if (isMultiSelected || isSingleSelected) {
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

      // AI badge
      if (file.aiAnalyzed) {
        const badge = document.createElement("span");
        if (file.aiConfirmed) {
          badge.className = "ai-badge ai-badge--confirmed";
          badge.textContent = "KI";
          badge.title = "KI-analysiert und best\u00E4tigt";
        } else {
          badge.className = "ai-badge ai-badge--pending";
          badge.textContent = "KI";
          badge.title = "KI-analysiert, nicht best\u00E4tigt";
        }
        nameEl.appendChild(badge);
      }

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

      const index = i;
      card.addEventListener("click", (e) => {
        this.handleClick(file.id, index, e);
      });

      list.appendChild(card);
    }

    this.el.appendChild(list);
  }

  private handleClick(fileId: number, index: number, e: MouseEvent): void {
    const files = appState.get("files");

    if (e.shiftKey && this.lastClickedIndex !== null) {
      // Shift+click: range select
      const start = Math.min(this.lastClickedIndex, index);
      const end = Math.max(this.lastClickedIndex, index);
      const rangeIds = files.slice(start, end + 1).map((f) => f.id);
      appState.set("selectedFileIds", rangeIds);
      appState.set("selectedFileId", fileId);
    } else if (e.metaKey || e.ctrlKey) {
      // Cmd/Ctrl+click: toggle in multi-select
      const current = appState.get("selectedFileIds");
      const singleId = appState.get("selectedFileId");
      let newIds: number[];

      if (current.length === 0 && singleId !== null) {
        // Transition from single-select to multi-select
        newIds = singleId === fileId ? [] : [singleId, fileId];
      } else if (current.includes(fileId)) {
        newIds = current.filter((id) => id !== fileId);
      } else {
        newIds = [...current, fileId];
      }

      appState.set("selectedFileIds", newIds);
      appState.set("selectedFileId", newIds.length > 0 ? newIds[newIds.length - 1] : null);
      this.lastClickedIndex = index;
    } else {
      // Normal click: single select, clear multi-select
      appState.set("selectedFileIds", []);
      appState.set("selectedFileId", fileId);
      this.lastClickedIndex = index;
    }
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
