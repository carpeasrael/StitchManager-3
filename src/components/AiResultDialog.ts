import { EventBus } from "../state/EventBus";
import * as AiService from "../services/AiService";
import * as FileService from "../services/FileService";
import type {
  AiAnalysisResult,
  SelectedFields,
  ThreadColor,
} from "../types/index";

interface AiColor {
  hex: string;
  name?: string;
}

export class AiResultDialog {
  private overlay: HTMLElement | null = null;
  private result: AiAnalysisResult;
  private fileId: number;
  private existingColors: ThreadColor[];

  constructor(
    result: AiAnalysisResult,
    fileId: number,
    existingColors: ThreadColor[]
  ) {
    this.result = result;
    this.fileId = fileId;
    this.existingColors = existingColors;
  }

  static async open(
    result: AiAnalysisResult,
    fileId: number
  ): Promise<void> {
    const existingColors = await FileService.getColors(fileId);
    const dialog = new AiResultDialog(result, fileId, existingColors);
    dialog.show();
  }

  private show(): void {
    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });
    this.overlay.addEventListener("dialog-dismiss", () => this.close());

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-ai-result";

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";
    header.innerHTML =
      '<span class="dialog-title">KI-Ergebnis</span>';
    const closeBtn = document.createElement("button");
    closeBtn.className = "dialog-close";
    closeBtn.textContent = "\u00D7";
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    // Body
    const body = document.createElement("div");
    body.className = "dialog-body dialog-result-body";

    // Field checkboxes
    const fieldsSection = document.createElement("div");
    fieldsSection.className = "dialog-result-fields";

    const fieldsHeader = document.createElement("div");
    fieldsHeader.className = "dialog-section-header";
    fieldsHeader.textContent = "KI-Vorschlaege";
    fieldsSection.appendChild(fieldsHeader);

    const checkboxes: Record<string, HTMLInputElement> = {};

    if (this.result.parsedName) {
      checkboxes.name = this.addFieldCheckbox(
        fieldsSection,
        "Name",
        this.result.parsedName
      );
    }
    if (this.result.parsedTheme) {
      checkboxes.theme = this.addFieldCheckbox(
        fieldsSection,
        "Thema",
        this.result.parsedTheme
      );
    }
    if (this.result.parsedDesc) {
      checkboxes.description = this.addFieldCheckbox(
        fieldsSection,
        "Beschreibung",
        this.result.parsedDesc
      );
    }
    if (this.result.parsedTags) {
      const tags = this.parseTags(this.result.parsedTags);
      if (tags.length > 0) {
        checkboxes.tags = this.addFieldCheckbox(
          fieldsSection,
          "Tags",
          tags.join(", ")
        );
      }
    }

    body.appendChild(fieldsSection);

    // Color comparison
    const aiColors = this.parseColors(this.result.parsedColors);
    if (aiColors.length > 0 || this.existingColors.length > 0) {
      const colorSection = document.createElement("div");
      colorSection.className = "dialog-result-colors";

      const colorHeader = document.createElement("div");
      colorHeader.className = "dialog-section-header";
      colorHeader.textContent = "Farben";
      colorSection.appendChild(colorHeader);

      // Existing (parser) colors
      if (this.existingColors.filter((c) => !c.isAi).length > 0) {
        const parserLabel = document.createElement("div");
        parserLabel.className = "dialog-color-label";
        parserLabel.textContent = "Parser-Farben:";
        colorSection.appendChild(parserLabel);

        const parserSwatches = document.createElement("div");
        parserSwatches.className = "dialog-color-swatches";
        for (const c of this.existingColors.filter((c) => !c.isAi)) {
          this.addSwatch(parserSwatches, c.colorHex, c.colorName || undefined);
        }
        colorSection.appendChild(parserSwatches);
      }

      // AI colors
      if (aiColors.length > 0) {
        const aiLabel = document.createElement("div");
        aiLabel.className = "dialog-color-label";
        aiLabel.textContent = "KI-Farben:";
        colorSection.appendChild(aiLabel);

        const aiSwatches = document.createElement("div");
        aiSwatches.className = "dialog-color-swatches";
        for (const c of aiColors) {
          this.addSwatch(aiSwatches, c.hex, c.name);
        }
        colorSection.appendChild(aiSwatches);

        checkboxes.colors = this.addFieldCheckbox(
          colorSection,
          "KI-Farben uebernehmen",
          ""
        );
      }

      body.appendChild(colorSection);
    }

    dialog.appendChild(body);

    // Footer
    const footer = document.createElement("div");
    footer.className = "dialog-footer";

    const rejectBtn = document.createElement("button");
    rejectBtn.className = "dialog-btn dialog-btn-danger";
    rejectBtn.textContent = "Ablehnen";
    rejectBtn.addEventListener("click", () => this.reject());
    footer.appendChild(rejectBtn);

    const acceptAllBtn = document.createElement("button");
    acceptAllBtn.className = "dialog-btn dialog-btn-secondary";
    acceptAllBtn.textContent = "Alle akzeptieren";
    acceptAllBtn.addEventListener("click", () => {
      Object.values(checkboxes).forEach((cb) => (cb.checked = true));
      this.accept(checkboxes);
    });
    footer.appendChild(acceptAllBtn);

    const acceptBtn = document.createElement("button");
    acceptBtn.className = "dialog-btn dialog-btn-primary";
    acceptBtn.textContent = "Akzeptieren";
    acceptBtn.addEventListener("click", () => this.accept(checkboxes));
    footer.appendChild(acceptBtn);

    dialog.appendChild(footer);
    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);
  }

  private addFieldCheckbox(
    container: HTMLElement,
    label: string,
    value: string
  ): HTMLInputElement {
    const row = document.createElement("label");
    row.className = "dialog-result-row";

    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = true;
    checkbox.className = "dialog-checkbox";
    row.appendChild(checkbox);

    const textWrapper = document.createElement("div");
    textWrapper.className = "dialog-result-text";

    const labelEl = document.createElement("span");
    labelEl.className = "dialog-result-label";
    labelEl.textContent = label;
    textWrapper.appendChild(labelEl);

    if (value) {
      const valueEl = document.createElement("span");
      valueEl.className = "dialog-result-value";
      valueEl.textContent = value;
      textWrapper.appendChild(valueEl);
    }

    row.appendChild(textWrapper);
    container.appendChild(row);

    return checkbox;
  }

  private isValidHex(hex: string): boolean {
    return /^#[0-9a-fA-F]{6}$/.test(hex);
  }

  private addSwatch(
    container: HTMLElement,
    hex: string,
    name?: string
  ): void {
    const swatch = document.createElement("div");
    swatch.className = "dialog-swatch";

    const colorBox = document.createElement("div");
    colorBox.className = "dialog-swatch-color";
    colorBox.style.backgroundColor = this.isValidHex(hex) ? hex : "#cccccc";
    colorBox.title = name ? `${name} (${hex})` : hex;
    swatch.appendChild(colorBox);

    const label = document.createElement("span");
    label.className = "dialog-swatch-label";
    label.textContent = name || hex;
    swatch.appendChild(label);

    container.appendChild(swatch);
  }

  private async accept(
    checkboxes: Record<string, HTMLInputElement>
  ): Promise<void> {
    const selectedFields: SelectedFields = {
      name: checkboxes.name?.checked,
      theme: checkboxes.theme?.checked,
      description: checkboxes.description?.checked,
      tags: checkboxes.tags?.checked,
      colors: checkboxes.colors?.checked,
    };

    try {
      await AiService.acceptResult(this.result.id, selectedFields);
      EventBus.emit("file:updated", { fileId: this.fileId });
      this.close();
    } catch (e) {
      console.warn("Failed to accept AI result:", e);
      this.showError("Fehler beim Akzeptieren der KI-Ergebnisse");
    }
  }

  private async reject(): Promise<void> {
    try {
      await AiService.rejectResult(this.result.id);
      EventBus.emit("file:updated", { fileId: this.fileId });
      this.close();
    } catch (e) {
      console.warn("Failed to reject AI result:", e);
      this.showError("Fehler beim Ablehnen der KI-Ergebnisse");
    }
  }

  private showError(message: string): void {
    if (!this.overlay) return;
    const footer = this.overlay.querySelector(".dialog-footer");
    if (!footer) return;

    let errorEl = footer.querySelector(".dialog-error");
    if (!errorEl) {
      errorEl = document.createElement("div");
      errorEl.className = "dialog-error";
      footer.insertBefore(errorEl, footer.firstChild);
    }
    errorEl.textContent = message;
  }

  private close(): void {
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }

  private parseTags(tagsJson: string): string[] {
    try {
      const parsed = JSON.parse(tagsJson);
      if (Array.isArray(parsed)) return parsed;
    } catch {
      // ignore
    }
    return [];
  }

  private parseColors(colorsJson: string | null): AiColor[] {
    if (!colorsJson) return [];
    try {
      const parsed = JSON.parse(colorsJson);
      if (Array.isArray(parsed)) return parsed;
    } catch {
      // ignore
    }
    return [];
  }
}
