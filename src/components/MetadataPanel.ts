import { Component } from "./Component";
import { appState } from "../state/AppState";
import { convertFileSrc } from "@tauri-apps/api/core";
import * as FileService from "../services/FileService";
import type { EmbroideryFile, ThreadColor, FileFormat } from "../types/index";

export class MetadataPanel extends Component {
  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("selectedFileId", () => this.onSelectionChanged())
    );
    this.render();
  }

  private async onSelectionChanged(): Promise<void> {
    const fileId = appState.get("selectedFileId");
    if (fileId === null) {
      this.render();
      return;
    }

    try {
      const [file, formats, colors] = await Promise.all([
        FileService.getFile(fileId),
        FileService.getFormats(fileId),
        FileService.getColors(fileId),
      ]);
      this.renderFileInfo(file, formats, colors);
    } catch (e) {
      console.warn("Failed to load file details:", e);
      this.renderError();
    }
  }

  render(): void {
    this.el.innerHTML = "";
    const empty = document.createElement("div");
    empty.className = "metadata-empty";
    empty.innerHTML =
      '<div class="metadata-empty-icon">&#9881;</div>' +
      '<div class="metadata-empty-text">Keine Datei ausgewählt</div>' +
      '<div class="metadata-empty-hint">Wähle eine Datei aus der Liste, um Details anzuzeigen.</div>';
    this.el.appendChild(empty);
  }

  private renderFileInfo(
    file: EmbroideryFile,
    formats: FileFormat[],
    colors: ThreadColor[]
  ): void {
    this.el.innerHTML = "";

    const wrapper = document.createElement("div");
    wrapper.className = "metadata-panel";

    // Thumbnail section
    const thumbSection = document.createElement("div");
    thumbSection.className = "metadata-thumbnail";
    if (file.thumbnailPath) {
      const img = document.createElement("img");
      img.src = convertFileSrc(file.thumbnailPath);
      img.alt = file.name || file.filename;
      img.className = "metadata-thumbnail-img";
      thumbSection.appendChild(img);
    } else {
      const placeholder = document.createElement("div");
      placeholder.className = "metadata-thumbnail-placeholder";
      placeholder.textContent = this.getFormatLabel(file.filename);
      thumbSection.appendChild(placeholder);
    }
    wrapper.appendChild(thumbSection);

    // File info section
    const infoSection = document.createElement("div");
    infoSection.className = "metadata-section";

    const infoHeader = document.createElement("div");
    infoHeader.className = "metadata-section-header";
    infoHeader.textContent = "Dateiinformationen";
    infoSection.appendChild(infoHeader);

    const infoGrid = document.createElement("div");
    infoGrid.className = "metadata-info-grid";

    this.addInfoRow(infoGrid, "Name", file.name || file.filename);
    this.addInfoRow(infoGrid, "Format", this.getFormatLabel(file.filename));

    if (formats.length > 0 && formats[0].formatVersion) {
      this.addInfoRow(infoGrid, "Version", formats[0].formatVersion);
    }

    if (file.widthMm !== null && file.heightMm !== null) {
      this.addInfoRow(
        infoGrid,
        "Abmessungen",
        `${file.widthMm.toFixed(1)} × ${file.heightMm.toFixed(1)} mm`
      );
    }

    if (file.stitchCount !== null) {
      this.addInfoRow(
        infoGrid,
        "Stiche",
        file.stitchCount.toLocaleString("de-DE")
      );
    }

    if (file.colorCount !== null) {
      this.addInfoRow(infoGrid, "Farben", String(file.colorCount));
    }

    if (file.fileSizeBytes !== null) {
      this.addInfoRow(infoGrid, "Dateigröße", this.formatSize(file.fileSizeBytes));
    }

    infoSection.appendChild(infoGrid);
    wrapper.appendChild(infoSection);

    // Color swatches section
    const colorSection = document.createElement("div");
    colorSection.className = "metadata-section";

    const colorHeader = document.createElement("div");
    colorHeader.className = "metadata-section-header";
    colorHeader.textContent = "Farben";
    colorSection.appendChild(colorHeader);

    if (colors.length > 0) {
      const swatchGrid = document.createElement("div");
      swatchGrid.className = "metadata-swatch-grid";

      for (const color of colors) {
        const swatch = document.createElement("div");
        swatch.className = "metadata-swatch";

        const colorBox = document.createElement("div");
        colorBox.className = "metadata-swatch-color";
        colorBox.style.backgroundColor = color.colorHex;
        swatch.appendChild(colorBox);

        const colorInfo = document.createElement("div");
        colorInfo.className = "metadata-swatch-info";

        if (color.colorName) {
          const nameEl = document.createElement("span");
          nameEl.className = "metadata-swatch-name";
          nameEl.textContent = color.colorName;
          colorInfo.appendChild(nameEl);
        }

        if (color.brand) {
          const brandEl = document.createElement("span");
          brandEl.className = "metadata-swatch-brand";
          brandEl.textContent = color.brand;
          colorInfo.appendChild(brandEl);
        }

        if (!color.colorName && !color.brand) {
          const hexEl = document.createElement("span");
          hexEl.className = "metadata-swatch-name";
          hexEl.textContent = color.colorHex;
          colorInfo.appendChild(hexEl);
        }

        swatch.appendChild(colorInfo);
        swatchGrid.appendChild(swatch);
      }

      colorSection.appendChild(swatchGrid);
    } else {
      const noColors = document.createElement("div");
      noColors.className = "metadata-no-colors";
      noColors.textContent = "Keine Farbinformationen";
      colorSection.appendChild(noColors);
    }

    wrapper.appendChild(colorSection);
    this.el.appendChild(wrapper);
  }

  private renderError(): void {
    this.el.innerHTML = "";
    const error = document.createElement("div");
    error.className = "metadata-empty";
    error.innerHTML =
      '<div class="metadata-empty-text">Fehler beim Laden der Dateidetails</div>';
    this.el.appendChild(error);
  }

  private addInfoRow(grid: HTMLElement, label: string, value: string): void {
    const row = document.createElement("div");
    row.className = "metadata-info-row";

    const labelEl = document.createElement("span");
    labelEl.className = "metadata-info-label";
    labelEl.textContent = label;
    row.appendChild(labelEl);

    const valueEl = document.createElement("span");
    valueEl.className = "metadata-info-value";
    valueEl.textContent = value;
    row.appendChild(valueEl);

    grid.appendChild(row);
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
