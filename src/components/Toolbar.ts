import { Component } from "./Component";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { open } from "@tauri-apps/plugin-dialog";
import * as FolderService from "../services/FolderService";
import * as ScannerService from "../services/ScannerService";
import * as FileService from "../services/FileService";

export class Toolbar extends Component {
  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("selectedFolderId", () => this.updateButtonStates())
    );
    this.subscribe(
      appState.on("selectedFileId", () => this.updateButtonStates())
    );
    this.subscribe(
      appState.on("selectedFileIds", () => this.updateButtonStates())
    );
    this.render();
  }

  render(): void {
    this.el.innerHTML = "";

    const actions = document.createElement("div");
    actions.className = "toolbar-actions";

    actions.appendChild(
      this.createButton("toolbar-btn-add", "\uD83D\uDCC1", "Ordner hinzuf\u00FCgen", () =>
        this.addFolder()
      )
    );

    actions.appendChild(
      this.createButton("toolbar-btn-scan", "\uD83D\uDD0D", "Ordner scannen", () =>
        this.scanFolder()
      )
    );

    actions.appendChild(
      this.createButton("toolbar-btn-mass-import", "\uD83D\uDCE5", "Massenimport", () =>
        EventBus.emit("toolbar:mass-import")
      )
    );

    actions.appendChild(
      this.createButton("toolbar-btn-save", "\uD83D\uDCBE", "Speichern", () =>
        EventBus.emit("toolbar:save")
      )
    );

    actions.appendChild(
      this.createButton("toolbar-btn-reveal", "\uD83D\uDCCD", "Im Ordner anzeigen", () =>
        EventBus.emit("toolbar:reveal-in-folder")
      )
    );

    actions.appendChild(
      this.createButton("toolbar-btn-ai", "\u2728", "KI Analyse", () =>
        EventBus.emit("toolbar:ai-analyze")
      )
    );

    // Batch actions (shown when multiple files selected)
    actions.appendChild(
      this.createButton("toolbar-btn-batch-rename", "\u270F", "Batch Umbenennen", () =>
        EventBus.emit("toolbar:batch-rename")
      )
    );

    actions.appendChild(
      this.createButton("toolbar-btn-batch-organize", "\uD83D\uDCC2", "Batch Organisieren", () =>
        EventBus.emit("toolbar:batch-organize")
      )
    );

    actions.appendChild(
      this.createButton("toolbar-btn-batch-export", "\uD83D\uDCE4", "USB-Export", () =>
        EventBus.emit("toolbar:batch-export")
      )
    );

    actions.appendChild(
      this.createButton("toolbar-btn-batch-ai", "\u2728", "Batch KI", () =>
        EventBus.emit("toolbar:batch-ai")
      )
    );

    const settingsBtn = this.createButton(
      "toolbar-btn-settings",
      "\u2699",
      "Einstellungen",
      () => EventBus.emit("toolbar:settings")
    );
    actions.appendChild(settingsBtn);

    this.el.appendChild(actions);
    this.updateButtonStates();
  }

  private createButton(
    className: string,
    icon: string,
    label: string,
    onClick: () => void
  ): HTMLButtonElement {
    const btn = document.createElement("button");
    btn.className = `toolbar-btn ${className}`;
    btn.title = label;

    const iconSpan = document.createElement("span");
    iconSpan.className = "toolbar-btn-icon";
    iconSpan.textContent = icon;
    btn.appendChild(iconSpan);

    const labelSpan = document.createElement("span");
    labelSpan.className = "toolbar-btn-label";
    labelSpan.textContent = label;
    btn.appendChild(labelSpan);

    btn.addEventListener("click", onClick);
    return btn;
  }

  private updateButtonStates(): void {
    const hasFolder = appState.get("selectedFolderId") !== null;
    const hasFile = appState.get("selectedFileId") !== null;
    const multiCount = appState.get("selectedFileIds").length;
    const hasMulti = multiCount > 1;

    const scanBtn = this.el.querySelector<HTMLButtonElement>(".toolbar-btn-scan");
    if (scanBtn) {
      scanBtn.disabled = !hasFolder;
      scanBtn.title = hasFolder ? "Ordner scannen" : "Ordner auswählen, um zu scannen";
    }

    const revealBtn = this.el.querySelector<HTMLButtonElement>(".toolbar-btn-reveal");
    if (revealBtn) revealBtn.disabled = !hasFile || hasMulti;

    const aiBtn = this.el.querySelector<HTMLButtonElement>(".toolbar-btn-ai");
    if (aiBtn) aiBtn.disabled = !hasFile || hasMulti;

    // USB-Export: visible when any file is selected (single or multi)
    const exportBtn = this.el.querySelector<HTMLButtonElement>(".toolbar-btn-batch-export");
    if (exportBtn) {
      exportBtn.style.display = hasFile || hasMulti ? "" : "none";
    }

    // Other batch buttons: visible only when multiple files selected
    const batchBtns = [
      ".toolbar-btn-batch-rename",
      ".toolbar-btn-batch-organize",
      ".toolbar-btn-batch-ai",
    ];
    for (const sel of batchBtns) {
      const btn = this.el.querySelector<HTMLButtonElement>(sel);
      if (btn) {
        btn.style.display = hasMulti ? "" : "none";
      }
    }
  }

  private async addFolder(): Promise<void> {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Ordner ausw\u00E4hlen",
      });
      if (!selected) return;

      const path = typeof selected === "string" ? selected : String(selected);
      if (!path) return;

      const folderName =
        path.split("/").filter(Boolean).pop() ||
        path.split("\\").filter(Boolean).pop() ||
        path;

      await FolderService.create(folderName, path);
      const folders = await FolderService.getAll();
      appState.set("folders", folders);
    } catch (e) {
      console.warn("Failed to add folder:", e);
    }
  }

  private async scanFolder(): Promise<void> {
    const folderId = appState.get("selectedFolderId");
    if (folderId === null) return;

    const folders = appState.get("folders");
    const folder = folders.find((f) => f.id === folderId);
    if (!folder) return;

    const scanBtn = this.el.querySelector<HTMLButtonElement>(".toolbar-btn-scan");
    if (scanBtn) {
      scanBtn.disabled = true;
      const label = scanBtn.querySelector(".toolbar-btn-label");
      if (label) label.textContent = "Scanne...";
    }

    try {
      const result = await ScannerService.scanDirectory(folder.path);

      if (result.foundFiles.length > 0) {
        await ScannerService.importFiles(result.foundFiles, folderId);
      }

      EventBus.emit("scan:complete", {
        folderId,
        foundFiles: result.foundFiles.length,
      });

      // Reload files for the selected folder
      const files = await FileService.getFiles(folderId);
      appState.set("files", files);
    } catch (e) {
      console.warn("Failed to scan folder:", e);
    } finally {
      if (scanBtn) {
        const label = scanBtn.querySelector(".toolbar-btn-label");
        if (label) label.textContent = "Ordner scannen";
      }
      this.updateButtonStates();
    }
  }
}
