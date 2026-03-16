import * as ViewerService from "../services/ViewerService";

interface ImageSource {
  filePath: string;
  displayName: string;
}

export class ImageViewerDialog {
  private static instance: ImageViewerDialog | null = null;

  private images: ImageSource[] = [];
  private currentIndex = 0;
  private zoom = 1.0;
  private panX = 0;
  private panY = 0;
  private overlay: HTMLElement | null = null;
  private imgEl: HTMLImageElement | null = null;
  private isPanning = false;
  private lastX = 0;
  private lastY = 0;
  private keyHandler: ((e: KeyboardEvent) => void) | null = null;

  static async open(
    images: ImageSource[],
    startIndex = 0
  ): Promise<void> {
    if (ImageViewerDialog.instance) {
      ImageViewerDialog.dismiss();
    }
    const viewer = new ImageViewerDialog();
    ImageViewerDialog.instance = viewer;
    await viewer.init(images, startIndex);
  }

  static dismiss(): void {
    if (ImageViewerDialog.instance) {
      ImageViewerDialog.instance.close();
      ImageViewerDialog.instance = null;
    }
  }

  private async init(
    images: ImageSource[],
    startIndex: number
  ): Promise<void> {
    this.images = images;
    this.currentIndex = startIndex;

    this.overlay = this.buildUI();
    document.body.appendChild(this.overlay);

    this.keyHandler = (e: KeyboardEvent) => this.onKeyDown(e);
    document.addEventListener("keydown", this.keyHandler);

    await this.loadImage(this.currentIndex);
  }

  private buildUI(): HTMLElement {
    const overlay = document.createElement("div");
    overlay.className = "image-viewer-overlay";

    const dialog = document.createElement("div");
    dialog.className = "image-viewer-dialog";

    // Header
    const header = document.createElement("div");
    header.className = "image-viewer-header";

    const title = document.createElement("span");
    title.className = "image-viewer-title";
    title.dataset.id = "iv-title";
    header.appendChild(title);

    if (this.images.length > 1) {
      const counter = document.createElement("span");
      counter.className = "image-viewer-counter";
      counter.dataset.id = "iv-counter";
      header.appendChild(counter);
    }

    const closeBtn = document.createElement("button");
    closeBtn.className = "image-viewer-close";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Schliessen");
    closeBtn.addEventListener("click", () => ImageViewerDialog.dismiss());
    header.appendChild(closeBtn);

    dialog.appendChild(header);

    // Content
    const content = document.createElement("div");
    content.className = "image-viewer-content";

    if (this.images.length > 1) {
      const prevBtn = document.createElement("button");
      prevBtn.className = "image-viewer-nav image-viewer-prev";
      prevBtn.textContent = "\u2039";
      prevBtn.addEventListener("click", () => this.prev());
      content.appendChild(prevBtn);
    }

    const imgContainer = document.createElement("div");
    imgContainer.className = "image-viewer-img-container";

    this.imgEl = document.createElement("img");
    this.imgEl.className = "image-viewer-img";
    imgContainer.appendChild(this.imgEl);

    // Pan
    imgContainer.addEventListener("mousedown", (e) => {
      this.isPanning = true;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      imgContainer.style.cursor = "grabbing";
    });
    imgContainer.addEventListener("mousemove", (e) => {
      if (!this.isPanning) return;
      this.panX += e.clientX - this.lastX;
      this.panY += e.clientY - this.lastY;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      this.updateTransform();
    });
    imgContainer.addEventListener("mouseup", () => {
      this.isPanning = false;
      imgContainer.style.cursor = "grab";
    });
    imgContainer.addEventListener("mouseleave", () => {
      this.isPanning = false;
      imgContainer.style.cursor = "grab";
    });

    // Wheel zoom (Ctrl+wheel only, plain wheel scrolls)
    imgContainer.addEventListener(
      "wheel",
      (e) => {
        if (!e.ctrlKey) return;
        e.preventDefault();
        const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
        this.zoom = Math.min(10, Math.max(0.1, this.zoom * factor));
        this.updateTransform();
      },
      { passive: false }
    );

    // Double-click reset
    imgContainer.addEventListener("dblclick", () => {
      this.zoom = 1.0;
      this.panX = 0;
      this.panY = 0;
      this.updateTransform();
    });

    content.appendChild(imgContainer);

    if (this.images.length > 1) {
      const nextBtn = document.createElement("button");
      nextBtn.className = "image-viewer-nav image-viewer-next";
      nextBtn.textContent = "\u203A";
      nextBtn.addEventListener("click", () => this.next());
      content.appendChild(nextBtn);
    }

    dialog.appendChild(content);

    // Controls
    const controls = document.createElement("div");
    controls.className = "image-viewer-controls";

    const zoomOutBtn = document.createElement("button");
    zoomOutBtn.className = "dv-btn";
    zoomOutBtn.textContent = "\u2212";
    zoomOutBtn.addEventListener("click", () => {
      this.zoom = Math.max(0.1, this.zoom / 1.25);
      this.updateTransform();
    });
    controls.appendChild(zoomOutBtn);

    const zoomLabel = document.createElement("span");
    zoomLabel.className = "image-viewer-zoom-label";
    zoomLabel.dataset.id = "iv-zoom";
    controls.appendChild(zoomLabel);

    const zoomInBtn = document.createElement("button");
    zoomInBtn.className = "dv-btn";
    zoomInBtn.textContent = "+";
    zoomInBtn.addEventListener("click", () => {
      this.zoom = Math.min(10, this.zoom * 1.25);
      this.updateTransform();
    });
    controls.appendChild(zoomInBtn);

    const fitBtn = document.createElement("button");
    fitBtn.className = "dv-btn";
    fitBtn.textContent = "Einpassen";
    fitBtn.addEventListener("click", () => {
      this.zoom = 1.0;
      this.panX = 0;
      this.panY = 0;
      this.updateTransform();
    });
    controls.appendChild(fitBtn);

    dialog.appendChild(controls);
    overlay.appendChild(dialog);
    return overlay;
  }

