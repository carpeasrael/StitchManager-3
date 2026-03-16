import * as pdfjs from "pdfjs-dist";
import type {
  PDFDocumentProxy,
  PDFPageProxy,
  RenderTask,
} from "pdfjs-dist/types/src/display/api";
import * as ViewerService from "../services/ViewerService";
import type { InstructionBookmark, InstructionNote } from "../types";

// Configure pdf.js worker
pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url
).href;

export class DocumentViewer {
  private static instance: DocumentViewer | null = null;

  private fileId = 0;
  private filePath = "";
  private fileName = "";
  private pdfDoc: PDFDocumentProxy | null = null;
  private currentPage = 1;
  private totalPages = 0;
  private zoom = 1.0;
  private zoomMode: "fit-width" | "fit-page" | "custom" = "fit-width";
  private overlay: HTMLElement | null = null;
  private canvasContainer: HTMLElement | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private renderTask: RenderTask | null = null;
  private isPanning = false;
  private panStartX = 0;
  private panStartY = 0;
  private scrollStartX = 0;
  private scrollStartY = 0;
  private overviewMode = false;

  // Bookmarks & notes
  private bookmarks: InstructionBookmark[] = [];
  private notes: InstructionNote[] = [];
  private sidebarOpen = false;
  private sidebarTab: "bookmarks" | "notes" = "bookmarks";

  // Event handlers stored for cleanup
  private keyHandler: ((e: KeyboardEvent) => void) | null = null;
  private wheelHandler: ((e: WheelEvent) => void) | null = null;
  private panMouseDown: ((e: MouseEvent) => void) | null = null;
  private panMouseMove: ((e: MouseEvent) => void) | null = null;
  private panMouseUp: (() => void) | null = null;

  static async open(
    filePath: string,
    fileId: number,
    fileName: string
  ): Promise<void> {
    if (DocumentViewer.instance) {
      DocumentViewer.dismiss();
    }
    const viewer = new DocumentViewer();
    DocumentViewer.instance = viewer;
    await viewer.init(filePath, fileId, fileName);
  }

