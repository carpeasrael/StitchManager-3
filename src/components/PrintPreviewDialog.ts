import * as pdfjs from "pdfjs-dist";
import type {
  PDFDocumentProxy,
} from "pdfjs-dist/types/src/display/api";
import * as ViewerService from "../services/ViewerService";
import * as PrintService from "../services/PrintService";
import { ToastContainer } from "./Toast";
import type { PrinterInfo, PrintSettings } from "../types";

// Reuse the same worker config as DocumentViewer
pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url
).href;

export class PrintPreviewDialog {
  private static instance: PrintPreviewDialog | null = null;

  private fileId = 0;
  private filePath = "";
  private fileName = "";
  private pdfDoc: PDFDocumentProxy | null = null;
  private totalPages = 0;
  private selectedPages = new Set<number>();
  private overlay: HTMLElement | null = null;
  private previewCanvas: HTMLCanvasElement | null = null;
  private previewContainer: HTMLElement | null = null;
  private printers: PrinterInfo[] = [];
  private keyHandler: ((e: KeyboardEvent) => void) | null = null;
  private layers: { id: string; name: string; visible: boolean }[] = [];
  private pageDimensions: Map<number, { widthMm: number; heightMm: number }> = new Map();

  // Settings
  private settings: PrintSettings = {
    printerName: null,
    paperSize: "A4",
    orientation: "auto",
    copies: 1,
    scale: 1.0,
    fitToPage: false,
    pageRanges: null,
    tileEnabled: false,
    tileOverlapMm: 15,
  };

  static async open(
    filePath: string,
    fileId: number,
    fileName: string
  ): Promise<void> {
    if (PrintPreviewDialog.instance) {
      PrintPreviewDialog.dismiss();
    }
    const dialog = new PrintPreviewDialog();
    PrintPreviewDialog.instance = dialog;
    await dialog.init(filePath, fileId, fileName);
  }

  static dismiss(): void {
    if (PrintPreviewDialog.instance) {
      PrintPreviewDialog.instance.close();
      PrintPreviewDialog.instance = null;
    }
  }

  private async init(
    filePath: string,
    fileId: number,
    fileName: string
  ): Promise<void> {
    this.fileId = fileId;
    this.filePath = filePath;
    this.fileName = fileName;

    // Load printers and saved settings in parallel
    const [printers, savedSettings] = await Promise.all([
      PrintService.getPrinters().catch(() => []),
      PrintService.loadPrintSettings().catch(() => ({} as Record<string, string>)),
    ]);
    this.printers = printers;

    // Apply saved settings
    if (savedSettings["print_paper_size"]) this.settings.paperSize = savedSettings["print_paper_size"];
    if (savedSettings["print_orientation"]) this.settings.orientation = savedSettings["print_orientation"];
    if (savedSettings["print_printer"]) this.settings.printerName = savedSettings["print_printer"];

    // Load PDF
    try {
      const bytes = await ViewerService.readFileBytes(filePath);
      this.pdfDoc = await pdfjs.getDocument({ data: bytes }).promise;
      this.totalPages = this.pdfDoc.numPages;

      // Select all pages by default
      for (let i = 1; i <= this.totalPages; i++) {
        this.selectedPages.add(i);
      }

      // Detect page dimensions for tiling detection
      for (let i = 1; i <= this.totalPages; i++) {
        const page = await this.pdfDoc.getPage(i);
        const vp = page.getViewport({ scale: 1.0 });
        this.pageDimensions.set(i, {
          widthMm: vp.width * 0.3528,
          heightMm: vp.height * 0.3528,
        });
      }

      // Detect OCG layers
      try {
        const occ = await this.pdfDoc.getOptionalContentConfig();
        if (occ) {
          // Access internal groups via the config object
          const occAny = occ as unknown as Record<string, unknown>;
          const groups = occAny["_groups"] as Map<string, Record<string, unknown>> | undefined;
          if (groups && groups instanceof Map) {
            for (const [id, group] of groups) {
              this.layers.push({
                id,
                name: (group["name"] as string) || id,
                visible: (group["visible"] as boolean) ?? true,
              });
            }
          }
        }
      } catch {
        // OCG not supported or no layers — fine
      }
    } catch (err) {
      console.error("Failed to load PDF for print:", err);
      ToastContainer.show("error", "PDF konnte nicht geladen werden");
      return;
    }

    this.overlay = this.buildUI();
    document.body.appendChild(this.overlay);

    // Detect large-format pages now that overlay is in the DOM
    this.detectLargeFormat();

    this.keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") PrintPreviewDialog.dismiss();
    };
    document.addEventListener("keydown", this.keyHandler);

