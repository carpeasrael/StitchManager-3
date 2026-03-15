import { Component } from "./Component";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { getFormatLabel, formatSize } from "../utils/format";
import * as FileService from "../services/FileService";
import * as SettingsService from "../services/SettingsService";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { ToastContainer } from "./Toast";
import { TagInput } from "./TagInput";
import { ImagePreviewDialog } from "./ImagePreviewDialog";
import { open } from "@tauri-apps/plugin-dialog";
import * as ThreadColorService from "../services/ThreadColorService";
import type {
  EmbroideryFile,
  FileAttachment,
  ThreadColor,
  ThreadMatch,
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
  sizeRange: string;
  skillLevel: string;
  language: string;
  formatType: string;
  fileSource: string;
  purchaseLink: string;
  status: string;
}

export class MetadataPanel extends Component {
  private currentFile: EmbroideryFile | null = null;
  private currentTags: Tag[] = [];
  private allTags: Tag[] = [];
  private customFields: CustomFieldDef[] = [];
  private customFieldValues: Record<number, string> = {};
  private snapshot: FormSnapshot | null = null;
  private dirty = false;
  private saving = false;
  private previewCleanup: (() => void) | null = null;
  private previewGeneration = 0;
  private tagInput: TagInput | null = null;
  private currentSegments: StitchSegment[] = [];

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("selectedFileId", () => this.onSelectionChanged(false))
    );
    this.subscribe(
      EventBus.on("file:refresh", () => this.onSelectionChanged(true))
    );
    this.subscribe(
      EventBus.on("metadata:save", () => this.save())
    );
    this.render();
  }

  destroy(): void {
    // Increment previewGeneration to cancel any in-flight loadStitchPreview
    this.previewGeneration++;
    if (this.previewCleanup) { this.previewCleanup(); this.previewCleanup = null; }
    if (this.tagInput) { this.tagInput.destroy(); this.tagInput = null; }
    super.destroy();
  }

  private async onSelectionChanged(force: boolean): Promise<void> {
    const fileId = appState.get("selectedFileId");
    if (!force && fileId !== null && fileId === this.currentFile?.id) return;
    if (fileId === null) {
      this.currentFile = null;
      this.currentTags = [];
      this.snapshot = null;
      this.dirty = false;
      this.render();
      return;
    }

    try {
      const [file, formats, colors, tags, allTags, customFields, attachments, customFieldValues] = await Promise.all([
        FileService.getFile(fileId),
        FileService.getFormats(fileId),
        FileService.getColors(fileId),
        FileService.getTags(fileId),
        FileService.getAllTags(),
        SettingsService.getCustomFields(),
        FileService.getAttachments(fileId),
        SettingsService.getCustomFieldValues(fileId),
      ]);
      this.currentFile = file;
      this.currentTags = tags;
      this.allTags = allTags;
      this.customFields = customFields;
      this.customFieldValues = customFieldValues;
      this.snapshot = this.takeSnapshot(file, tags);
      this.dirty = false;
      this.renderFileInfo(file, formats, colors, tags, attachments);
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
      sizeRange: file.sizeRange || "",
      skillLevel: file.skillLevel || "",
      language: file.language || "",
      formatType: file.formatType || "",
      fileSource: file.fileSource || "",
      purchaseLink: file.purchaseLink || "",
      status: file.status || "none",
    };
  }

  private checkDirty(): void {
    if (!this.snapshot) {
      this.dirty = false;
      return;
    }
    const current = this.getCurrentFormValues();
    let dirty =
      current.name !== this.snapshot.name ||
      current.theme !== this.snapshot.theme ||
      current.description !== this.snapshot.description ||
      current.license !== this.snapshot.license ||
      JSON.stringify(current.tags) !== JSON.stringify(this.snapshot.tags) ||
      current.sizeRange !== this.snapshot.sizeRange ||
      current.skillLevel !== this.snapshot.skillLevel ||
      current.language !== this.snapshot.language ||
      current.formatType !== this.snapshot.formatType ||
      current.fileSource !== this.snapshot.fileSource ||
      current.purchaseLink !== this.snapshot.purchaseLink ||
      current.status !== this.snapshot.status;

    // Check custom fields for changes
    if (!dirty) {
      const customInputs = this.el.querySelectorAll<HTMLInputElement | HTMLSelectElement>("[data-custom-field]");
      customInputs.forEach((el) => {
        const fieldId = Number(el.dataset.customField);
        if (!isNaN(fieldId) && el.value !== (this.customFieldValues[fieldId] || "")) {
          dirty = true;
        }
      });
    }

    this.dirty = dirty;
    const saveBtn = this.el.querySelector<HTMLButtonElement>(".metadata-save-btn");
    if (saveBtn) {
      saveBtn.disabled = !this.dirty || this.saving;
    }
  }

  private getCurrentFormValues(): FormSnapshot {
    const getValue = (name: string): string => {
      const el = this.el.querySelector<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>(
        `[data-field="${name}"]`
      );
      return el ? el.value : "";
    };

    const tags: string[] = this.tagInput ? this.tagInput.getTags() : [];

    return {
      name: getValue("name"),
      theme: getValue("theme"),
      description: getValue("description"),
      license: getValue("license"),
      tags: tags.sort(),
      sizeRange: getValue("sizeRange"),
      skillLevel: getValue("skillLevel"),
      language: getValue("language"),
      formatType: getValue("formatType"),
      fileSource: getValue("fileSource"),
      purchaseLink: getValue("purchaseLink"),
      status: getValue("status"),
    };
  }

  render(): void {
    if (this.previewCleanup) { this.previewCleanup(); this.previewCleanup = null; }
    if (this.tagInput) { this.tagInput.destroy(); this.tagInput = null; }
    this.currentSegments = [];
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
    tags: Tag[],
    attachments: FileAttachment[] = []
  ): void {
    if (this.previewCleanup) { this.previewCleanup(); this.previewCleanup = null; }
    if (this.tagInput) { this.tagInput.destroy(); this.tagInput = null; }
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
    zoomInBtn.setAttribute("aria-label", "Vergr\u00F6\u00DFern");
    const zoomOutBtn = document.createElement("button");
    zoomOutBtn.className = "stitch-preview-btn";
    zoomOutBtn.textContent = "\u2212";
    zoomOutBtn.title = "Verkleinern";
    zoomOutBtn.setAttribute("aria-label", "Verkleinern");
    const zoomResetBtn = document.createElement("button");
    zoomResetBtn.className = "stitch-preview-btn";
    zoomResetBtn.textContent = "\u21BA";
    zoomResetBtn.title = "Zur\u00FCcksetzen";
    zoomResetBtn.setAttribute("aria-label", "Zur\u00FCcksetzen");
    const zoomLabel = document.createElement("span");
    zoomLabel.className = "stitch-preview-zoom-label";
    zoomLabel.textContent = "100%";
    controls.appendChild(zoomInBtn);
    controls.appendChild(zoomOutBtn);
    controls.appendChild(zoomResetBtn);
    controls.appendChild(zoomLabel);
    previewContainer.appendChild(controls);

    // Click on preview to open full-screen image dialog
    const expandBtn = document.createElement("button");
    expandBtn.className = "stitch-preview-btn stitch-preview-expand";
    expandBtn.textContent = "\u2922";
    expandBtn.title = "Vollbild";
    expandBtn.setAttribute("aria-label", "Vollbild");
    expandBtn.addEventListener("click", () => {
      if (this.currentSegments.length > 0) {
        ImagePreviewDialog.open(this.currentSegments);
      }
    });
    controls.appendChild(expandBtn);

    previewSection.appendChild(previewContainer);
    wrapper.appendChild(previewSection);

    // Load stitch segments and render on canvas
    this.currentSegments = [];
    const previewFileId = file.id;
    if (file.filepath) {
      this.loadStitchPreview(canvas, file.filepath, previewFileId, zoomLabel, {
        zoomInBtn, zoomOutBtn, zoomResetBtn,
      }).catch(() => { /* keep empty canvas */ });
    }

    // "View document" button for PDFs and viewable files
    const fileExt = file.filepath?.split(".").pop()?.toLowerCase() || "";
    if (["pdf", "png", "jpg", "jpeg", "svg", "bmp", "gif", "webp"].includes(fileExt)) {
      const viewBar = document.createElement("div");
      viewBar.className = "metadata-view-bar";
      const viewBtn = document.createElement("button");
      viewBtn.className = "metadata-view-btn";
      viewBtn.textContent = fileExt === "pdf" ? "Dokument anzeigen" : "Bild anzeigen";
      viewBtn.addEventListener("click", () => {
        EventBus.emit("viewer:open", {
          filePath: file.filepath,
          fileId: file.id,
          fileName: file.name || file.filename,
        });
      });
      viewBar.appendChild(viewBtn);
      wrapper.appendChild(viewBar);
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

    const usbBtn = document.createElement("button");
    usbBtn.className = "metadata-action-btn";
    usbBtn.textContent = "\uD83D\uDCE4 USB-Export";
    usbBtn.addEventListener("click", () => {
      EventBus.emit("toolbar:batch-export");
    });
    aiBar.appendChild(usbBtn);

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

    const tagContainer = document.createElement("div");
    tagSection.appendChild(tagContainer);
    this.tagInput = new TagInput(tagContainer, {
      allTags: this.allTags.map((t) => t.name),
      selectedTags: tags.map((t) => t.name),
      placeholder: "Tag hinzufügen...",
      onChange: () => this.checkDirty(),
    });
    wrapper.appendChild(tagSection);

    // Status section (visible for all file types)
    const statusSection = document.createElement("div");
    statusSection.className = "metadata-section";
    const statusHeader = document.createElement("div");
    statusHeader.className = "metadata-section-header";
    statusHeader.textContent = "Status";
    statusSection.appendChild(statusHeader);
    const statusForm = document.createElement("div");
    statusForm.className = "metadata-form";
    this.addSelectField(statusForm, "Status", "status", file.status || "none", [
      { value: "none", label: "Keiner" },
      { value: "not_started", label: "Nicht begonnen" },
      { value: "planned", label: "Geplant" },
      { value: "in_progress", label: "In Arbeit" },
      { value: "completed", label: "Fertig" },
      { value: "archived", label: "Archiviert" },
    ]);
    statusSection.appendChild(statusForm);
    wrapper.appendChild(statusSection);

    // Sewing pattern fields (only for sewing_pattern file type)
    if (file.fileType === "sewing_pattern") {
      const sewingSection = document.createElement("div");
      sewingSection.className = "metadata-section";
      const sewingHeader = document.createElement("div");
      sewingHeader.className = "metadata-section-header";
      sewingHeader.textContent = "Schnittmuster";
      sewingSection.appendChild(sewingHeader);
      const sewingForm = document.createElement("div");
      sewingForm.className = "metadata-form";
      this.addFormField(sewingForm, "Größen", "sizeRange", file.sizeRange || "", "text");
      this.addSelectField(sewingForm, "Schwierigkeit", "skillLevel", file.skillLevel || "", [
        { value: "", label: "-- Auswählen --" },
        { value: "beginner", label: "Anfänger" },
        { value: "easy", label: "Einfach" },
        { value: "intermediate", label: "Mittel" },
        { value: "advanced", label: "Fortgeschritten" },
        { value: "expert", label: "Experte" },
      ]);
      this.addFormField(sewingForm, "Sprache", "language", file.language || "", "text");
      this.addFormField(sewingForm, "Formattyp", "formatType", file.formatType || "", "text");
      this.addFormField(sewingForm, "Quelle", "fileSource", file.fileSource || "", "text");
      this.addLinkField(sewingForm, "Kauflink", "purchaseLink", file.purchaseLink || "");
      sewingSection.appendChild(sewingForm);
      wrapper.appendChild(sewingSection);
    }

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

    if (file.uniqueId) {
      this.addCopyableInfoRow(infoGrid, "ID", file.uniqueId);
    }
    this.addInfoRow(infoGrid, "Dateiname", file.filename);
    if (file.filepath) {
      this.addClickableInfoRow(infoGrid, "Speicherort", file.filepath);
    }
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

    if (file.pageCount !== null) {
      this.addInfoRow(infoGrid, "Seiten", String(file.pageCount));
    }

    if (file.paperSize) {
      this.addInfoRow(infoGrid, "Papierformat", file.paperSize);
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
        const swatchContainer = document.createElement("div");
        swatchContainer.className = "metadata-swatch-container";

        const swatch = document.createElement("div");
        swatch.className = "metadata-swatch metadata-swatch-expandable";

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

        const expandIcon = document.createElement("span");
        expandIcon.className = "metadata-swatch-expand";
        expandIcon.textContent = "\u25B6";
        swatch.appendChild(expandIcon);

        swatchContainer.appendChild(swatch);

        // Expandable matches container (lazy-loaded)
        const matchesContainer = document.createElement("div");
        matchesContainer.className = "metadata-swatch-matches";
        matchesContainer.style.display = "none";
        swatchContainer.appendChild(matchesContainer);

        let expanded = false;
        let loaded = false;

        swatch.addEventListener("click", async () => {
          expanded = !expanded;
          matchesContainer.style.display = expanded ? "" : "none";
          expandIcon.textContent = expanded ? "\u25BC" : "\u25B6";

          if (expanded && !loaded && validHex) {
            loaded = true;
            matchesContainer.innerHTML =
              '<div class="metadata-swatch-loading">Suche Garnfarben...</div>';
            try {
              const matches = await ThreadColorService.getThreadMatches(
                color.colorHex,
                undefined,
                9
              );
              this.renderThreadMatches(matchesContainer, matches);
            } catch {
              matchesContainer.innerHTML =
                '<div class="metadata-swatch-loading">Fehler beim Laden</div>';
            }
          }
        });

        swatchGrid.appendChild(swatchContainer);
      }

      colorSection.appendChild(swatchGrid);
    } else {
      const noColors = document.createElement("div");
      noColors.className = "metadata-no-colors";
      noColors.textContent = "Keine Farbinformationen";
      colorSection.appendChild(noColors);
    }

    wrapper.appendChild(colorSection);

    // Attachments section
    const attachSection = document.createElement("div");
    attachSection.className = "metadata-section";

    const attachHeader = document.createElement("div");
    attachHeader.className = "metadata-section-header";
    attachHeader.textContent = "Anh\u00E4nge";
    attachSection.appendChild(attachHeader);

    const attachList = document.createElement("div");
    attachList.className = "metadata-attachments";

    for (const att of attachments) {
      const item = document.createElement("div");
      item.className = "metadata-attachment-item";

      const nameEl = document.createElement("span");
      nameEl.className = "metadata-attachment-name";
      nameEl.textContent = att.displayName || att.filename;
      nameEl.title = att.filename;
      nameEl.addEventListener("click", () => {
        FileService.openAttachment(att.id).catch((e) => {
          console.warn("Failed to open attachment:", e);
          ToastContainer.show("error", "Anhang konnte nicht ge\u00F6ffnet werden");
        });
      });
      item.appendChild(nameEl);

      const typeLabels: Record<string, string> = {
        pattern: "Schnittmuster", instruction: "Anleitung", cover_image: "Titelbild",
        measurement_chart: "Ma\u00DFtabelle", fabric_requirements: "Stoffbedarf",
        notes: "Notizen", license: "Lizenz", other: "Sonstiges",
      };
      const typeEl = document.createElement("span");
      typeEl.className = "metadata-attachment-type";
      typeEl.textContent = typeLabels[att.attachmentType] || att.attachmentType;
      item.appendChild(typeEl);

      // "View in app" button for viewable file types
      const viewableExts = ["pdf", "png", "jpg", "jpeg", "svg", "gif", "webp", "bmp"];
      const attExt = att.filename.split(".").pop()?.toLowerCase() || "";
      if (viewableExts.includes(attExt)) {
        const viewBtn = document.createElement("button");
        viewBtn.className = "metadata-attachment-view";
        viewBtn.textContent = "Anzeigen";
        viewBtn.title = "Im App anzeigen";
        viewBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          EventBus.emit("viewer:open", {
            filePath: att.filePath,
            fileId: this.currentFile!.id,
            fileName: att.displayName || att.filename,
          });
        });
        item.appendChild(viewBtn);
      }

      const delBtn = document.createElement("button");
      delBtn.className = "metadata-attachment-delete";
      delBtn.textContent = "\u00D7";
      delBtn.title = "Anhang entfernen";
      delBtn.setAttribute("aria-label", `Anhang ${att.filename} entfernen`);
      delBtn.addEventListener("click", async () => {
        try {
          await FileService.deleteAttachment(att.id);
          item.remove();
          ToastContainer.show("success", "Anhang entfernt");
          EventBus.emit("file:refresh");
        } catch (e) {
          console.warn("Failed to delete attachment:", e);
          ToastContainer.show("error", "Anhang konnte nicht entfernt werden");
        }
      });
      item.appendChild(delBtn);

      attachList.appendChild(item);
    }

    attachSection.appendChild(attachList);

    const addAttachBtn = document.createElement("button");
    addAttachBtn.className = "metadata-action-btn";
    addAttachBtn.textContent = "\uD83D\uDCCE Anhang hinzuf\u00FCgen";
    addAttachBtn.addEventListener("click", () => this.addAttachment());
    attachSection.appendChild(addAttachBtn);

    wrapper.appendChild(attachSection);

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

  private addSelectField(
    container: HTMLElement,
    label: string,
    field: string,
    value: string,
    options: { value: string; label: string }[]
  ): void {
    const group = document.createElement("div");
    group.className = "metadata-form-group";

    const labelEl = document.createElement("label");
    labelEl.className = "metadata-form-label";
    labelEl.textContent = label;
    group.appendChild(labelEl);

    const select = document.createElement("select");
    select.className = "metadata-form-input";
    select.dataset.field = field;

    for (const opt of options) {
      const optEl = document.createElement("option");
      optEl.value = opt.value;
      optEl.textContent = opt.label;
      select.appendChild(optEl);
    }

    select.value = value;
    select.addEventListener("change", () => this.checkDirty());
    group.appendChild(select);
    container.appendChild(group);
  }

  private addLinkField(
    container: HTMLElement,
    label: string,
    field: string,
    value: string
  ): void {
    const group = document.createElement("div");
    group.className = "metadata-form-group";

    const labelEl = document.createElement("label");
    labelEl.className = "metadata-form-label";
    labelEl.textContent = label;
    group.appendChild(labelEl);

    const row = document.createElement("div");
    row.style.display = "flex";
    row.style.gap = "var(--spacing-1)";
    row.style.alignItems = "center";

    const input = document.createElement("input");
    input.type = "url";
    input.className = "metadata-form-input";
    input.dataset.field = field;
    input.value = value;
    input.placeholder = "https://...";
    input.style.flex = "1";
    input.addEventListener("input", () => this.checkDirty());
    row.appendChild(input);

    if (value && /^https?:\/\//i.test(value)) {
      const linkBtn = document.createElement("a");
      linkBtn.className = "metadata-form-input";
      linkBtn.style.display = "inline-flex";
      linkBtn.style.alignItems = "center";
      linkBtn.style.justifyContent = "center";
      linkBtn.style.width = "auto";
      linkBtn.style.padding = "var(--spacing-1) var(--spacing-2)";
      linkBtn.style.textDecoration = "none";
      linkBtn.style.cursor = "pointer";
      linkBtn.href = value;
      linkBtn.target = "_blank";
      linkBtn.rel = "noopener noreferrer";
      linkBtn.textContent = "\u2197";
      linkBtn.title = "Link öffnen";
      row.appendChild(linkBtn);
    }

    group.appendChild(row);
    container.appendChild(group);
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
        if (values.sizeRange !== this.snapshot.sizeRange) {
          updates.sizeRange = values.sizeRange;
          hasUpdates = true;
        }
        if (values.skillLevel !== this.snapshot.skillLevel) {
          updates.skillLevel = values.skillLevel;
          hasUpdates = true;
        }
        if (values.language !== this.snapshot.language) {
          updates.language = values.language;
          hasUpdates = true;
        }
        if (values.formatType !== this.snapshot.formatType) {
          updates.formatType = values.formatType;
          hasUpdates = true;
        }
        if (values.fileSource !== this.snapshot.fileSource) {
          updates.fileSource = values.fileSource;
          hasUpdates = true;
        }
        if (values.purchaseLink !== this.snapshot.purchaseLink) {
          updates.purchaseLink = values.purchaseLink;
          hasUpdates = true;
        }
        if (values.status !== this.snapshot.status) {
          updates.status = values.status;
          hasUpdates = true;
        }
      }

      const tagsChanged =
        this.snapshot &&
        JSON.stringify(values.tags) !== JSON.stringify(this.snapshot.tags);

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

      // Save custom field values
      const customFieldInputs = this.el.querySelectorAll<HTMLInputElement | HTMLSelectElement>("[data-custom-field]");
      if (customFieldInputs.length > 0) {
        const cfValues: Record<number, string> = {};
        customFieldInputs.forEach((el) => {
          const fieldId = Number(el.dataset.customField);
          if (!isNaN(fieldId)) {
            cfValues[fieldId] = el.value;
          }
        });
        await SettingsService.setCustomFieldValues(saveFileId, cfValues);
        this.customFieldValues = cfValues;
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

    const existingValue = this.customFieldValues[field.id] || "";

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

      select.value = existingValue;
      select.addEventListener("input", () => this.checkDirty());
      group.appendChild(select);
    } else {
      const input = document.createElement("input");
      input.type = field.fieldType === "number" ? "number" : field.fieldType === "date" ? "date" : "text";
      input.className = "metadata-form-input";
      input.dataset.customField = String(field.id);
      input.value = existingValue;
      input.addEventListener("input", () => this.checkDirty());
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
    const gen = ++this.previewGeneration;
    let segments: StitchSegment[];
    try {
      segments = await FileService.getStitchSegments(filepath);
    } catch {
      return;
    }
    if (gen !== this.previewGeneration || this.currentFile?.id !== fileId || segments.length === 0) return;

    this.currentSegments = segments;

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
    const onMouseDown = (e: MouseEvent) => {
      dragging = true;
      lastX = e.clientX;
      lastY = e.clientY;
      canvas.style.cursor = "grabbing";
    };
    canvas.addEventListener("mousedown", onMouseDown);
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

    // Double-click to reset
    const onDblClick = () => {
      zoom = 1;
      panX = 0;
      panY = 0;
      drawPreview();
    };
    canvas.addEventListener("dblclick", onDblClick);

    // Zoom buttons
    const onZoomIn = () => {
      const center = canvas.clientWidth / 2;
      const centerY = canvas.clientHeight / 2;
      const oldZoom = zoom;
      zoom = Math.min(10, zoom * 1.3);
      panX = center - (center - panX) * (zoom / oldZoom);
      panY = centerY - (centerY - panY) * (zoom / oldZoom);
      drawPreview();
    };
    controls.zoomInBtn.addEventListener("click", onZoomIn);
    const onZoomOut = () => {
      const center = canvas.clientWidth / 2;
      const centerY = canvas.clientHeight / 2;
      const oldZoom = zoom;
      zoom = Math.max(0.25, zoom / 1.3);
      panX = center - (center - panX) * (zoom / oldZoom);
      panY = centerY - (centerY - panY) * (zoom / oldZoom);
      drawPreview();
    };
    controls.zoomOutBtn.addEventListener("click", onZoomOut);
    const onZoomReset = () => {
      zoom = 1;
      panX = 0;
      panY = 0;
      drawPreview();
    };
    controls.zoomResetBtn.addEventListener("click", onZoomReset);

    // Register cleanup for ALL listeners (canvas + document + buttons)
    this.previewCleanup = () => {
      canvas.removeEventListener("wheel", onWheel);
      canvas.removeEventListener("mousedown", onMouseDown);
      canvas.removeEventListener("dblclick", onDblClick);
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      controls.zoomInBtn.removeEventListener("click", onZoomIn);
      controls.zoomOutBtn.removeEventListener("click", onZoomOut);
      controls.zoomResetBtn.removeEventListener("click", onZoomReset);
    };
  }

  private renderThreadMatches(
    container: HTMLElement,
    matches: ThreadMatch[]
  ): void {
    container.innerHTML = "";

    if (matches.length === 0) {
      container.innerHTML =
        '<div class="metadata-swatch-loading">Keine Treffer gefunden</div>';
      return;
    }

    // Group by brand, show best match per brand
    const byBrand = new Map<string, ThreadMatch>();
    for (const m of matches) {
      if (!byBrand.has(m.brand)) {
        byBrand.set(m.brand, m);
      }
    }

    for (const [, match] of byBrand) {
      const row = document.createElement("div");
      row.className = "metadata-swatch-match";

      const matchColor = document.createElement("div");
      matchColor.className = "metadata-swatch-match-color";
      matchColor.style.backgroundColor = match.hex;
      row.appendChild(matchColor);

      const matchInfo = document.createElement("div");
      matchInfo.className = "metadata-swatch-match-info";

      const codeEl = document.createElement("span");
      codeEl.className = "metadata-swatch-match-code";
      codeEl.textContent = `${match.brand} ${match.code}`;
      codeEl.title = "Klicken zum Suchen";
      codeEl.addEventListener("click", (e) => {
        e.stopPropagation();
        appState.set("searchQuery", `${match.brand} ${match.code}`);
        appState.update("searchParams", (sp) => ({ ...sp, colorSearch: match.hex }));
      });
      matchInfo.appendChild(codeEl);

      const nameEl = document.createElement("span");
      nameEl.className = "metadata-swatch-match-name";
      nameEl.textContent = match.name;
      matchInfo.appendChild(nameEl);

      row.appendChild(matchInfo);

      const deltaEl = document.createElement("span");
      deltaEl.className = "metadata-swatch-delta";
      deltaEl.textContent = `\u0394E ${match.deltaE.toFixed(1)}`;
      row.appendChild(deltaEl);

      container.appendChild(row);
    }
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

  private addCopyableInfoRow(grid: HTMLElement, label: string, value: string): void {
    const row = document.createElement("div");
    row.className = "metadata-info-row";

    const labelEl = document.createElement("span");
    labelEl.className = "metadata-info-label";
    labelEl.textContent = label;
    row.appendChild(labelEl);

    const valueWrapper = document.createElement("span");
    valueWrapper.className = "metadata-info-value metadata-info-copyable";

    const valueEl = document.createElement("span");
    valueEl.textContent = value;
    valueWrapper.appendChild(valueEl);

    const copyBtn = document.createElement("button");
    copyBtn.className = "metadata-copy-btn";
    copyBtn.textContent = "\uD83D\uDCCB";
    copyBtn.title = "Kopieren";
    copyBtn.setAttribute("aria-label", `${label} kopieren`);
    copyBtn.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(value);
        ToastContainer.show("success", "ID kopiert");
      } catch {
        ToastContainer.show("error", "Kopieren fehlgeschlagen");
      }
    });
    valueWrapper.appendChild(copyBtn);

    row.appendChild(valueWrapper);
    grid.appendChild(row);
  }

  private async addAttachment(): Promise<void> {
    if (!this.currentFile) return;
    const fileId = this.currentFile.id;

    // Show type selector first
    const attachmentType = await this.showAttachmentTypeSelector();
    if (!attachmentType) return;

    try {
      const selected = await open({
        multiple: false,
        title: "Anhang ausw\u00E4hlen",
      });
      if (!selected) return;

      const path = typeof selected === "string" ? selected : String(selected);
      if (!path) return;

      await FileService.attachFile(fileId, path, attachmentType);
      ToastContainer.show("success", "Anhang hinzugef\u00FCgt");

      // Refresh the panel to show the new attachment
      EventBus.emit("file:refresh");
    } catch (e) {
      console.warn("Failed to attach file:", e);
      ToastContainer.show("error", "Anhang konnte nicht hinzugef\u00FCgt werden");
    }
  }

  private showAttachmentTypeSelector(): Promise<string | null> {
    return new Promise((resolve) => {
      const overlay = document.createElement("div");
      overlay.className = "dialog-overlay";
      overlay.style.display = "flex";

      const dialog = document.createElement("div");
      dialog.className = "dialog-content";
      dialog.style.maxWidth = "320px";
      dialog.style.padding = "var(--spacing-4)";

      const title = document.createElement("h3");
      title.style.margin = "0 0 var(--spacing-3) 0";
      title.textContent = "Anhangstyp w\u00E4hlen";
      dialog.appendChild(title);

      const types: { value: string; label: string }[] = [
        { value: "pattern", label: "Schnittmuster" },
        { value: "instruction", label: "Anleitung" },
        { value: "cover_image", label: "Titelbild" },
        { value: "measurement_chart", label: "Ma\u00DFtabelle" },
        { value: "fabric_requirements", label: "Stoffbedarf" },
        { value: "notes", label: "Notizen" },
        { value: "license", label: "Lizenz" },
        { value: "other", label: "Sonstiges" },
      ];

      for (const t of types) {
        const btn = document.createElement("button");
        btn.className = "metadata-action-btn";
        btn.style.display = "block";
        btn.style.width = "100%";
        btn.style.marginBottom = "var(--spacing-1)";
        btn.style.textAlign = "left";
        btn.textContent = t.label;
        btn.addEventListener("click", () => {
          overlay.remove();
          resolve(t.value);
        });
        dialog.appendChild(btn);
      }

      const cancelBtn = document.createElement("button");
      cancelBtn.className = "metadata-action-btn";
      cancelBtn.style.display = "block";
      cancelBtn.style.width = "100%";
      cancelBtn.style.marginTop = "var(--spacing-2)";
      cancelBtn.style.opacity = "0.7";
      cancelBtn.textContent = "Abbrechen";
      cancelBtn.addEventListener("click", () => {
        overlay.remove();
        resolve(null);
      });
      dialog.appendChild(cancelBtn);

      overlay.appendChild(dialog);
      overlay.addEventListener("click", (e) => {
        if (e.target === overlay) {
          overlay.remove();
          resolve(null);
        }
      });
      document.body.appendChild(overlay);
    });
  }

  private addClickableInfoRow(grid: HTMLElement, label: string, filepath: string): void {
    const row = document.createElement("div");
    row.className = "metadata-info-row";

    const labelEl = document.createElement("span");
    labelEl.className = "metadata-info-label";
    labelEl.textContent = label;
    row.appendChild(labelEl);

    const dirPath = filepath.replace(/[\\/][^\\/]+$/, "");
    const valueEl = document.createElement("span");
    valueEl.className = "metadata-info-value metadata-info-link";
    valueEl.textContent = dirPath;
    valueEl.title = "Im Dateimanager öffnen";
    valueEl.addEventListener("click", () => {
      revealItemInDir(filepath).catch((e) => {
        console.warn("Failed to reveal file:", e);
        ToastContainer.show("error", "Datei konnte nicht im Ordner angezeigt werden");
      });
    });
    row.appendChild(valueEl);

    grid.appendChild(row);
  }

}
