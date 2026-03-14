import { Component } from "./Component";
import { appState } from "../state/AppState";
import * as FileService from "../services/FileService";
import { formatSize } from "../utils/format";
import type { EmbroideryFile, LibraryStats } from "../types/index";

export class Dashboard extends Component {
  private visible = false;

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("selectedFolderId", () => this.checkVisibility())
    );
    this.subscribe(
      appState.on("files", () => this.checkVisibility())
    );
    this.checkVisibility();
  }

  private checkVisibility(): void {
    const folderId = appState.getRef("selectedFolderId");
    const shouldShow = folderId === null;
    if (shouldShow !== this.visible) {
      this.visible = shouldShow;
      // Toggle visibility of dashboard and sibling file list
      this.el.style.display = shouldShow ? "" : "none";
      const sibling = this.el.nextElementSibling as HTMLElement | null;
      if (sibling) sibling.style.display = shouldShow ? "none" : "";
      if (shouldShow) {
        this.load();
      } else {
        this.el.innerHTML = "";
      }
    }
  }

  private async load(): Promise<void> {
    try {
      const [stats, recent, favorites] = await Promise.all([
        FileService.getLibraryStats(),
        FileService.getRecentFiles(12),
        FileService.getFavoriteFiles(),
      ]);
      if (!this.visible) return;
      this.renderDashboard(stats, recent, favorites);
    } catch (e) {
      console.warn("Failed to load dashboard:", e);
      this.el.innerHTML = '<div class="dashboard-empty">Dashboard konnte nicht geladen werden</div>';
    }
  }

  private renderDashboard(stats: LibraryStats, recent: EmbroideryFile[], favorites: EmbroideryFile[]): void {
    this.el.innerHTML = "";
    this.el.className = "dashboard";

    // Stats section
    const statsSection = document.createElement("div");
    statsSection.className = "dashboard-section";

    const statsTitle = document.createElement("h3");
    statsTitle.className = "dashboard-section-title";
    statsTitle.textContent = "Bibliothek";
    statsSection.appendChild(statsTitle);

    const statsGrid = document.createElement("div");
    statsGrid.className = "dashboard-stats-grid";

    statsGrid.appendChild(this.createStatCard("Dateien", String(stats.totalFiles)));
    statsGrid.appendChild(this.createStatCard("Ordner", String(stats.totalFolders)));
    statsGrid.appendChild(this.createStatCard("Stiche", this.formatNumber(stats.totalStitches)));

    // Format breakdown
    const formats = Object.entries(stats.formatCounts);
    if (formats.length > 0) {
      for (const [fmt, count] of formats) {
        statsGrid.appendChild(this.createStatCard(fmt, String(count)));
      }
    }

    statsSection.appendChild(statsGrid);
    this.el.appendChild(statsSection);

    // Recent files section
    if (recent.length > 0) {
      const recentSection = document.createElement("div");
      recentSection.className = "dashboard-section";

      const recentTitle = document.createElement("h3");
      recentTitle.className = "dashboard-section-title";
      recentTitle.textContent = "Zuletzt bearbeitet";
      recentSection.appendChild(recentTitle);

      const recentGrid = document.createElement("div");
      recentGrid.className = "dashboard-file-grid";
      for (const file of recent) {
        recentGrid.appendChild(this.createFileCard(file));
      }
      recentSection.appendChild(recentGrid);
      this.el.appendChild(recentSection);
    }

    // Favorites section
    if (favorites.length > 0) {
      const favSection = document.createElement("div");
      favSection.className = "dashboard-section";

      const favTitle = document.createElement("h3");
      favTitle.className = "dashboard-section-title";
      favTitle.textContent = "Favoriten";
      favSection.appendChild(favTitle);

      const favGrid = document.createElement("div");
      favGrid.className = "dashboard-file-grid";
      for (const file of favorites) {
        favGrid.appendChild(this.createFileCard(file));
      }
      favSection.appendChild(favGrid);
      this.el.appendChild(favSection);
    }

    // Empty state
    if (stats.totalFiles === 0) {
      const empty = document.createElement("div");
      empty.className = "dashboard-empty";
      empty.textContent = "Keine Dateien vorhanden. Ordner hinzufuegen um zu starten.";
      this.el.appendChild(empty);
    }
  }

  private createStatCard(label: string, value: string): HTMLElement {
    const card = document.createElement("div");
    card.className = "dashboard-stat-card";

    const valEl = document.createElement("div");
    valEl.className = "dashboard-stat-value";
    valEl.textContent = value;
    card.appendChild(valEl);

    const labelEl = document.createElement("div");
    labelEl.className = "dashboard-stat-label";
    labelEl.textContent = label;
    card.appendChild(labelEl);

    return card;
  }

  private createFileCard(file: EmbroideryFile): HTMLElement {
    const card = document.createElement("div");
    card.className = "dashboard-file-card";
    card.title = file.name || file.filename;

    const thumb = document.createElement("div");
    thumb.className = "dashboard-file-thumb";
    const ext = file.filename.split(".").pop()?.toUpperCase() || "";
    thumb.textContent = ext;

    // Load thumbnail async
    FileService.getThumbnail(file.id).then((dataUri) => {
      if (dataUri && thumb.isConnected) {
        const img = document.createElement("img");
        img.src = dataUri;
        img.alt = file.name || file.filename;
        img.className = "dashboard-file-thumb-img";
        thumb.textContent = "";
        thumb.appendChild(img);
      }
    }).catch(() => { /* keep ext label */ });

    card.appendChild(thumb);

    const name = document.createElement("div");
    name.className = "dashboard-file-name";
    name.textContent = file.name || file.filename;
    card.appendChild(name);

    const meta = document.createElement("div");
    meta.className = "dashboard-file-meta";
    const parts: string[] = [];
    if (file.fileSizeBytes) parts.push(formatSize(file.fileSizeBytes));
    if (file.stitchCount) parts.push(`${file.stitchCount} Stiche`);
    meta.textContent = parts.join(" \u00B7 ");
    card.appendChild(meta);

    card.addEventListener("click", () => {
      // Navigate to the file's folder and select it
      appState.set("selectedFileIds", []);
      appState.set("selectedFolderId", file.folderId);
      appState.set("selectedFileId", file.id);
    });

    return card;
  }

  private formatNumber(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return String(n);
  }

  render(): void {
    this.checkVisibility();
  }
}