    await this.renderPreview(1);
    this.updateSummary();
  }

  private buildUI(): HTMLElement {
    const overlay = document.createElement("div");
    overlay.className = "print-preview-overlay";

    const dialog = document.createElement("div");
    dialog.className = "print-preview-dialog";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Druckvorschau");

    // Header
    const header = document.createElement("div");
    header.className = "pp-header";

    const title = document.createElement("span");
    title.className = "pp-title";
    title.textContent = `Druckvorschau: ${this.fileName}`;
    header.appendChild(title);

    const closeBtn = document.createElement("button");
    closeBtn.className = "dv-close-btn";
    closeBtn.textContent = "\u00D7";
    closeBtn.addEventListener("click", () => PrintPreviewDialog.dismiss());
    header.appendChild(closeBtn);

    dialog.appendChild(header);

    // Scale warning banner (hidden by default)
    const warning = document.createElement("div");
    warning.className = "pp-scale-warning";
    warning.dataset.id = "pp-warning";
    warning.style.display = "none";
    warning.textContent =
      "\u26A0 WARNUNG: Skalierung ist aktiv! Schnittmuster werden NICHT in Originalgroesse gedruckt.";
    dialog.appendChild(warning);

    // Main content
    const content = document.createElement("div");
    content.className = "pp-content";

    // Left sidebar: page thumbnails with checkboxes
    const sidebar = document.createElement("div");
    sidebar.className = "pp-sidebar";

    const selectActions = document.createElement("div");
    selectActions.className = "pp-select-actions";

    const selectAllBtn = document.createElement("button");
    selectAllBtn.className = "dv-btn";
    selectAllBtn.textContent = "Alle";
    selectAllBtn.addEventListener("click", () => {
      for (let i = 1; i <= this.totalPages; i++) this.selectedPages.add(i);
      this.renderThumbnails(sidebar);
      this.updateSummary();
    });
    selectActions.appendChild(selectAllBtn);

    const selectNoneBtn = document.createElement("button");
    selectNoneBtn.className = "dv-btn";
    selectNoneBtn.textContent = "Keine";
    selectNoneBtn.addEventListener("click", () => {
      this.selectedPages.clear();
      this.renderThumbnails(sidebar);
      this.updateSummary();
    });
    selectActions.appendChild(selectNoneBtn);

    const rangeInput = document.createElement("input");
    rangeInput.className = "pp-range-input";
    rangeInput.placeholder = "z.B. 1-3, 5";
    rangeInput.addEventListener("change", () => {
      const pages = this.parsePageRange(rangeInput.value);
      this.selectedPages = new Set(pages);
      this.renderThumbnails(sidebar);
      this.updateSummary();
    });
    selectActions.appendChild(rangeInput);

    sidebar.appendChild(selectActions);

    // Thumbnails container
    const thumbContainer = document.createElement("div");
    thumbContainer.className = "pp-thumb-container";
    thumbContainer.dataset.id = "pp-thumbs";
    sidebar.appendChild(thumbContainer);

    content.appendChild(sidebar);

    // Center: preview
    this.previewContainer = document.createElement("div");
    this.previewContainer.className = "pp-preview";

    this.previewCanvas = document.createElement("canvas");
    this.previewCanvas.className = "pp-preview-canvas";
    this.previewContainer.appendChild(this.previewCanvas);

    // Calibration overlay
    const calibration = document.createElement("div");
    calibration.className = "pp-calibration";
    calibration.title = "Kalibrierungsquadrat: 25.4 mm (1 Zoll)";
    this.previewContainer.appendChild(calibration);

    content.appendChild(this.previewContainer);

    // Right: settings
    const settingsPanel = document.createElement("div");
    settingsPanel.className = "pp-settings";
    this.buildSettings(settingsPanel);
    content.appendChild(settingsPanel);

    dialog.appendChild(content);

    // Footer
    const footer = document.createElement("div");
    footer.className = "pp-footer";

    const summary = document.createElement("span");
    summary.className = "pp-summary";
    summary.dataset.id = "pp-summary";
    footer.appendChild(summary);

    const printBtn = document.createElement("button");
    printBtn.className = "pp-print-btn";
    printBtn.textContent = "Drucken";
    printBtn.addEventListener("click", () => this.executePrint());
    footer.appendChild(printBtn);

    dialog.appendChild(footer);
    overlay.appendChild(dialog);

    // Render thumbnails asynchronously
    this.renderThumbnails(sidebar);

    return overlay;
  }

  private buildSettings(container: HTMLElement): void {
    const h = document.createElement("h4");
    h.className = "pp-settings-title";
    h.textContent = "Druckeinstellungen";
    container.appendChild(h);

    // Printer
    this.addSettingDropdown(container, "Drucker", this.printers.map(p => ({
      value: p.name,
      label: p.displayName + (p.isDefault ? " (Standard)" : ""),
    })), this.settings.printerName || this.printers.find(p => p.isDefault)?.name || "", (v) => {
      this.settings.printerName = v || null;
    });

    // Paper size
    this.addSettingDropdown(container, "Papiergroesse", [
      { value: "A4", label: "A4 (210\u00D7297 mm)" },
      { value: "Letter", label: "US Letter (216\u00D7279 mm)" },
      { value: "A3", label: "A3 (297\u00D7420 mm)" },
    ], this.settings.paperSize, (v) => {
      this.settings.paperSize = v;
      this.checkScaleWarning();
    });

    // Orientation
    this.addSettingDropdown(container, "Ausrichtung", [
      { value: "auto", label: "Automatisch" },
      { value: "portrait", label: "Hochformat" },
      { value: "landscape", label: "Querformat" },
    ], this.settings.orientation, (v) => {
      this.settings.orientation = v;
    });

    // Copies
    const copiesGroup = document.createElement("div");
    copiesGroup.className = "pp-setting-group";
    const copiesLabel = document.createElement("label");
    copiesLabel.className = "pp-setting-label";
    copiesLabel.textContent = "Exemplare";
    const copiesInput = document.createElement("input");
    copiesInput.type = "number";
    copiesInput.className = "pp-setting-input";
    copiesInput.min = "1";
    copiesInput.max = "99";
    copiesInput.value = String(this.settings.copies);
    copiesInput.addEventListener("change", () => {
      this.settings.copies = Math.max(1, Math.min(99, parseInt(copiesInput.value, 10) || 1));
    });
    copiesGroup.appendChild(copiesLabel);
    copiesGroup.appendChild(copiesInput);
    container.appendChild(copiesGroup);

    // Scale warning checkbox
    const scaleGroup = document.createElement("div");
    scaleGroup.className = "pp-setting-group";
    const fitCheck = document.createElement("input");
    fitCheck.type = "checkbox";
    fitCheck.id = "pp-fit-to-page";
    fitCheck.checked = this.settings.fitToPage;
    fitCheck.addEventListener("change", () => {
      this.settings.fitToPage = fitCheck.checked;
      this.checkScaleWarning();
    });
    const fitLabel = document.createElement("label");
    fitLabel.htmlFor = "pp-fit-to-page";
    fitLabel.textContent = " An Seite anpassen";
    fitLabel.className = "pp-setting-label-inline";
    scaleGroup.appendChild(fitCheck);
    scaleGroup.appendChild(fitLabel);
    container.appendChild(scaleGroup);

    // Tiling section
    const tileGroup = document.createElement("div");
    tileGroup.className = "pp-setting-group";

    const tileCheck = document.createElement("input");
    tileCheck.type = "checkbox";
    tileCheck.id = "pp-tile-enabled";
    tileCheck.checked = this.settings.tileEnabled;
    tileCheck.addEventListener("change", () => {
      this.settings.tileEnabled = tileCheck.checked;
      tileOverlapGroup.style.display = tileCheck.checked ? "" : "none";
      this.updateTileInfo();
    });
    const tileLabel = document.createElement("label");
    tileLabel.htmlFor = "pp-tile-enabled";
    tileLabel.textContent = " Kachelung (Grossformat)";
    tileLabel.className = "pp-setting-label-inline";
    tileGroup.appendChild(tileCheck);
    tileGroup.appendChild(tileLabel);
    container.appendChild(tileGroup);

    const tileOverlapGroup = document.createElement("div");
    tileOverlapGroup.className = "pp-setting-group";
    tileOverlapGroup.style.display = "none";
    const overlapLabel = document.createElement("label");
    overlapLabel.className = "pp-setting-label";
    overlapLabel.textContent = "Ueberlappung (mm)";
    const overlapInput = document.createElement("input");
    overlapInput.type = "number";
    overlapInput.className = "pp-setting-input";
    overlapInput.min = "5";
    overlapInput.max = "30";
    overlapInput.value = String(this.settings.tileOverlapMm);
    overlapInput.addEventListener("change", () => {
      this.settings.tileOverlapMm = Math.max(5, Math.min(30, parseFloat(overlapInput.value) || 15));
      this.updateTileInfo();
    });
    tileOverlapGroup.appendChild(overlapLabel);
    tileOverlapGroup.appendChild(overlapInput);
    container.appendChild(tileOverlapGroup);

    const tileInfoEl = document.createElement("div");
    tileInfoEl.className = "pp-tile-info";
    tileInfoEl.dataset.id = "pp-tile-info";
    container.appendChild(tileInfoEl);

    // Large format detection is called after overlay is assigned (in init)

    // Layer section (OCG)
    if (this.layers.length > 0) {
      const layerHeader = document.createElement("h4");
      layerHeader.className = "pp-settings-title";
      layerHeader.textContent = "Ebenen";
      container.appendChild(layerHeader);

      for (const layer of this.layers) {
        const layerGroup = document.createElement("div");
        layerGroup.className = "pp-setting-group";

        const layerCheck = document.createElement("input");
        layerCheck.type = "checkbox";
        layerCheck.id = `pp-layer-${layer.id}`;
        layerCheck.checked = layer.visible;
        layerCheck.addEventListener("change", () => {
          layer.visible = layerCheck.checked;
          this.renderPreview(1); // Re-render with updated layer visibility
        });

        const layerLabel = document.createElement("label");
        layerLabel.htmlFor = `pp-layer-${layer.id}`;
        layerLabel.textContent = ` ${layer.name}`;
        layerLabel.className = "pp-setting-label-inline";

        layerGroup.appendChild(layerCheck);
        layerGroup.appendChild(layerLabel);
        container.appendChild(layerGroup);
      }
    }
  }

  private addSettingDropdown(
    container: HTMLElement,
    label: string,
    options: { value: string; label: string }[],
    current: string,
    onChange: (v: string) => void
  ): void {
    const group = document.createElement("div");
    group.className = "pp-setting-group";

    const lbl = document.createElement("label");
    lbl.className = "pp-setting-label";
    lbl.textContent = label;
    group.appendChild(lbl);

    const select = document.createElement("select");
    select.className = "pp-setting-select";
    for (const opt of options) {
      const o = document.createElement("option");
      o.value = opt.value;
      o.textContent = opt.label;
      if (opt.value === current) o.selected = true;
      select.appendChild(o);
    }
    select.addEventListener("change", () => onChange(select.value));
    group.appendChild(select);

    container.appendChild(group);
  }

  private async renderThumbnails(sidebar: HTMLElement): Promise<void> {
    const container = sidebar.querySelector<HTMLElement>('[data-id="pp-thumbs"]');
    if (!container || !this.pdfDoc) return;
    container.innerHTML = "";

    for (let i = 1; i <= this.totalPages; i++) {
      const pageNum = i;
      const thumb = document.createElement("div");
      thumb.className = "pp-thumb";
      if (this.selectedPages.has(pageNum)) thumb.classList.add("selected");

      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.className = "pp-thumb-check";
      checkbox.checked = this.selectedPages.has(pageNum);
      checkbox.addEventListener("change", () => {
        if (checkbox.checked) {
          this.selectedPages.add(pageNum);
        } else {
          this.selectedPages.delete(pageNum);
        }
        thumb.classList.toggle("selected", checkbox.checked);
        this.updateSummary();
      });
      thumb.appendChild(checkbox);

      // Render small thumbnail
      const canvas = document.createElement("canvas");
      canvas.className = "pp-thumb-canvas";
      thumb.appendChild(canvas);

      const label = document.createElement("span");
      label.className = "pp-thumb-label";
      label.textContent = String(pageNum);
      thumb.appendChild(label);

      thumb.addEventListener("click", (e) => {
        if ((e.target as HTMLElement).tagName === "INPUT") return;
        this.renderPreview(pageNum);
      });

      container.appendChild(thumb);

      // Render thumbnail async (don't block UI)
      this.pdfDoc.getPage(pageNum).then(async (page) => {
        const viewport = page.getViewport({ scale: 0.15 });
        canvas.width = viewport.width;
        canvas.height = viewport.height;
        const ctx = canvas.getContext("2d");
        if (ctx) {
          await page.render({ canvasContext: ctx, viewport, canvas }).promise;
        }
      }).catch((e) => console.warn(`Failed to render thumbnail for page ${pageNum}:`, e));
    }
  }

  private async renderPreview(pageNum: number): Promise<void> {
    if (!this.pdfDoc || !this.previewCanvas || !this.previewContainer) return;

    const page = await this.pdfDoc.getPage(pageNum);
    const containerW = this.previewContainer.clientWidth - 48;
    const containerH = this.previewContainer.clientHeight - 48;
    const baseViewport = page.getViewport({ scale: 1.0 });
    const scale = Math.min(
      containerW / baseViewport.width,
      containerH / baseViewport.height
    );
    const viewport = page.getViewport({ scale });

    this.previewCanvas.width = viewport.width;
    this.previewCanvas.height = viewport.height;

    const ctx = this.previewCanvas.getContext("2d");
    if (!ctx) return;

    // Apply layer visibility if OCG layers exist
    const renderParams: Record<string, unknown> = {
      canvasContext: ctx,
      viewport,
      canvas: this.previewCanvas,
    };

    if (this.layers.length > 0 && this.pdfDoc) {
      try {
        const occ = await this.pdfDoc.getOptionalContentConfig();
        if (occ) {
          for (const layer of this.layers) {
            occ.setVisibility(layer.id, layer.visible);
          }
          renderParams["optionalContentConfigPromise"] = Promise.resolve(occ);
        }
      } catch {
        // OCG not available
      }
    }

    await page.render(renderParams as Parameters<typeof page.render>[0]).promise;
  }

  private parsePageRange(input: string): number[] {
    const pages = new Set<number>();
    const parts = input.split(",").map((s) => s.trim()).filter(Boolean);
    for (const part of parts) {
      const rangeParts = part.split("-").map((s) => parseInt(s.trim(), 10));
      if (rangeParts.length === 1 && !isNaN(rangeParts[0])) {
        const p = rangeParts[0];
        if (p >= 1 && p <= this.totalPages) pages.add(p);
      } else if (rangeParts.length === 2 && !isNaN(rangeParts[0]) && !isNaN(rangeParts[1])) {
        const start = Math.max(1, rangeParts[0]);
        const end = Math.min(this.totalPages, rangeParts[1]);
        for (let i = start; i <= end; i++) pages.add(i);
      }
    }
    return [...pages].sort((a, b) => a - b);
  }

  private checkScaleWarning(): void {
    const warning = this.overlay?.querySelector<HTMLElement>('[data-id="pp-warning"]');
    if (!warning) return;
    const showWarning = this.settings.fitToPage || this.settings.scale !== 1.0;
    warning.style.display = showWarning ? "" : "none";
  }

  private updateSummary(): void {
    const el = this.overlay?.querySelector<HTMLElement>('[data-id="pp-summary"]');
    if (!el) return;
    const count = this.selectedPages.size;
    const printer = this.settings.printerName
      || this.printers.find(p => p.isDefault)?.displayName
      || "Standard";
    el.textContent = `${count} von ${this.totalPages} Seiten | ${this.settings.paperSize} | ${printer}`;
  }

  private async executePrint(): Promise<void> {
    if (this.selectedPages.size === 0) {
      ToastContainer.show("info", "Keine Seiten ausgewaehlt");
      return;
    }

    // Build page ranges string for lpr
    const sortedPages = [...this.selectedPages].sort((a, b) => a - b);
    let pageRanges: string | null = null;

    if (sortedPages.length < this.totalPages) {
      // Convert to range notation: [1,2,3,5,8,9,10] -> "1-3,5,8-10"
      const ranges: string[] = [];
      let start = sortedPages[0];
      let end = start;
      for (let i = 1; i < sortedPages.length; i++) {
        if (sortedPages[i] === end + 1) {
          end = sortedPages[i];
        } else {
          ranges.push(start === end ? String(start) : `${start}-${end}`);
          start = sortedPages[i];
          end = start;
        }
      }
      ranges.push(start === end ? String(start) : `${start}-${end}`);
      pageRanges = ranges.join(",");
    }

    const printSettings: PrintSettings = {
      ...this.settings,
      pageRanges,
    };

    try {
      await PrintService.printPdf(this.filePath, printSettings);
      // Save settings for next time
      PrintService.savePrintSettings(
        this.settings.paperSize,
        this.settings.orientation,
        this.settings.printerName
      ).catch(() => {});
      // Track last printed
      PrintService.markAsPrinted(this.fileId).catch(() => {});
      ToastContainer.show("success", `Druckauftrag gesendet (${sortedPages.length} Seiten)`);
      PrintPreviewDialog.dismiss();
    } catch (err) {
      console.error("Print failed:", err);
      ToastContainer.show("error", "Drucken fehlgeschlagen");
    }
  }

  private detectLargeFormat(): void {
    const paperMm: Record<string, { w: number; h: number }> = {
      A4: { w: 210, h: 297 },
      Letter: { w: 216, h: 279 },
      A3: { w: 297, h: 420 },
    };
    const target = paperMm[this.settings.paperSize] || paperMm.A4;

    let hasLarge = false;
    for (const [, dim] of this.pageDimensions) {
      if (dim.widthMm > target.w + 5 || dim.heightMm > target.h + 5) {
        hasLarge = true;
        break;
      }
    }

    if (hasLarge) {
      const tileInfo = this.overlay?.querySelector<HTMLElement>('[data-id="pp-tile-info"]');
      if (tileInfo) {
        tileInfo.textContent = "Grossformat erkannt — Kachelung empfohlen";
        tileInfo.style.color = "var(--color-warning-text)";
      }
    }
  }

  private async updateTileInfo(): Promise<void> {
    const infoEl = this.overlay?.querySelector<HTMLElement>('[data-id="pp-tile-info"]');
    if (!infoEl || !this.settings.tileEnabled) {
      if (infoEl) infoEl.textContent = "";
      return;
    }

    // Compute tiles for the first page as representative
    const firstDim = this.pageDimensions.get(1);
    if (!firstDim) return;

    try {
      const tiles = await PrintService.computeTiles(
        firstDim.widthMm,
        firstDim.heightMm,
        this.settings.paperSize,
        this.settings.tileOverlapMm
      );
      infoEl.textContent = `${tiles.totalTiles} Kacheln (${tiles.cols}\u00D7${tiles.rows})`;
      infoEl.style.color = "";
    } catch {
      infoEl.textContent = "Kachelberechnung fehlgeschlagen";
    }
  }

  private close(): void {
    if (this.keyHandler) {
      document.removeEventListener("keydown", this.keyHandler);
      this.keyHandler = null;
    }
    if (this.pdfDoc) {
      this.pdfDoc.destroy();
      this.pdfDoc = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
    this.previewCanvas = null;
    this.previewContainer = null;
  }
}
