import { ToastContainer } from "./Toast";
import { trapFocus } from "../utils/focus-trap";
import { save } from "@tauri-apps/plugin-dialog";
import * as FileService from "../services/FileService";
import type { Transform } from "../types/index";

export class EditDialog {
  private static instance: EditDialog | null = null;

  static async open(fileId: number, filename: string): Promise<void> {
    if (EditDialog.instance) return;
    const dialog = new EditDialog();
    EditDialog.instance = dialog;
    await dialog.show(fileId, filename);
  }

  private overlay: HTMLElement | null = null;
  private releaseFocusTrap: (() => void) | null = null;

  private async show(fileId: number, filename: string): Promise<void> {
    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-edit";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Bearbeiten");

    // Title
    const title = document.createElement("h3");
    title.className = "dialog-edit-title";
    title.textContent = `Bearbeiten: ${filename}`;
    dialog.appendChild(title);

    // Transform buttons grid
    const grid = document.createElement("div");
    grid.className = "edit-transform-grid";

    // Rotate buttons
    const rotateLabel = document.createElement("div");
    rotateLabel.className = "edit-section-label";
    rotateLabel.textContent = "Drehen";
    grid.appendChild(rotateLabel);

    const rotateRow = document.createElement("div");
    rotateRow.className = "edit-btn-row";
    for (const deg of [90, 180, 270]) {
      const btn = document.createElement("button");
      btn.className = "edit-btn";
      btn.textContent = `${deg}\u00B0`;
      btn.addEventListener("click", () => this.applyAndSave(fileId, [{ type: "rotate", degrees: deg }]));
      rotateRow.appendChild(btn);
    }
    grid.appendChild(rotateRow);

    // Mirror buttons
    const mirrorLabel = document.createElement("div");
    mirrorLabel.className = "edit-section-label";
    mirrorLabel.textContent = "Spiegeln";
    grid.appendChild(mirrorLabel);

    const mirrorRow = document.createElement("div");
    mirrorRow.className = "edit-btn-row";
    const hBtn = document.createElement("button");
    hBtn.className = "edit-btn";
    hBtn.textContent = "\u2194 Horizontal";
    hBtn.addEventListener("click", () => this.applyAndSave(fileId, [{ type: "mirrorHorizontal" }]));
    mirrorRow.appendChild(hBtn);

    const vBtn = document.createElement("button");
    vBtn.className = "edit-btn";
    vBtn.textContent = "\u2195 Vertikal";
    vBtn.addEventListener("click", () => this.applyAndSave(fileId, [{ type: "mirrorVertical" }]));
    mirrorRow.appendChild(vBtn);
    grid.appendChild(mirrorRow);

    // Resize
    const resizeLabel = document.createElement("div");
    resizeLabel.className = "edit-section-label";
    resizeLabel.textContent = "Skalieren (%)";
    grid.appendChild(resizeLabel);

    const resizeRow = document.createElement("div");
    resizeRow.className = "edit-btn-row";
    for (const pct of [50, 75, 125, 150, 200]) {
      const btn = document.createElement("button");
      btn.className = "edit-btn";
      btn.textContent = `${pct}%`;
      const scale = pct / 100;
      btn.addEventListener("click", () => this.applyAndSave(fileId, [{ type: "resize", scaleX: scale, scaleY: scale }]));
      resizeRow.appendChild(btn);
    }
    grid.appendChild(resizeRow);

    dialog.appendChild(grid);

    // Close button
    const closeBtn = document.createElement("button");
    closeBtn.className = "edit-close-btn";
    closeBtn.textContent = "Schließen";
    closeBtn.addEventListener("click", () => this.close());
    dialog.appendChild(closeBtn);

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);
    this.releaseFocusTrap = trapFocus(dialog);
  }

  private async applyAndSave(fileId: number, transforms: Transform[]): Promise<void> {
    const selected = await save({
      title: "Speichern unter",
      filters: [
        { name: "Stickdateien", extensions: ["pes", "dst"] },
      ],
    });
    if (!selected) return;

    const outputPath = selected;
    if (!outputPath) return;

    try {
      await FileService.saveTransformed(fileId, transforms, outputPath);
      ToastContainer.show("success", `Gespeichert: ${outputPath.split(/[\\/]/).pop()}`);
      this.close();
    } catch (e) {
      console.warn("Transform failed:", e);
      ToastContainer.show("error", "Bearbeitung fehlgeschlagen");
    }
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
    EditDialog.instance = null;
  }
}
