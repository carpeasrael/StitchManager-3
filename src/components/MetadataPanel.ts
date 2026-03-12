import { Component } from "./Component";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { getFormatLabel, formatSize } from "../utils/format";
import * as FileService from "../services/FileService";
import * as SettingsService from "../services/SettingsService";
import type {
  EmbroideryFile,
  ThreadColor,
  FileFormat,
  Tag,
  FileUpdate,
  CustomFieldDef,
  StitchSegment,
} from "../types/index";

interface FormSnapshot {
  name: string;
  theme: string;
  description: string;
  license: string;
  tags: string[];
}

export class MetadataPanel extends Component {
  private currentFile: EmbroideryFile | null = null;
  private currentTags: Tag[] = [];
  private allTags: Tag[] = [];
  private customFields: CustomFieldDef[] = [];
  private snapshot: FormSnapshot | null = null;
  private dirty = false;
  private saving = false;
  private previewCleanup: (() => void) | null = null;

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("selectedFileId", () => this.onSelectionChanged())
    );
    this.subscribe(
      EventBus.on("file:refresh", () => this.onSelectionChanged())
    );
    this.subscribe(
      EventBus.on("metadata:save", () => this.save())
    );
    this.render();
  }

  destroy(): void {
    if (this.previewCleanup) { this.previewCleanup(); this.previewCleanup = null; }
    super.destroy();
  }

  private async onSelectionChanged(): Promise<void> {
    const fileId = appState.get("selectedFileId");
    if (fileId === null) {
      this.currentFile = null;
      this.currentTags = [];
      this.snapshot = null;
      this.dirty = false;
      this.render();
      return;
    }

    try {
      const [file, formats, colors, tags, allTags, customFields] = await Promise.all([
        FileService.getFile(fileId),
        FileService.getFormats(fileId),
        FileService.getColors(fileId),
        FileService.getTags(fileId),
        FileService.getAllTags(),
        SettingsService.getCustomFields(),
      ]);
      this.currentFile = file;
      this.currentTags = tags;
      this.allTags = allTags;
      this.customFields = customFields;
      this.snapshot = this.takeSnapshot(file, tags);
      this.dirty = false;
      this.renderFileInfo(file, formats, colors, tags);
    } catch (e) {
      console.warn("Failed to load file details:", e);
      this.renderError();
    }
  }

  private takeSnapshot(file: EmbroideryFile, tags: Tag[]): FormSnapshot {
    return {
      name: file.name || "",
      theme: file.theme || "",
      description: file.description || "",
      license: file.license || "",
      tags: tags.map((t) => t.name).sort(),
    };
  }

  private checkDirty(): void {
    if (!this.snapshot) {
      this.dirty = false;
      return;
    }
    const current = this.getCurrentFormValues();
    this.dirty =
      current.name !== this.snapshot.name ||
      current.theme !== this.snapshot.theme ||
      current.description !== this.snapshot.description ||
      current.license !== this.snapshot.license ||
      current.tags.join(",") !== this.snapshot.tags.join(",");

    const saveBtn = this.el.querySelector<HTMLButtonElement>(".metadata-save-btn");
    if (saveBtn) {
      saveBtn.disabled = !this.dirty || this.saving;
    }
  }

  private getCurrentFormValues(): FormSnapshot {
    const getValue = (name: string): string => {
      const el = this.el.querySelector<HTMLInputElement | HTMLTextAreaElement>(
        `[data-field="${name}"]`
      );
      return el ? el.value : "";
    };

    const tagChips = this.el.querySelectorAll<HTMLElement>(".tag-chip");
    const tags: string[] = [];
    tagChips.forEach((chip) => {
      const name = chip.dataset.tag;
      if (name) tags.push(name);
    });

    return {
      name: getValue("name"),
      theme: getValue("theme"),
      description: getValue("description"),
      license: getValue("license"),
      tags: tags.sort(),
    };
  }

  render(): void {
    if (this.previewCleanup) { this.previewCleanup(); this.previewCleanup = null; }
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
    colors: ThreadColor[],
    tags: Tag[]
  ): void {
    if (this.previewCleanup) { this.previewCleanup(); this.previewCleanup = null; }
    this.el.innerHTML = "";

    const wrapper = document.createElement("div");
    wrapper.className = "metadata-panel";

    // Stitch preview section — interactive canvas with zoom/pan
    const previewSection = document.createElement("div");
    previewSection.className = "stitch-preview-section";

    const previewContainer = document.createElement("div");
    previewContainer.className = "stitch-preview-container";

    const canvas = document.createElement("canvas");
    canvas.className = "stitch-preview-canvas";
    previewContainer.appendChild(canvas);

    // Zoom controls overlay
    const controls = document.createElement("div");
    controls.className = "stitch-preview-controls";
    const zoomInBtn = document.createElement("button");
    zoomInBtn.className = "stitch-preview-btn";
    zoomInBtn.textContent = "+";
    zoomInBtn.title = "Vergr\u00F6\u00DFern";
    const zoomOutBtn = document.createElement("button");
    zoomOutBtn.className = "stitch-preview-btn";
    zoomOutBtn.textContent = "\u2212";
    zoomOutBtn.title = "Verkleinern";
    const zoomResetBtn = document.createElement("button");
    zoomResetBtn.className = "stitch-preview-btn";
    zoomResetBtn.textContent = "\u21BA";
    zoomResetBtn.title = "Zur\u00FCcksetzen";
    const zoomLabel = document.createElement("span");
    zoomLabel.className = "stitch-preview-zoom-label";
    zoomLabel.textContent = "100%";
    controls.appendChild(zoomInBtn);
    controls.appendChild(zoomOutBtn);
    controls.appendChild(zoomResetBtn);
    controls.appendChild(zoomLabel);
    previewContainer.appendChild(controls);

    previewSection.appendChild(previewContainer);
    wrapper.appendChild(previewSection);

    // Load stitch segments and render on canvas
    const previewFileId = file.id;
    if (file.filepath) {
      this.loadStitchPreview(canvas, file.filepath, previewFileId, zoomLabel, {
        zoomInBtn, zoomOutBtn, zoomResetBtn,
      }).catch(() => { /* keep empty canvas */ });
    }

    // AI analyze button (always visible for analysis)
    const aiBar = document.createElement("div");
    aiBar.className = "metadata-ai-bar";

    const aiBtn = document.createElement("button");
    aiBtn.className = "metadata-ai-btn";
    aiBtn.textContent = "\u2728 KI analysieren";
    aiBtn.addEventListener("click", () => {
      EventBus.emit("toolbar:ai-analyze");
    });

    if (file.aiAnalyzed) {
      const label = document.createElement("span");
      label.className = file.aiConfirmed
        ? "metadata-ai-status metadata-ai-confirmed"
        : "metadata-ai-status metadata-ai-pending";
      label.textContent = file.aiConfirmed
        ? "KI-best\u00E4tigt"
        : "KI-analysiert";
      aiBar.appendChild(label);
    }

    aiBar.appendChild(aiBtn);
    wrapper.appendChild(aiBar);

    // Editable form section
    const formSection = document.createElement("div");
    formSection.className = "metadata-section";

    const formHeader = document.createElement("div");
    formHeader.className = "metadata-section-header";
    formHeader.textContent = "Metadaten";
    formSection.appendChild(formHeader);

    const form = document.createElement("div");
    form.className = "metadata-form";

    this.addFormField(form, "Name", "name", file.name || "", "text");
    this.addFormField(form, "Thema", "theme", file.theme || "", "text");
    this.addFormField(
      form,
      "Beschreibung",
      "description",
      file.description || "",
      "textarea"
    );
    this.addFormField(form, "Lizenz", "license", file.license || "", "text");

    formSection.appendChild(form);
    wrapper.appendChild(formSection);

    // Tags section
    const tagSection = document.createElement("div");
    tagSection.className = "metadata-section";

    const tagHeader = document.createElement("div");
    tagHeader.className = "metadata-section-header";
    tagHeader.textContent = "Tags";
    tagSection.appendChild(tagHeader);

    this.renderTagEditor(tagSection, tags);
    wrapper.appendChild(tagSection);

    // Custom fields section
    if (this.customFields.length > 0) {
      const customSection = document.createElement("div");
      customSection.className = "metadata-section";

      const customHeader = document.createElement("div");
      customHeader.className = "metadata-section-header";
      customHeader.textContent = "Benutzerdefinierte Felder";
      customSection.appendChild(customHeader);

      const customForm = document.createElement("div");
      customForm.className = "metadata-form";

      for (const field of this.customFields) {
        this.renderCustomField(customForm, field);
      }

      customSection.appendChild(customForm);
      wrapper.appendChild(customSection);
    }

    // Read-only info section
    const infoSection = document.createElement("div");
    infoSection.className = "metadata-section";

    const infoHeader = document.createElement("div");
    infoHeader.className = "metadata-section-header";
    infoHeader.textContent = "Dateiinformationen";
    infoSection.appendChild(infoHeader);

    const infoGrid = document.createElement("div");
    infoGrid.className = "metadata-info-grid";

    this.addInfoRow(infoGrid, "Dateiname", file.filename);
    this.addInfoRow(infoGrid, "Format", getFormatLabel(file.filename));

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

    if (file.designName) {
      this.addInfoRow(infoGrid, "Designname", file.designName);
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

    if (file.jumpCount !== null && file.jumpCount > 0) {
      this.addInfoRow(
        infoGrid,
        "Sprungstiche",
        file.jumpCount.toLocaleString("de-DE")
      );
    }

    if (file.trimCount !== null && file.trimCount > 0) {
      this.addInfoRow(
        infoGrid,
        "Schnitte",
        file.trimCount.toLocaleString("de-DE")
      );
    }

    if (file.hoopWidthMm !== null && file.hoopHeightMm !== null) {
      this.addInfoRow(
        infoGrid,
        "Stickrahmen",
        `${file.hoopWidthMm.toFixed(0)} \u00D7 ${file.hoopHeightMm.toFixed(0)} mm`
      );
    }

    if (file.fileSizeBytes !== null) {
      this.addInfoRow(
        infoGrid,
        "Dateigröße",
        formatSize(file.fileSizeBytes)
      );
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
        const validHex = /^#[0-9a-fA-F]{6}$/.test(color.colorHex);
        colorBox.style.backgroundColor = validHex ? color.colorHex : "#cccccc";
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

    // Save button
    const saveBar = document.createElement("div");
    saveBar.className = "metadata-save-bar";

    const saveBtn = document.createElement("button");
    saveBtn.className = "metadata-save-btn";
    saveBtn.textContent = "Speichern";
    saveBtn.disabled = true;
    saveBtn.addEventListener("click", () => this.save());
    saveBar.appendChild(saveBtn);

    wrapper.appendChild(saveBar);

    this.el.appendChild(wrapper);
  }

  private addFormField(
    container: HTMLElement,
    label: string,
    field: string,
    value: string,
    type: "text" | "textarea"
  ): void {
    const group = document.createElement("div");
    group.className = "metadata-form-group";

    const labelEl = document.createElement("label");
    labelEl.className = "metadata-form-label";
    labelEl.textContent = label;
    group.appendChild(labelEl);

    if (type === "textarea") {
      const textarea = document.createElement("textarea");
      textarea.className = "metadata-form-input metadata-form-textarea";
      textarea.dataset.field = field;
      textarea.value = value;
      textarea.rows = 3;
      textarea.addEventListener("input", () => this.checkDirty());
      group.appendChild(textarea);
    } else {
      const input = document.createElement("input");
      input.type = "text";
      input.className = "metadata-form-input";
      input.dataset.field = field;
      input.value = value;
      input.addEventListener("input", () => this.checkDirty());
      group.appendChild(input);
    }

    container.appendChild(group);
  }

  private renderTagEditor(container: HTMLElement, tags: Tag[]): void {
    const tagEditor = document.createElement("div");
    tagEditor.className = "tag-editor";

    const chipContainer = document.createElement("div");
    chipContainer.className = "tag-chip-container";

    for (const tag of tags) {
      this.addTagChip(chipContainer, tag.name);
    }

    tagEditor.appendChild(chipContainer);

    // Tag input with autocomplete
    const inputWrapper = document.createElement("div");
    inputWrapper.className = "tag-input-wrapper";

    const input = document.createElement("input");
    input.type = "text";
    input.className = "tag-input";
    input.placeholder = "Tag hinzufügen...";

    const suggestions = document.createElement("div");
    suggestions.className = "tag-suggestions";
    suggestions.style.display = "none";

    input.addEventListener("input", () => {
      const val = input.value.trim().toLowerCase();
      if (val.length === 0) {
        suggestions.style.display = "none";
        return;
      }

      const currentTags = new Set(
        Array.from(
          chipContainer.querySelectorAll<HTMLElement>(".tag-chip")
        ).map((c) => c.dataset.tag || "")
      );

      const matches = this.allTags.filter(
        (t) =>
          t.name.toLowerCase().includes(val) && !currentTags.has(t.name)
      );

      if (matches.length === 0) {
        suggestions.style.display = "none";
        return;
      }

      suggestions.innerHTML = "";
      for (const match of matches.slice(0, 8)) {
        const item = document.createElement("div");
        item.className = "tag-suggestion-item";
        item.textContent = match.name;
        item.addEventListener("mousedown", (e) => {
          e.preventDefault();
          this.addTagChip(chipContainer, match.name);
          input.value = "";
          suggestions.style.display = "none";
          this.checkDirty();
        });
        suggestions.appendChild(item);
      }
      suggestions.style.display = "block";
    });

    input.addEventListener("blur", () => {
      setTimeout(() => {
        suggestions.style.display = "none";
      }, 150);
    });

    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === ",") {
        e.preventDefault();
        const val = input.value.trim().replace(/,$/g, "");
        if (val) {
          const currentTags = new Set(
            Array.from(
              chipContainer.querySelectorAll<HTMLElement>(".tag-chip")
            ).map((c) => c.dataset.tag || "")
          );
          if (!currentTags.has(val)) {
            this.addTagChip(chipContainer, val);
            this.checkDirty();
          }
          input.value = "";
          suggestions.style.display = "none";
        }
      }
    });

    inputWrapper.appendChild(input);
    inputWrapper.appendChild(suggestions);
    tagEditor.appendChild(inputWrapper);

    container.appendChild(tagEditor);
  }

  private addTagChip(container: HTMLElement, tagName: string): void {
    const chip = document.createElement("span");
    chip.className = "tag-chip";
    chip.dataset.tag = tagName;

    const text = document.createElement("span");
    text.textContent = tagName;
    chip.appendChild(text);

    const removeBtn = document.createElement("button");
    removeBtn.className = "tag-chip-remove";
    removeBtn.textContent = "\u00D7";
    removeBtn.addEventListener("click", () => {
      chip.remove();
      this.checkDirty();
    });
    chip.appendChild(removeBtn);

    container.appendChild(chip);
  }

  private async save(): Promise<void> {
    if (!this.currentFile || !this.dirty || this.saving) return;

    // Capture the file ID at the start so we can detect selection changes
    const saveFileId = this.currentFile.id;

    this.saving = true;
    const saveBtn = this.el.querySelector<HTMLButtonElement>(".metadata-save-btn");
    if (saveBtn) {
      saveBtn.disabled = true;
      saveBtn.textContent = "Speichern...";
    }

    try {
      const values = this.getCurrentFormValues();
      const updates: FileUpdate = {};
      let hasUpdates = false;

      if (this.snapshot) {
        if (values.name !== this.snapshot.name) {
          updates.name = values.name;
          hasUpdates = true;
        }
        if (values.theme !== this.snapshot.theme) {
          updates.theme = values.theme;
          hasUpdates = true;
        }
        if (values.description !== this.snapshot.description) {
          updates.description = values.description;
          hasUpdates = true;
        }
        if (values.license !== this.snapshot.license) {
          updates.license = values.license;
          hasUpdates = true;
        }
      }

      const tagsChanged =
        this.snapshot &&
        values.tags.join(",") !== this.snapshot.tags.join(",");

      if (hasUpdates) {
        const updatedFile = await FileService.updateFile(saveFileId, updates);

        // Abort if user selected a different file while we were saving
        if (this.currentFile?.id !== saveFileId) {
          if (saveBtn) saveBtn.textContent = "Speichern";
          return;
        }

        // Safe to update: onSelectionChanged listens to "selectedFileId", not "files",
        // so this assignment cannot be overwritten synchronously by the update below.
        this.currentFile = updatedFile;

        // Atomically update only this file in the files array
        appState.update("files", (files) =>
          files.map((f) => (f.id === updatedFile.id ? updatedFile : f))
        );
      }

      if (tagsChanged) {
        const newTags = await FileService.setTags(saveFileId, values.tags);

        // Abort if user selected a different file while we were saving
        if (this.currentFile?.id !== saveFileId) {
          if (saveBtn) saveBtn.textContent = "Speichern";
          return;
        }

        this.currentTags = newTags;
      }

      this.snapshot = this.takeSnapshot(this.currentFile, this.currentTags);
      this.dirty = false;

      EventBus.emit("file:saved", { fileId: saveFileId });

      if (saveBtn) {
        saveBtn.textContent = "Gespeichert!";
        setTimeout(() => {
          if (saveBtn) saveBtn.textContent = "Speichern";
        }, 1500);
      }
    } catch (e) {
      console.warn("Failed to save file:", e);
      if (saveBtn) {
        saveBtn.textContent = "Fehler!";
        setTimeout(() => {
          if (saveBtn) saveBtn.textContent = "Speichern";
        }, 2000);
      }
    } finally {
      this.saving = false;
      this.checkDirty();
    }
  }

  private renderCustomField(container: HTMLElement, field: CustomFieldDef): void {
    const group = document.createElement("div");
    group.className = "metadata-form-group";

    const label = document.createElement("label");
    label.className = "metadata-form-label";
    label.textContent = field.name;
    group.appendChild(label);

    if (field.fieldType === "select" && field.options) {
      const select = document.createElement("select");
      select.className = "metadata-form-input";
      select.dataset.customField = String(field.id);

      const emptyOpt = document.createElement("option");
      emptyOpt.value = "";
      emptyOpt.textContent = "— Auswählen —";
      select.appendChild(emptyOpt);

      for (const opt of field.options.split(",")) {
        const option = document.createElement("option");
        option.value = opt.trim();
        option.textContent = opt.trim();
        select.appendChild(option);
      }

      group.appendChild(select);
    } else {
      const input = document.createElement("input");
      input.type = field.fieldType === "number" ? "number" : field.fieldType === "date" ? "date" : "text";
      input.className = "metadata-form-input";
      input.dataset.customField = String(field.id);
      group.appendChild(input);
    }

    container.appendChild(group);
  }

  private async loadStitchPreview(
    canvas: HTMLCanvasElement,
    filepath: string,
    fileId: number,
    zoomLabel: HTMLElement,
    controls: {
      zoomInBtn: HTMLButtonElement;
      zoomOutBtn: HTMLButtonElement;
      zoomResetBtn: HTMLButtonElement;
    }
  ): Promise<void> {
    let segments: StitchSegment[];
    try {
      segments = await FileService.getStitchSegments(filepath);
    } catch {
      return;
    }
    if (this.currentFile?.id !== fileId || segments.length === 0) return;

    // Compute bounding box
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (const seg of segments) {
      for (const [x, y] of seg.points) {
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
      }
    }
    const dataW = maxX - minX;
    const dataH = maxY - minY;
    if (dataW <= 0 || dataH <= 0) return;

    const padding = 16;

    let zoom = 1;
    let panX = 0;
    let panY = 0;

    const drawPreview = () => {
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      const dpr = window.devicePixelRatio || 1;
      const displayW = canvas.clientWidth;
      const displayH = canvas.clientHeight;
      const targetW = Math.round(displayW * dpr);
      const targetH = Math.round(displayH * dpr);
      if (canvas.width !== targetW || canvas.height !== targetH) {
        canvas.width = targetW;
        canvas.height = targetH;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

      // Background — use container's computed background for theme compatibility
      const bgColor = getComputedStyle(canvas.parentElement || canvas).backgroundColor || "#ffffff";
      ctx.fillStyle = bgColor;
      ctx.fillRect(0, 0, displayW, displayH);

      ctx.save();
      ctx.translate(panX, panY);
      ctx.scale(zoom, zoom);

      // Recalculate base transform for current display size
      const curDrawW = displayW - 2 * padding;
      const curDrawH = displayH - 2 * padding;
      const curScale = Math.min(curDrawW / dataW, curDrawH / dataH);
      const curOffX = padding + (curDrawW - dataW * curScale) / 2;
      const curOffY = padding + (curDrawH - dataH * curScale) / 2;

      ctx.lineWidth = Math.max(1, 1.5 / zoom);
      ctx.lineCap = "round";
      ctx.lineJoin = "round";

      for (const seg of segments) {
        if (seg.points.length < 2) continue;
        ctx.strokeStyle = seg.colorHex || "#000000";
        ctx.beginPath();
        const [sx, sy] = seg.points[0];
        ctx.moveTo((sx - minX) * curScale + curOffX, (sy - minY) * curScale + curOffY);
        for (let i = 1; i < seg.points.length; i++) {
          const [px, py] = seg.points[i];
          ctx.lineTo((px - minX) * curScale + curOffX, (py - minY) * curScale + curOffY);
        }
        ctx.stroke();
      }

      ctx.restore();
      zoomLabel.textContent = `${Math.round(zoom * 100)}%`;
    };

    drawPreview();

    // Zoom with mouse wheel
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const rect = canvas.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;
      const mouseY = e.clientY - rect.top;

      const oldZoom = zoom;
      const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
      zoom = Math.min(10, Math.max(0.25, zoom * factor));

      // Zoom toward cursor
      panX = mouseX - (mouseX - panX) * (zoom / oldZoom);
      panY = mouseY - (mouseY - panY) * (zoom / oldZoom);
      drawPreview();
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });

    // Pan with mouse drag
    let dragging = false;
    let lastX = 0, lastY = 0;
    canvas.addEventListener("mousedown", (e) => {
      dragging = true;
      lastX = e.clientX;
      lastY = e.clientY;
      canvas.style.cursor = "grabbing";
    });
    const onMouseMove = (e: MouseEvent) => {
      if (!dragging) return;
      panX += e.clientX - lastX;
      panY += e.clientY - lastY;
      lastX = e.clientX;
      lastY = e.clientY;
      drawPreview();
    };
    const onMouseUp = () => {
      dragging = false;
      canvas.style.cursor = "grab";
    };
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
    canvas.style.cursor = "grab";

    // Register cleanup for document-level listeners
    this.previewCleanup = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };

    // Double-click to reset
    canvas.addEventListener("dblclick", () => {
      zoom = 1;
      panX = 0;
      panY = 0;
      drawPreview();
    });

    // Zoom buttons
    controls.zoomInBtn.addEventListener("click", () => {
      const center = canvas.clientWidth / 2;
      const centerY = canvas.clientHeight / 2;
      const oldZoom = zoom;
      zoom = Math.min(10, zoom * 1.3);
      panX = center - (center - panX) * (zoom / oldZoom);
      panY = centerY - (centerY - panY) * (zoom / oldZoom);
      drawPreview();
    });
    controls.zoomOutBtn.addEventListener("click", () => {
      const center = canvas.clientWidth / 2;
      const centerY = canvas.clientHeight / 2;
      const oldZoom = zoom;
      zoom = Math.max(0.25, zoom / 1.3);
      panX = center - (center - panX) * (zoom / oldZoom);
      panY = centerY - (centerY - panY) * (zoom / oldZoom);
      drawPreview();
    });
    controls.zoomResetBtn.addEventListener("click", () => {
      zoom = 1;
      panX = 0;
      panY = 0;
      drawPreview();
    });
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

}
