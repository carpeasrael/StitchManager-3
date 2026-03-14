import { trapFocus } from "../utils/focus-trap";
import type { StitchSegment } from "../types/index";

export class ImagePreviewDialog {
  private static instance: ImagePreviewDialog | null = null;
  private overlay: HTMLElement | null = null;
  private releaseFocusTrap: (() => void) | null = null;
  private cleanupListeners: (() => void) | null = null;

  private segments: StitchSegment[];

  private constructor(segments: StitchSegment[]) {
    this.segments = segments;
  }

  static open(segments: StitchSegment[]): void {
    if (ImagePreviewDialog.instance) {
      ImagePreviewDialog.dismiss();
    }
    const dialog = new ImagePreviewDialog(segments);
    ImagePreviewDialog.instance = dialog;
    dialog.show();
  }

  static dismiss(): void {
    if (ImagePreviewDialog.instance) {
      ImagePreviewDialog.instance.close();
      ImagePreviewDialog.instance = null;
    }
  }

  private show(): void {
    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay image-preview-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });

    const dialog = document.createElement("div");
    dialog.className = "dialog image-preview-dialog";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Bildvorschau");

    // Close button
    const closeBtn = document.createElement("button");
    closeBtn.className = "image-preview-close";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Schliessen");
    closeBtn.addEventListener("click", () => this.close());
    dialog.appendChild(closeBtn);

    // Canvas container
    const canvasContainer = document.createElement("div");
    canvasContainer.className = "image-preview-canvas-container";

    const canvas = document.createElement("canvas");
    canvas.className = "image-preview-canvas";
    canvasContainer.appendChild(canvas);
    dialog.appendChild(canvasContainer);

    // Controls bar
    const controls = document.createElement("div");
    controls.className = "image-preview-controls";

    const zoomOutBtn = document.createElement("button");
    zoomOutBtn.className = "image-preview-btn";
    zoomOutBtn.textContent = "\u2212";
    zoomOutBtn.title = "Verkleinern";
    zoomOutBtn.setAttribute("aria-label", "Verkleinern");
    controls.appendChild(zoomOutBtn);

    const zoomLabel = document.createElement("span");
    zoomLabel.className = "image-preview-zoom-label";
    zoomLabel.textContent = "100%";
    controls.appendChild(zoomLabel);

    const zoomInBtn = document.createElement("button");
    zoomInBtn.className = "image-preview-btn";
    zoomInBtn.textContent = "+";
    zoomInBtn.title = "Vergrössern";
    zoomInBtn.setAttribute("aria-label", "Vergrössern");
    controls.appendChild(zoomInBtn);

    const fitBtn = document.createElement("button");
    fitBtn.className = "image-preview-btn";
    fitBtn.textContent = "\u21BA";
    fitBtn.title = "Einpassen";
    fitBtn.setAttribute("aria-label", "Einpassen");
    controls.appendChild(fitBtn);

    dialog.appendChild(controls);

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);
    this.releaseFocusTrap = trapFocus(dialog);

    // Set up rendering and interaction
    this.setupCanvas(canvas, canvasContainer, zoomLabel, {
      zoomInBtn,
      zoomOutBtn,
      fitBtn,
    });

    // Escape key handler
    const onKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        this.close();
      }
    };
    document.addEventListener("keydown", onKeydown);

    const prevCleanup = this.cleanupListeners;
    this.cleanupListeners = () => {
      document.removeEventListener("keydown", onKeydown);
      if (prevCleanup) prevCleanup();
    };
  }

  private setupCanvas(
    canvas: HTMLCanvasElement,
    container: HTMLElement,
    zoomLabel: HTMLElement,
    btns: {
      zoomInBtn: HTMLButtonElement;
      zoomOutBtn: HTMLButtonElement;
      fitBtn: HTMLButtonElement;
    }
  ): void {
    const segments = this.segments;
    if (segments.length === 0) return;

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

    const padding = 24;
    let zoom = 1;
    let panX = 0;
    let panY = 0;

    const draw = () => {
      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const dpr = window.devicePixelRatio || 1;
      const displayW = container.clientWidth;
      const displayH = container.clientHeight;
      const targetW = Math.round(displayW * dpr);
      const targetH = Math.round(displayH * dpr);
      if (canvas.width !== targetW || canvas.height !== targetH) {
        canvas.width = targetW;
        canvas.height = targetH;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

      const bgColor = getComputedStyle(container).backgroundColor || "#1a1a2e";
      ctx.fillStyle = bgColor;
      ctx.fillRect(0, 0, displayW, displayH);

      ctx.save();
      ctx.translate(panX, panY);
      ctx.scale(zoom, zoom);

      const drawW = displayW - 2 * padding;
      const drawH = displayH - 2 * padding;
      const scale = Math.min(drawW / dataW, drawH / dataH);
      const offX = padding + (drawW - dataW * scale) / 2;
      const offY = padding + (drawH - dataH * scale) / 2;

      ctx.lineWidth = Math.max(1, 1.5 / zoom);
      ctx.lineCap = "round";
      ctx.lineJoin = "round";

      for (const seg of segments) {
        if (seg.points.length < 2) continue;
        ctx.strokeStyle = seg.colorHex || "#ffffff";
        ctx.beginPath();
        const [sx, sy] = seg.points[0];
        ctx.moveTo((sx - minX) * scale + offX, (sy - minY) * scale + offY);
        for (let i = 1; i < seg.points.length; i++) {
          const [px, py] = seg.points[i];
          ctx.lineTo((px - minX) * scale + offX, (py - minY) * scale + offY);
        }
        ctx.stroke();
      }

      ctx.restore();
      zoomLabel.textContent = `${Math.round(zoom * 100)}%`;
    };

    // Initial draw after layout
    requestAnimationFrame(draw);

    // Mouse wheel zoom
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const rect = canvas.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;
      const mouseY = e.clientY - rect.top;
      const oldZoom = zoom;
      const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
      zoom = Math.min(4, Math.max(0.25, zoom * factor));
      panX = mouseX - (mouseX - panX) * (zoom / oldZoom);
      panY = mouseY - (mouseY - panY) * (zoom / oldZoom);
      draw();
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });

    // Pan with drag
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
      draw();
    };
    document.addEventListener("mousemove", onMouseMove);

    const onMouseUp = () => {
      dragging = false;
      canvas.style.cursor = "grab";
    };
    document.addEventListener("mouseup", onMouseUp);
    canvas.style.cursor = "grab";

    // Double-click to reset
    const onDblClick = () => {
      zoom = 1;
      panX = 0;
      panY = 0;
      draw();
    };
    canvas.addEventListener("dblclick", onDblClick);

    // Zoom buttons
    const zoomAt = (factor: number) => {
      const centerX = container.clientWidth / 2;
      const centerY = container.clientHeight / 2;
      const oldZoom = zoom;
      zoom = Math.min(4, Math.max(0.25, zoom * factor));
      panX = centerX - (centerX - panX) * (zoom / oldZoom);
      panY = centerY - (centerY - panY) * (zoom / oldZoom);
      draw();
    };

    const onZoomIn = () => zoomAt(1.3);
    btns.zoomInBtn.addEventListener("click", onZoomIn);

    const onZoomOut = () => zoomAt(1 / 1.3);
    btns.zoomOutBtn.addEventListener("click", onZoomOut);

    const onFit = () => {
      zoom = 1;
      panX = 0;
      panY = 0;
      draw();
    };
    btns.fitBtn.addEventListener("click", onFit);

    // Redraw on container resize
    const resizeObserver = new ResizeObserver(() => draw());
    resizeObserver.observe(container);

    // Store cleanup
    const prevCleanup = this.cleanupListeners;
    this.cleanupListeners = () => {
      resizeObserver.disconnect();
      canvas.removeEventListener("wheel", onWheel);
      canvas.removeEventListener("mousedown", onMouseDown);
      canvas.removeEventListener("dblclick", onDblClick);
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      btns.zoomInBtn.removeEventListener("click", onZoomIn);
      btns.zoomOutBtn.removeEventListener("click", onZoomOut);
      btns.fitBtn.removeEventListener("click", onFit);
      if (prevCleanup) prevCleanup();
    };
  }

  private close(): void {
    if (this.cleanupListeners) {
      this.cleanupListeners();
      this.cleanupListeners = null;
    }
    if (this.releaseFocusTrap) {
      this.releaseFocusTrap();
      this.releaseFocusTrap = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
    ImagePreviewDialog.instance = null;
  }
}