  static dismiss(): void {
    if (DocumentViewer.instance) {
      DocumentViewer.instance.close();
      DocumentViewer.instance = null;
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

    this.overlay = this.buildUI();
    document.body.appendChild(this.overlay);

    // Register keyboard shortcuts
    this.keyHandler = (e: KeyboardEvent) => this.onKeyDown(e);
    document.addEventListener("keydown", this.keyHandler);

    // Show loading indicator
    if (this.canvasContainer) {
      this.canvasContainer.innerHTML = '<div class="dv-loading">PDF wird geladen\u2026</div>';
    }

    try {
      await this.loadPdf(filePath);
      // Restore canvas after loading
      if (this.canvasContainer && this.canvas) {
        this.canvasContainer.innerHTML = "";
        this.canvasContainer.appendChild(this.canvas);
      }
      // Restore last viewed page
      const lastPage = await ViewerService.getLastViewedPage(fileId);
      if (lastPage && lastPage > 0 && lastPage <= this.totalPages) {
        this.currentPage = lastPage;
      }
      // Load bookmarks
      this.bookmarks = await ViewerService.getBookmarks(fileId);
      await this.renderPage(this.currentPage);
      this.updateNavUI();
    } catch (err) {
      console.error("Failed to load PDF:", err);
      this.showError("PDF konnte nicht geladen werden.");
    }
  }

  private async loadPdf(filePath: string): Promise<void> {
    const bytes = await ViewerService.readFileBytes(filePath);
    this.pdfDoc = await pdfjs.getDocument({ data: bytes }).promise;
    this.totalPages = this.pdfDoc.numPages;
  }

  private async renderPage(pageNum: number): Promise<void> {
    if (!this.pdfDoc || !this.canvas || !this.canvasContainer) return;

    // Cancel any in-flight render
    if (this.renderTask) {
      this.renderTask.cancel();
      this.renderTask = null;
    }

    const page: PDFPageProxy = await this.pdfDoc.getPage(pageNum);
    const scale = this.getEffectiveScale(page);
    const viewport = page.getViewport({ scale });

    this.canvas.width = viewport.width;
    this.canvas.height = viewport.height;

    const ctx = this.canvas.getContext("2d");
    if (!ctx) return;

    try {
      this.renderTask = page.render({ canvasContext: ctx, viewport, canvas: this.canvas! });
      await this.renderTask.promise;
    } catch (err: unknown) {
      if (err instanceof Error && err.message?.includes("cancelled")) return;
      throw err;
    }

    // Save last viewed page
    ViewerService.setLastViewedPage(this.fileId, pageNum).catch(() => {});

    // Update properties
    this.updateProperties(page, viewport);
    this.updateBookmarkToggle();
  }

  private getEffectiveScale(page: PDFPageProxy): number {
    if (!this.canvasContainer) return this.zoom;
    const viewport = page.getViewport({ scale: 1.0 });
    const containerW = this.canvasContainer.clientWidth - 32;
    const containerH = this.canvasContainer.clientHeight - 32;

    switch (this.zoomMode) {
      case "fit-width":
        return (containerW / viewport.width) * this.zoom;
      case "fit-page":
        return (
          Math.min(containerW / viewport.width, containerH / viewport.height) *
          this.zoom
        );
      case "custom":
        return this.zoom;
    }
  }

  // --- UI Building ---

  private buildUI(): HTMLElement {
    const overlay = document.createElement("div");
    overlay.className = "document-viewer-overlay";

    const viewer = document.createElement("div");
    viewer.className = "document-viewer";

    // Header
    const header = document.createElement("div");
    header.className = "dv-header";

    const title = document.createElement("span");
    title.className = "dv-title";
    title.textContent = this.fileName;
    header.appendChild(title);

    const props = document.createElement("span");
    props.className = "dv-properties";
    props.dataset.id = "dv-props";
    header.appendChild(props);

    const closeBtn = document.createElement("button");
    closeBtn.className = "dv-close-btn";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Schliessen");
    closeBtn.addEventListener("click", () => DocumentViewer.dismiss());
    header.appendChild(closeBtn);

    viewer.appendChild(header);

    // Toolbar
    const toolbar = document.createElement("div");
    toolbar.className = "dv-toolbar";

    // Navigation group
    const navGroup = document.createElement("div");
    navGroup.className = "dv-toolbar-group";

    const prevBtn = this.createToolbarBtn("\u2039", "Vorherige Seite", () =>
      this.prevPage()
    );
    navGroup.appendChild(prevBtn);

    const pageInput = document.createElement("input");
    pageInput.className = "dv-page-input";
    pageInput.type = "number";
    pageInput.min = "1";
    pageInput.setAttribute("aria-label", "Seitennummer");
    pageInput.addEventListener("change", () => {
      this.goToPage(parseInt(pageInput.value, 10) || 1);
    });
    pageInput.addEventListener("keydown", (e) => {
      if (e.key === "Enter") this.goToPage(parseInt(pageInput.value, 10) || 1);
    });
    navGroup.appendChild(pageInput);

    const pageTotal = document.createElement("span");
    pageTotal.className = "dv-page-total";
    pageTotal.dataset.id = "dv-page-total";
    navGroup.appendChild(pageTotal);

    const nextBtn = this.createToolbarBtn("\u203A", "Naechste Seite", () =>
      this.nextPage()
    );
    navGroup.appendChild(nextBtn);

    const overviewBtn = this.createToolbarBtn(
      "\u25A6",
      "Seitenuebersicht",
      () => this.toggleOverview()
    );
    overviewBtn.dataset.id = "dv-overview-btn";
    navGroup.appendChild(overviewBtn);

    toolbar.appendChild(navGroup);

    // Zoom group
    const zoomGroup = document.createElement("div");
    zoomGroup.className = "dv-toolbar-group";

    zoomGroup.appendChild(
      this.createToolbarBtn("\u2212", "Verkleinern", () => this.zoomOut())
    );

    const zoomLabel = document.createElement("span");
    zoomLabel.className = "dv-zoom-label";
    zoomLabel.dataset.id = "dv-zoom-label";
    zoomLabel.textContent = "100%";
    zoomGroup.appendChild(zoomLabel);

    zoomGroup.appendChild(
      this.createToolbarBtn("+", "Vergroessern", () => this.zoomIn())
    );
    zoomGroup.appendChild(
      this.createToolbarBtn(
        "\u2194",
        "Breite anpassen",
        () => {
          this.zoomMode = "fit-width";
          this.zoom = 1.0;
          this.renderPage(this.currentPage);
        }
      )
    );
    zoomGroup.appendChild(
      this.createToolbarBtn(
        "\u2B1C",
        "Seite einpassen",
        () => {
          this.zoomMode = "fit-page";
          this.zoom = 1.0;
          this.renderPage(this.currentPage);
        }
      )
    );

    toolbar.appendChild(zoomGroup);

    // Sidebar toggle group
    const sideGroup = document.createElement("div");
    sideGroup.className = "dv-toolbar-group";

    const bmToggle = this.createToolbarBtn(
      "\u2606",
      "Lesezeichen",
      () => this.toggleBookmarkForPage()
    );
    bmToggle.dataset.id = "dv-bookmark-toggle";
    sideGroup.appendChild(bmToggle);

    sideGroup.appendChild(
      this.createToolbarBtn(
        "\u2630",
        "Seitenleiste",
        () => this.toggleSidebar()
      )
    );

    sideGroup.appendChild(
      this.createToolbarBtn(
        "\u2399",
        "Drucken",
        () => this.openPrintPreview()
      )
    );

    toolbar.appendChild(sideGroup);
    viewer.appendChild(toolbar);

    // Main content area
    const contentWrapper = document.createElement("div");
    contentWrapper.className = "dv-content-wrapper";

    // Canvas container
    this.canvasContainer = document.createElement("div");
    this.canvasContainer.className = "dv-canvas-container";

    this.canvas = document.createElement("canvas");
    this.canvas.className = "dv-canvas";
    this.canvasContainer.appendChild(this.canvas);

    // Wheel zoom
    this.wheelHandler = (e: WheelEvent) => this.onWheel(e);
    this.canvasContainer.addEventListener("wheel", this.wheelHandler, {
      passive: false,
    });

    // Pan — store handlers for cleanup
    this.panMouseDown = (e: MouseEvent) => this.onMouseDown(e);
    this.panMouseMove = (e: MouseEvent) => this.onMouseMove(e);
    this.panMouseUp = () => this.onMouseUp();
    this.canvasContainer.addEventListener("mousedown", this.panMouseDown);
    this.canvasContainer.addEventListener("mousemove", this.panMouseMove);
    this.canvasContainer.addEventListener("mouseup", this.panMouseUp);
    this.canvasContainer.addEventListener("mouseleave", this.panMouseUp);

    contentWrapper.appendChild(this.canvasContainer);

    // Sidebar
    const sidebar = document.createElement("div");
    sidebar.className = "dv-sidebar";
    sidebar.dataset.id = "dv-sidebar";
    sidebar.style.display = "none";

    const sidebarTabs = document.createElement("div");
    sidebarTabs.className = "dv-sidebar-tabs";

    const bmTab = document.createElement("button");
    bmTab.className = "dv-sidebar-tab active";
    bmTab.textContent = "Lesezeichen";
    bmTab.addEventListener("click", () => {
      this.sidebarTab = "bookmarks";
      this.renderSidebar();
    });
    sidebarTabs.appendChild(bmTab);

    const notesTab = document.createElement("button");
    notesTab.className = "dv-sidebar-tab";
    notesTab.textContent = "Notizen";
    notesTab.addEventListener("click", () => {
      this.sidebarTab = "notes";
      this.renderSidebar();
    });
    sidebarTabs.appendChild(notesTab);

    sidebar.appendChild(sidebarTabs);

    const sidebarContent = document.createElement("div");
    sidebarContent.className = "dv-sidebar-content";
    sidebarContent.dataset.id = "dv-sidebar-content";
    sidebar.appendChild(sidebarContent);

    contentWrapper.appendChild(sidebar);
    viewer.appendChild(contentWrapper);

    overlay.appendChild(viewer);
    return overlay;
  }

  private createToolbarBtn(
    text: string,
    label: string,
    onClick: () => void
  ): HTMLButtonElement {
    const btn = document.createElement("button");
    btn.className = "dv-btn";
    btn.textContent = text;
    btn.setAttribute("aria-label", label);
    btn.addEventListener("click", onClick);
    return btn;
  }

  // --- Navigation ---

  private goToPage(pageNum: number): void {
    if (pageNum < 1 || pageNum > this.totalPages) return;
    this.currentPage = pageNum;
    if (this.overviewMode) {
      this.updateOverviewActive();
    } else {
      this.renderPage(pageNum);
    }
    this.updateNavUI();
  }

  private nextPage(): void {
    this.goToPage(this.currentPage + 1);
  }
  private prevPage(): void {
    this.goToPage(this.currentPage - 1);
  }

  private updateNavUI(): void {
    if (!this.overlay) return;
    const input = this.overlay.querySelector<HTMLInputElement>(".dv-page-input");
    const total = this.overlay.querySelector<HTMLElement>(
      '[data-id="dv-page-total"]'
    );
    if (input) input.value = String(this.currentPage);
    if (total) total.textContent = `/ ${this.totalPages}`;

    const zoomLabel = this.overlay.querySelector<HTMLElement>(
      '[data-id="dv-zoom-label"]'
    );
    if (zoomLabel && this.pdfDoc) {
      this.pdfDoc.getPage(this.currentPage).then((page) => {
        const scale = this.getEffectiveScale(page);
        zoomLabel.textContent = `${Math.round(scale * 100)}%`;
      }).catch(() => {});
    }
  }

  // --- Overview mode ---

  private async toggleOverview(): Promise<void> {
    this.overviewMode = !this.overviewMode;
    if (!this.canvasContainer || !this.pdfDoc) return;

    const btn = this.overlay?.querySelector<HTMLElement>(
      '[data-id="dv-overview-btn"]'
    );
    if (btn) btn.classList.toggle("active", this.overviewMode);

    if (this.overviewMode) {
      if (this.canvas) this.canvas.style.display = "none";
      await this.renderOverview();
    } else {
      // Remove overview grid
      const grid = this.canvasContainer.querySelector(".dv-overview-grid");
      if (grid) grid.remove();
      if (this.canvas) this.canvas.style.display = "";
      this.renderPage(this.currentPage);
    }
  }

  private async renderOverview(): Promise<void> {
    if (!this.pdfDoc || !this.canvasContainer) return;

    let grid = this.canvasContainer.querySelector(".dv-overview-grid");
    if (grid) grid.remove();

    grid = document.createElement("div");
    grid.className = "dv-overview-grid";
    this.canvasContainer.appendChild(grid);

    // Render thumbnails in batches to avoid blocking the UI
    const BATCH_SIZE = 6;
    for (let start = 1; start <= this.totalPages; start += BATCH_SIZE) {
      const end = Math.min(start + BATCH_SIZE, this.totalPages + 1);
      const batch: Promise<void>[] = [];

      for (let i = start; i < end; i++) {
        const pageNum = i;
        batch.push(
          (async () => {
            if (!this.pdfDoc || !this.overviewMode) return;
            const page = await this.pdfDoc.getPage(pageNum);
            const viewport = page.getViewport({ scale: 0.3 });

            const thumb = document.createElement("div");
            thumb.className = "dv-overview-thumb";
            if (pageNum === this.currentPage) thumb.classList.add("active");
            thumb.dataset.page = String(pageNum);

            const thumbCanvas = document.createElement("canvas");
            thumbCanvas.width = viewport.width;
            thumbCanvas.height = viewport.height;
            const ctx = thumbCanvas.getContext("2d");
            if (ctx) {
              await page.render({ canvasContext: ctx, viewport, canvas: thumbCanvas }).promise;
            }

            const label = document.createElement("div");
            label.className = "dv-overview-label";
            label.textContent = this.bookmarks.some((b) => b.pageNumber === pageNum)
              ? `\u2605 ${pageNum}`
              : String(pageNum);

            thumb.appendChild(thumbCanvas);
            thumb.appendChild(label);
            thumb.addEventListener("click", () => {
              this.currentPage = pageNum;
              this.toggleOverview();
            });

            grid!.appendChild(thumb);
          })()
        );
      }

      await Promise.all(batch);
      // Yield to the event loop between batches
      await new Promise((r) => requestAnimationFrame(r));
    }
  }

  private updateOverviewActive(): void {
    if (!this.canvasContainer) return;
    const thumbs =
      this.canvasContainer.querySelectorAll<HTMLElement>(".dv-overview-thumb");
    thumbs.forEach((t) => {
      t.classList.toggle(
        "active",
        t.dataset.page === String(this.currentPage)
      );
    });
  }

  // --- Zoom & Pan ---

  private zoomIn(): void {
    this.zoom = Math.min(5.0, this.zoom * 1.25);
    if (this.zoomMode !== "custom") this.zoomMode = "custom";
    this.renderPage(this.currentPage);
    this.updateNavUI();
  }

  private zoomOut(): void {
    this.zoom = Math.max(0.25, this.zoom / 1.25);
    if (this.zoomMode !== "custom") this.zoomMode = "custom";
    this.renderPage(this.currentPage);
    this.updateNavUI();
  }

  private onWheel(e: WheelEvent): void {
    if (!e.ctrlKey) return;
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    this.zoom = Math.min(5.0, Math.max(0.25, this.zoom * factor));
    this.zoomMode = "custom";
    this.renderPage(this.currentPage);
    this.updateNavUI();
  }

  private onMouseDown(e: MouseEvent): void {
    if (!this.canvasContainer) return;
    this.isPanning = true;
    this.panStartX = e.clientX;
    this.panStartY = e.clientY;
    this.scrollStartX = this.canvasContainer.scrollLeft;
    this.scrollStartY = this.canvasContainer.scrollTop;
    this.canvasContainer.style.cursor = "grabbing";
  }

  private onMouseMove(e: MouseEvent): void {
    if (!this.isPanning || !this.canvasContainer) return;
    this.canvasContainer.scrollLeft =
      this.scrollStartX - (e.clientX - this.panStartX);
    this.canvasContainer.scrollTop =
      this.scrollStartY - (e.clientY - this.panStartY);
  }

  private onMouseUp(): void {
    this.isPanning = false;
    if (this.canvasContainer) this.canvasContainer.style.cursor = "grab";
  }

  // --- Properties ---

  private updateProperties(page: PDFPageProxy, viewport: { width: number; height: number }): void {
    const propsEl = this.overlay?.querySelector<HTMLElement>(
      '[data-id="dv-props"]'
    );
    if (!propsEl) return;

    const widthMm = (viewport.width / (this.getEffectiveScale(page)) * 0.3528).toFixed(0);
    const heightMm = (viewport.height / (this.getEffectiveScale(page)) * 0.3528).toFixed(0);
    const paperSize = this.classifyPaperSize(
      parseFloat(widthMm),
      parseFloat(heightMm)
    );

    propsEl.textContent = `${this.totalPages} Seiten \u00B7 ${paperSize} \u00B7 ${widthMm}\u00D7${heightMm} mm`;
  }

  private classifyPaperSize(wMm: number, hMm: number): string {
    const w = Math.min(wMm, hMm);
    const h = Math.max(wMm, hMm);
    if (Math.abs(w - 210) < 5 && Math.abs(h - 297) < 5) return "A4";
    if (Math.abs(w - 216) < 5 && Math.abs(h - 279) < 5) return "US Letter";
    if (Math.abs(w - 297) < 5 && Math.abs(h - 420) < 5) return "A3";
    if (Math.abs(w - 420) < 5 && Math.abs(h - 594) < 5) return "A2";
    if (Math.abs(w - 594) < 5 && Math.abs(h - 841) < 5) return "A1";
    if (Math.abs(w - 841) < 5 && Math.abs(h - 1189) < 5) return "A0";
    return `${Math.round(w)}\u00D7${Math.round(h)} mm`;
  }

  // --- Bookmarks ---

  private async toggleBookmarkForPage(): Promise<void> {
    const added = await ViewerService.toggleBookmark(
      this.fileId,
      this.currentPage
    );
    this.bookmarks = await ViewerService.getBookmarks(this.fileId);
    this.updateBookmarkToggle();
    if (this.sidebarOpen && this.sidebarTab === "bookmarks") {
      this.renderSidebar();
    }
    if (added) {
      // Could show a toast, but keeping it simple
    }
  }

  private updateBookmarkToggle(): void {
    const btn = this.overlay?.querySelector<HTMLElement>(
      '[data-id="dv-bookmark-toggle"]'
    );
    if (!btn) return;
    const isBookmarked = this.bookmarks.some(
      (b) => b.pageNumber === this.currentPage
    );
    btn.textContent = isBookmarked ? "\u2605" : "\u2606";
    btn.classList.toggle("active", isBookmarked);
  }

  // --- Sidebar ---

  private toggleSidebar(): void {
    this.sidebarOpen = !this.sidebarOpen;
    const sidebar = this.overlay?.querySelector<HTMLElement>(
      '[data-id="dv-sidebar"]'
    );
    if (sidebar) sidebar.style.display = this.sidebarOpen ? "" : "none";
    if (this.sidebarOpen) this.renderSidebar();
  }

  private async renderSidebar(): Promise<void> {
    const content = this.overlay?.querySelector<HTMLElement>(
      '[data-id="dv-sidebar-content"]'
    );
    if (!content) return;
    content.innerHTML = "";

    // Update tab active state
    const tabs = this.overlay?.querySelectorAll<HTMLElement>(".dv-sidebar-tab");
    if (tabs) {
      tabs[0].classList.toggle("active", this.sidebarTab === "bookmarks");
      tabs[1].classList.toggle("active", this.sidebarTab === "notes");
    }

    if (this.sidebarTab === "bookmarks") {
      this.renderBookmarksList(content);
    } else {
      await this.renderNotesList(content);
    }
  }

  private renderBookmarksList(container: HTMLElement): void {
    if (this.bookmarks.length === 0) {
      const empty = document.createElement("div");
      empty.className = "dv-sidebar-empty";
      empty.textContent = "Keine Lesezeichen";
      container.appendChild(empty);
      return;
    }

    const list = document.createElement("div");
    list.className = "dv-bookmark-list";

    for (const bm of this.bookmarks) {
      const item = document.createElement("div");
      item.className = "dv-bookmark-item";

      const pageSpan = document.createElement("span");
      pageSpan.className = "dv-bookmark-page";
      pageSpan.textContent = `S. ${bm.pageNumber}`;
      item.appendChild(pageSpan);

      const labelInput = document.createElement("input");
      labelInput.className = "dv-bookmark-label";
      labelInput.value = bm.label || "";
      labelInput.placeholder = "Bezeichnung...";
      labelInput.addEventListener("change", () => {
        ViewerService.updateBookmarkLabel(bm.id, labelInput.value).catch(
          (e) => console.warn("Failed to update bookmark label:", e)
        );
        bm.label = labelInput.value;
      });
      item.appendChild(labelInput);

      const removeBtn = document.createElement("button");
      removeBtn.className = "dv-bookmark-remove";
      removeBtn.textContent = "\u00D7";
      removeBtn.addEventListener("click", async (e) => {
        e.stopPropagation();
        await ViewerService.toggleBookmark(this.fileId, bm.pageNumber);
        this.bookmarks = await ViewerService.getBookmarks(this.fileId);
        this.renderSidebar();
        this.updateBookmarkToggle();
      });
      item.appendChild(removeBtn);

      item.addEventListener("click", () => {
        this.goToPage(bm.pageNumber);
        if (this.overviewMode) this.toggleOverview();
      });

      list.appendChild(item);
    }

    container.appendChild(list);
  }

  private async renderNotesList(container: HTMLElement): Promise<void> {
    this.notes = await ViewerService.getNotes(this.fileId, this.currentPage);

    const header = document.createElement("div");
    header.className = "dv-notes-header";
    header.textContent = `Notizen \u2014 Seite ${this.currentPage}`;
    container.appendChild(header);

    for (const note of this.notes) {
      const item = document.createElement("div");
      item.className = "dv-note-item";

      const textarea = document.createElement("textarea");
      textarea.className = "dv-note-text";
      textarea.value = note.noteText;
      textarea.rows = 3;
      item.appendChild(textarea);

      const actions = document.createElement("div");
      actions.className = "dv-note-actions";

      const saveBtn = document.createElement("button");
      saveBtn.className = "dv-btn dv-note-save";
      saveBtn.textContent = "Speichern";
      saveBtn.addEventListener("click", async () => {
        await ViewerService.updateNote(note.id, textarea.value);
      });
      actions.appendChild(saveBtn);

      const delBtn = document.createElement("button");
      delBtn.className = "dv-btn dv-note-delete";
      delBtn.textContent = "Loeschen";
      delBtn.addEventListener("click", async () => {
        await ViewerService.deleteNote(note.id);
        this.renderSidebar();
      });
      actions.appendChild(delBtn);

      item.appendChild(actions);
      container.appendChild(item);
    }

    const addBtn = document.createElement("button");
    addBtn.className = "dv-btn dv-note-add";
    addBtn.textContent = "+ Notiz hinzufuegen";
    addBtn.addEventListener("click", async () => {
      await ViewerService.addNote(
        this.fileId,
        this.currentPage,
        "Neue Notiz"
      );
      this.renderSidebar();
    });
    container.appendChild(addBtn);
  }

  // --- Keyboard ---

  private onKeyDown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      DocumentViewer.dismiss();
      return;
    }

