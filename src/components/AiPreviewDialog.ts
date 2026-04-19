import * as AiService from "../services/AiService";
import * as FileService from "../services/FileService";
import { trapFocus } from "../utils/focus-trap";
import type { AiAnalysisResult, EmbroideryFile } from "../types/index";

export class AiPreviewDialog {
  private overlay: HTMLElement | null = null;
  private releaseFocusTrap: (() => void) | null = null;
  private fileId: number;
  private file: EmbroideryFile;
  private onResult: (result: AiAnalysisResult) => void;

  constructor(
    fileId: number,
    file: EmbroideryFile,
    onResult: (result: AiAnalysisResult) => void
  ) {
    this.fileId = fileId;
    this.file = file;
    this.onResult = onResult;
  }

  static async open(
    fileId: number,
    file: EmbroideryFile,
    onResult: (result: AiAnalysisResult) => void
  ): Promise<void> {
    const dialog = new AiPreviewDialog(fileId, file, onResult);
    await dialog.show();
  }

  private async show(): Promise<void> {
    // Load prompt and thumbnail in parallel
    const [prompt, thumbnailSrc] = await Promise.all([
      AiService.buildPrompt(this.fileId),
      FileService.getThumbnail(this.fileId),
    ]);

    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });
    this.overlay.addEventListener("dialog-dismiss", () => this.close());

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-ai-preview";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "KI-Analyse Vorschau");

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";
    header.innerHTML =
      '<span class="dialog-title">KI-Analyse Vorschau</span>';
    const closeBtn = document.createElement("button");
    closeBtn.className = "dialog-close";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Schließen");
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    // Body: split view
    const body = document.createElement("div");
    body.className = "dialog-body dialog-split";

    // Left: editable prompt
    const leftPane = document.createElement("div");
    leftPane.className = "dialog-pane dialog-pane-left";

    const promptLabel = document.createElement("label");
    promptLabel.className = "dialog-label";
    promptLabel.textContent = "Prompt";
    leftPane.appendChild(promptLabel);

    const promptArea = document.createElement("textarea");
    promptArea.className = "dialog-textarea";
    promptArea.value = prompt;
    leftPane.appendChild(promptArea);

    body.appendChild(leftPane);

    // Right: file preview
    const rightPane = document.createElement("div");
    rightPane.className = "dialog-pane dialog-pane-right";

    if (thumbnailSrc) {
      const img = document.createElement("img");
      img.src = thumbnailSrc;
      img.alt = this.file.name || this.file.filename;
      img.className = "dialog-preview-img";
      rightPane.appendChild(img);
    }

    const metaList = document.createElement("div");
    metaList.className = "dialog-meta-list";

    this.addMetaRow(metaList, "Datei", this.file.filename);
    if (this.file.name) this.addMetaRow(metaList, "Name", this.file.name);
    if (this.file.widthMm !== null && this.file.heightMm !== null) {
      this.addMetaRow(
        metaList,
        "Abmessungen",
        `${this.file.widthMm.toFixed(1)} \u00D7 ${this.file.heightMm.toFixed(1)} mm`
      );
    }
    if (this.file.stitchCount !== null) {
      this.addMetaRow(
        metaList,
        "Stiche",
        this.file.stitchCount.toLocaleString("de-DE")
      );
    }
    if (this.file.colorCount !== null) {
      this.addMetaRow(metaList, "Farben", String(this.file.colorCount));
    }

    rightPane.appendChild(metaList);
    body.appendChild(rightPane);
    dialog.appendChild(body);

    // Footer
    const footer = document.createElement("div");
    footer.className = "dialog-footer";

    const cancelBtn = document.createElement("button");
    cancelBtn.className = "dialog-btn dialog-btn-secondary";
    cancelBtn.textContent = "Abbrechen";
    cancelBtn.addEventListener("click", () => this.close());
    footer.appendChild(cancelBtn);

    const sendBtn = document.createElement("button");
    sendBtn.className = "dialog-btn dialog-btn-primary";
    sendBtn.textContent = "Senden";
    sendBtn.addEventListener("click", async () => {
      sendBtn.disabled = true;
      sendBtn.textContent = "Analysiere...";
      cancelBtn.disabled = true;

      // Clear previous error
      const prevError = dialog.querySelector(".dialog-error");
      if (prevError) prevError.remove();

      try {
        const result = await AiService.analyzeFile(
          this.fileId,
          promptArea.value
        );
        this.close();
        this.onResult(result);
      } catch (e) {
        sendBtn.disabled = false;
        sendBtn.textContent = "Senden";
        cancelBtn.disabled = false;

        const errorEl =
          dialog.querySelector(".dialog-error") ||
          document.createElement("div");
        errorEl.className = "dialog-error";
        const msg = e instanceof Error
          ? e.message
          : e && typeof e === "object" && "message" in e
            ? (e as { message: string }).message
            : String(e);
        errorEl.textContent = `Fehler: ${msg}`;
        footer.insertBefore(errorEl, cancelBtn);
      }
    });
    footer.appendChild(sendBtn);
    dialog.appendChild(footer);

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);
    this.releaseFocusTrap = trapFocus(dialog);
  }

  private close(): void {
    if (this.releaseFocusTrap) {
      this.releaseFocusTrap();
      this.releaseFocusTrap = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }

  private addMetaRow(
    container: HTMLElement,
    label: string,
    value: string
  ): void {
    const row = document.createElement("div");
    row.className = "dialog-meta-row";

    const labelEl = document.createElement("span");
    labelEl.className = "dialog-meta-label";
    labelEl.textContent = label;
    row.appendChild(labelEl);

    const valueEl = document.createElement("span");
    valueEl.className = "dialog-meta-value";
    valueEl.textContent = value;
    row.appendChild(valueEl);

    container.appendChild(row);
  }
}
