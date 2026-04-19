import { ToastContainer } from "./Toast";
import { trapFocus } from "../utils/focus-trap";
import { open } from "@tauri-apps/plugin-dialog";
import { appState } from "../state/AppState";
import { buildFolderTree, flattenVisibleTree } from "../utils/tree";
import * as FolderService from "../services/FolderService";
import type { FolderType } from "../types/index";

export class FolderDialog {
  private static instance: FolderDialog | null = null;

  static open(): void {
    if (FolderDialog.instance) return;
    const dialog = new FolderDialog();
    FolderDialog.instance = dialog;
    dialog.show();
  }

  private overlay: HTMLElement | null = null;
  private releaseFocusTrap: (() => void) | null = null;
  private autoName = "";

  private show(): void {
    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-folder";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Neuer Ordner");

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";

    const title = document.createElement("h3");
    title.className = "dialog-title";
    title.textContent = "Neuer Ordner";
    header.appendChild(title);

    const closeBtn = document.createElement("button");
    closeBtn.className = "dialog-close-btn";
    closeBtn.textContent = "\u00D7";
    closeBtn.title = "Schließen";
    closeBtn.setAttribute("aria-label", "Dialog schließen");
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);

    dialog.appendChild(header);

    // Body
    const body = document.createElement("div");
    body.className = "dialog-body dialog-folder-body";

    // Folder name
    const nameGroup = document.createElement("div");
    nameGroup.className = "settings-form-group";
    const nameLabel = document.createElement("label");
    nameLabel.className = "settings-label";
    nameLabel.textContent = "Ordnername";
    nameLabel.htmlFor = "folder-dialog-name";
    const nameInput = document.createElement("input");
    nameInput.className = "settings-input";
    nameInput.id = "folder-dialog-name";
    nameInput.type = "text";
    nameInput.placeholder = "z.B. Weihnachtsmuster";
    nameGroup.appendChild(nameLabel);
    nameGroup.appendChild(nameInput);
    body.appendChild(nameGroup);

    // Path with browse button
    const pathGroup = document.createElement("div");
    pathGroup.className = "settings-form-group";
    const pathLabel = document.createElement("label");
    pathLabel.className = "settings-label";
    pathLabel.textContent = "Pfad";
    pathLabel.htmlFor = "folder-dialog-path";
    const pathRow = document.createElement("div");
    pathRow.className = "folder-dialog-browse-row";
    const pathInput = document.createElement("input");
    pathInput.className = "settings-input";
    pathInput.id = "folder-dialog-path";
    pathInput.type = "text";
    pathInput.readOnly = true;
    pathInput.placeholder = "Verzeichnis auswählen...";
    const browseBtn = document.createElement("button");
    browseBtn.className = "btn btn-secondary";
    browseBtn.type = "button";
    browseBtn.textContent = "Durchsuchen\u2026";
    browseBtn.addEventListener("click", async () => {
      try {
        const selected = await open({
          directory: true,
          multiple: false,
          title: "Ordner auswählen",
        });
        if (!selected) return;
        const p = typeof selected === "string" ? selected : String(selected);
        if (!p) return;
        pathInput.value = p;

        // Auto-fill name from basename if name is empty or was auto-derived
        const basename =
          p.split("/").filter(Boolean).pop() ||
          p.split("\\").filter(Boolean).pop() ||
          p;
        if (!nameInput.value || nameInput.value === this.autoName) {
          nameInput.value = basename;
          this.autoName = basename;
        }
      } catch (e) {
        console.warn("Browse failed:", e);
      }
    });
    pathRow.appendChild(pathInput);
    pathRow.appendChild(browseBtn);
    pathGroup.appendChild(pathLabel);
    pathGroup.appendChild(pathRow);
    body.appendChild(pathGroup);

    // Parent folder dropdown
    const parentGroup = document.createElement("div");
    parentGroup.className = "settings-form-group";
    const parentLabel = document.createElement("label");
    parentLabel.className = "settings-label";
    parentLabel.textContent = "Übergeordneter Ordner";
    parentLabel.htmlFor = "folder-dialog-parent";
    const parentSelect = document.createElement("select");
    parentSelect.className = "settings-input";
    parentSelect.id = "folder-dialog-parent";

    const noneOption = document.createElement("option");
    noneOption.value = "";
    noneOption.textContent = "-- Keiner --";
    parentSelect.appendChild(noneOption);

    // Show folders in tree order with indentation
    const folders = appState.get("folders");
    const tree = buildFolderTree(folders);
    const allIds = new Set(folders.map((f) => f.id));
    const flatTree = flattenVisibleTree(tree, allIds);
    for (const entry of flatTree) {
      const opt = document.createElement("option");
      opt.value = String(entry.folder.id);
      opt.textContent = "\u00A0\u00A0".repeat(entry.depth) + entry.folder.name;
      parentSelect.appendChild(opt);
    }

    // Pre-select current folder if one is selected
    const currentFolderId = appState.get("selectedFolderId");
    if (currentFolderId !== null) {
      parentSelect.value = String(currentFolderId);
    }

    parentGroup.appendChild(parentLabel);
    parentGroup.appendChild(parentSelect);
    body.appendChild(parentGroup);

    // Folder type selector
    const typeGroup = document.createElement("div");
    typeGroup.className = "settings-form-group";
    const typeLabel = document.createElement("label");
    typeLabel.className = "settings-label";
    typeLabel.textContent = "Ordnertyp";
    typeLabel.htmlFor = "folder-dialog-type";
    const typeSelect = document.createElement("select");
    typeSelect.className = "settings-input";
    typeSelect.id = "folder-dialog-type";

    const types: { value: FolderType; label: string }[] = [
      { value: "mixed", label: "Gemischt" },
      { value: "embroidery", label: "Stickmuster" },
      { value: "sewing_pattern", label: "Schnittmuster" },
    ];
    for (const t of types) {
      const opt = document.createElement("option");
      opt.value = t.value;
      opt.textContent = t.label;
      typeSelect.appendChild(opt);
    }

    typeGroup.appendChild(typeLabel);
    typeGroup.appendChild(typeSelect);
    body.appendChild(typeGroup);

    dialog.appendChild(body);

    // Footer
    const footer = document.createElement("div");
    footer.className = "dialog-footer";

    const cancelBtn = document.createElement("button");
    cancelBtn.className = "btn btn-secondary";
    cancelBtn.textContent = "Abbrechen";
    cancelBtn.addEventListener("click", () => this.close());

    const createBtn = document.createElement("button");
    createBtn.className = "btn btn-primary";
    createBtn.textContent = "Erstellen";
    createBtn.addEventListener("click", async () => {
      const name = nameInput.value.trim();
      const path = pathInput.value.trim();

      if (!name) {
        ToastContainer.show("error", "Bitte einen Ordnernamen eingeben");
        nameInput.focus();
        return;
      }
      if (!path) {
        ToastContainer.show("error", "Bitte ein Verzeichnis auswählen");
        browseBtn.focus();
        return;
      }

      const parentId = parentSelect.value ? Number(parentSelect.value) : null;
      const folderType = typeSelect.value as FolderType;

      createBtn.disabled = true;
      try {
        await FolderService.create(name, path, parentId, folderType);
        const updatedFolders = await FolderService.getAll();
        appState.set("folders", updatedFolders);
        ToastContainer.show("success", `Ordner "${name}" erstellt`);
        this.close();
      } catch (e) {
        const msg =
          e && typeof e === "object" && "message" in e
            ? (e as { message: string }).message
            : String(e);
        ToastContainer.show("error", `Ordner konnte nicht erstellt werden: ${msg}`);
        createBtn.disabled = false;
      }
    });

    footer.appendChild(cancelBtn);
    footer.appendChild(createBtn);
    dialog.appendChild(footer);

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);

    // Escape to close
    dialog.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this.close();
      }
    });

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
    FolderDialog.instance = null;
  }
}