    // Don't intercept when user is typing in an input
    const target = e.target as HTMLElement;
    if (
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.isContentEditable
    )
      return;

    if (e.key === "ArrowLeft" || e.key === "PageUp") {
      e.preventDefault();
      this.prevPage();
    } else if (e.key === "ArrowRight" || e.key === "PageDown") {
      e.preventDefault();
      this.nextPage();
    } else if (e.key === "Home") {
      e.preventDefault();
      this.goToPage(1);
    } else if (e.key === "End") {
      e.preventDefault();
      this.goToPage(this.totalPages);
    } else if (e.ctrlKey && (e.key === "=" || e.key === "+")) {
      e.preventDefault();
      this.zoomIn();
    } else if (e.ctrlKey && e.key === "-") {
      e.preventDefault();
      this.zoomOut();
    } else if (e.ctrlKey && e.key === "0") {
      e.preventDefault();
      this.zoomMode = "fit-width";
      this.zoom = 1.0;
      this.renderPage(this.currentPage);
      this.updateNavUI();
    } else if (e.ctrlKey && e.key === "p") {
      e.preventDefault();
      this.openPrintPreview();
    }
  }

  // --- Print ---

  private openPrintPreview(): void {
    if (!this.filePath) return;
    import("./PrintPreviewDialog").then(({ PrintPreviewDialog }) => {
      PrintPreviewDialog.open(this.filePath, this.fileId, this.fileName);
    }).catch((e) => {
      console.error("Failed to load print preview:", e);
    });
  }

  // --- Error display ---

  private showError(msg: string): void {
    if (!this.canvasContainer) return;
    const err = document.createElement("div");
    err.className = "dv-error";
    err.textContent = msg;
    this.canvasContainer.innerHTML = "";
    this.canvasContainer.appendChild(err);
  }

  // --- Cleanup ---

  private close(): void {
    if (this.renderTask) {
      this.renderTask.cancel();
      this.renderTask = null;
    }
    if (this.keyHandler) {
      document.removeEventListener("keydown", this.keyHandler);
      this.keyHandler = null;
    }
    if (this.wheelHandler && this.canvasContainer) {
      this.canvasContainer.removeEventListener("wheel", this.wheelHandler);
      this.wheelHandler = null;
    }
    if (this.canvasContainer) {
      if (this.panMouseDown) this.canvasContainer.removeEventListener("mousedown", this.panMouseDown);
      if (this.panMouseMove) this.canvasContainer.removeEventListener("mousemove", this.panMouseMove);
      if (this.panMouseUp) {
        this.canvasContainer.removeEventListener("mouseup", this.panMouseUp);
        this.canvasContainer.removeEventListener("mouseleave", this.panMouseUp);
      }
    }
    this.panMouseDown = null;
    this.panMouseMove = null;
    this.panMouseUp = null;
    if (this.pdfDoc) {
      this.pdfDoc.destroy();
      this.pdfDoc = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
    this.canvas = null;
    this.canvasContainer = null;
  }
}