  private async loadImage(index: number): Promise<void> {
    if (!this.imgEl || index < 0 || index >= this.images.length) return;

    const source = this.images[index];
    this.zoom = 1.0;
    this.panX = 0;
    this.panY = 0;

    try {
      // Use raw base64 directly from backend — avoids decode+re-encode round-trip
      const base64 = await ViewerService.readFileBase64(source.filePath);
      const ext = source.filePath.split(".").pop()?.toLowerCase() || "png";
      const mimeMap: Record<string, string> = {
        png: "image/png",
        jpg: "image/jpeg",
        jpeg: "image/jpeg",
        gif: "image/gif",
        webp: "image/webp",
        svg: "image/svg+xml",
        bmp: "image/bmp",
      };
      const mime = mimeMap[ext] || "image/png";
      this.imgEl.src = `data:${mime};base64,${base64}`;
    } catch {
      this.imgEl.alt = "Bild konnte nicht geladen werden";
    }

    this.updateUI();
    this.updateTransform();
  }

  private updateUI(): void {
    if (!this.overlay) return;
    const title = this.overlay.querySelector<HTMLElement>(
      '[data-id="iv-title"]'
    );
    if (title) title.textContent = this.images[this.currentIndex]?.displayName || "";

    const counter = this.overlay.querySelector<HTMLElement>(
      '[data-id="iv-counter"]'
    );
    if (counter)
      counter.textContent = `${this.currentIndex + 1} / ${this.images.length}`;
  }

  private updateTransform(): void {
    if (!this.imgEl) return;
    this.imgEl.style.transform = `translate(${this.panX}px, ${this.panY}px) scale(${this.zoom})`;

    const label = this.overlay?.querySelector<HTMLElement>(
      '[data-id="iv-zoom"]'
    );
    if (label) label.textContent = `${Math.round(this.zoom * 100)}%`;
  }

  private next(): void {
    if (this.currentIndex < this.images.length - 1) {
      this.currentIndex++;
      this.loadImage(this.currentIndex);
    }
  }

  private prev(): void {
    if (this.currentIndex > 0) {
      this.currentIndex--;
      this.loadImage(this.currentIndex);
    }
  }

  private onKeyDown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.stopImmediatePropagation();
      ImageViewerDialog.dismiss();
    } else if (e.key === "ArrowLeft") {
      this.prev();
    } else if (e.key === "ArrowRight") {
      this.next();
    }
  }

  private close(): void {
    if (this.keyHandler) {
      document.removeEventListener("keydown", this.keyHandler);
      this.keyHandler = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
    this.imgEl = null;
  }
}
