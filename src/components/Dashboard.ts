import { Component } from "./Component";
import { appState } from "../state/AppState";
import * as FileService from "../services/FileService";
import * as StatisticsService from "../services/StatisticsService";
import { formatSize } from "../utils/format";
import type { EmbroideryFile, LibraryStats, DashboardStats } from "../types/index";

export class Dashboard extends Component {
  private visible = false;

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("selectedFolderId", () => this.checkVisibility())
    );
    this.subscribe(
      appState.on("selectedSmartFolderId", () => this.checkVisibility())
    );
    this.subscribe(
      appState.on("files", () => this.checkVisibility())
    );
    this.checkVisibility();
  }

  private checkVisibility(): void {
    const folderId = appState.getRef("selectedFolderId");
    const smartFolderId = appState.getRef("selectedSmartFolderId");
    const shouldShow = folderId === null && smartFolderId === null;
    if (shouldShow !== this.visible) {
      this.visible = shouldShow;
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
      const [stats, recent, favorites, dashStats] = await Promise.all([
        FileService.getLibraryStats(),
        FileService.getRecentFiles(12),
        FileService.getFavoriteFiles(),
        StatisticsService.getDashboardStats(),
      ]);
      if (!this.visible) return;
      this.renderDashboard(stats, recent, favorites, dashStats);
    } catch (e) {
      console.warn("Failed to load dashboard:", e);
      this.el.innerHTML = '<div class="dashboard-empty">Dashboard konnte nicht geladen werden</div>';
    }
  }

  private renderDashboard(
    stats: LibraryStats,
    recent: EmbroideryFile[],
    favorites: EmbroideryFile[],
    dashStats: DashboardStats
  ): void {
    this.el.innerHTML = "";
    this.el.className = "dashboard";

    // Library overview
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

    const formats = Object.entries(stats.formatCounts);
    for (const [fmt, count] of formats) {
      statsGrid.appendChild(this.createStatCard(fmt, String(count)));
    }
    statsSection.appendChild(statsGrid);
    this.el.appendChild(statsSection);

    // File type breakdown
    const typeEntries = Object.entries(dashStats.filesByType);
    if (typeEntries.length > 0) {
      const typeSection = document.createElement("div");
      typeSection.className = "dashboard-section";
      const typeTitle = document.createElement("h3");
      typeTitle.className = "dashboard-section-title";
      typeTitle.textContent = "Dateitypen";
      typeSection.appendChild(typeTitle);
      const typeGrid = document.createElement("div");
      typeGrid.className = "dashboard-stats-grid";
      const typeLabels: Record<string, string> = {
        embroidery: "Stickmuster",
        sewing_pattern: "Schnittmuster",
      };
      for (const [ft, count] of typeEntries) {
        typeGrid.appendChild(this.createStatCard(typeLabels[ft] ?? ft, String(count)));
      }
      typeSection.appendChild(typeGrid);
      this.el.appendChild(typeSection);
    }

    // AI analysis status
    const aiSection = document.createElement("div");
    aiSection.className = "dashboard-section";
    const aiTitle = document.createElement("h3");
    aiTitle.className = "dashboard-section-title";
    aiTitle.textContent = "KI-Analyse";
    aiSection.appendChild(aiTitle);
    const aiGrid = document.createElement("div");
    aiGrid.className = "dashboard-stats-grid";
    aiGrid.appendChild(this.createStatCard("Nicht analysiert", String(dashStats.aiStatus.none)));
    aiGrid.appendChild(this.createStatCard("Analysiert", String(dashStats.aiStatus.analyzed)));
    aiGrid.appendChild(this.createStatCard("Bestätigt", String(dashStats.aiStatus.confirmed)));
    aiSection.appendChild(aiGrid);
    this.el.appendChild(aiSection);

    // Missing metadata
    const metaSection = document.createElement("div");
    metaSection.className = "dashboard-section";
    const metaTitle = document.createElement("h3");
    metaTitle.className = "dashboard-section-title";
    metaTitle.textContent = "Fehlende Metadaten";
    metaSection.appendChild(metaTitle);
    const metaGrid = document.createElement("div");
    metaGrid.className = "dashboard-stats-grid";
    metaGrid.appendChild(this.createStatCard("Ohne Tags", String(dashStats.missingMetadata.noTags)));
    metaGrid.appendChild(this.createStatCard("Ohne Bewertung", String(dashStats.missingMetadata.noRating)));
    metaGrid.appendChild(this.createStatCard("Ohne Beschreibung", String(dashStats.missingMetadata.noDescription)));
    metaSection.appendChild(metaGrid);
    this.el.appendChild(metaSection);

    // Recent imports
    if (dashStats.recentImports > 0) {
      const importSection = document.createElement("div");
      importSection.className = "dashboard-section";
      const importGrid = document.createElement("div");
      importGrid.className = "dashboard-stats-grid";
      importGrid.appendChild(this.createStatCard("Letzte 7 Tage", String(dashStats.recentImports)));
      importSection.appendChild(importGrid);
      this.el.appendChild(importSection);
    }

    // Top folders
    if (dashStats.topFolders.length > 0) {
      const topSection = document.createElement("div");
      topSection.className = "dashboard-section";
      const topTitle = document.createElement("h3");
      topTitle.className = "dashboard-section-title";
      topTitle.textContent = "Top Ordner";
      topSection.appendChild(topTitle);
      const topList = document.createElement("div");
      topList.className = "dashboard-list";
      for (const f of dashStats.topFolders) {
        if (f.value === 0) continue;
        const row = document.createElement("div");
        row.className = "dashboard-list-row";
        const nameEl = document.createElement("span");
        nameEl.textContent = f.folderName;
        const valEl = document.createElement("span");
        valEl.className = "dashboard-list-value";
        valEl.textContent = `${f.value} Dateien`;
        row.appendChild(nameEl);
        row.appendChild(valEl);
        topList.appendChild(row);
      }
      topSection.appendChild(topList);
      this.el.appendChild(topSection);
    }

    // Storage by folder
    if (dashStats.storageByFolder.length > 0) {
      const storageSection = document.createElement("div");
      storageSection.className = "dashboard-section";
      const storageTitle = document.createElement("h3");
      storageTitle.className = "dashboard-section-title";
      storageTitle.textContent = "Speicherverbrauch";
      storageSection.appendChild(storageTitle);
      const storageList = document.createElement("div");
      storageList.className = "dashboard-list";
      for (const f of dashStats.storageByFolder) {
        if (f.value === 0) continue;
        const row = document.createElement("div");
        row.className = "dashboard-list-row";
        const nameEl = document.createElement("span");
        nameEl.textContent = f.folderName;
        const valEl = document.createElement("span");
        valEl.className = "dashboard-list-value";
        valEl.textContent = formatSize(f.value);
        row.appendChild(nameEl);
        row.appendChild(valEl);
        storageList.appendChild(row);
      }
      storageSection.appendChild(storageList);
      this.el.appendChild(storageSection);
    }

    // Recent files
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

    // Favorites
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
      empty.textContent = "Keine Dateien vorhanden. Ordner hinzufügen um zu starten.";
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
      appState.set("selectedSmartFolderId", null);
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
