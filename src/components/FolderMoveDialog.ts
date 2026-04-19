import { ToastContainer } from "./Toast";
import { extractBackendMessage } from "../utils/errors";
import { trapFocus } from "../utils/focus-trap";
import { appState } from "../state/AppState";
import { buildFolderTree, flattenVisibleTree, getDescendantIds } from "../utils/tree";
import * as FolderService from "../services/FolderService";

export class FolderMoveDialog {
  private static instance: FolderMoveDialog | null = null;

  static open(folderId: number): void {
    if (FolderMoveDialog.instance) return;
    const folders = appState.get("folders");
    if (!folders.find((f) => f.id === folderId)) return;
    const dialog = new FolderMoveDialog();
    FolderMoveDialog.instance = dialog;
    dialog.show(folderId);
  }

  private overlay: HTMLElement | null = null;
  private releaseFocusTrap: (() => void) | null = null;

  private show(folderId: number): void {
    const folders = appState.get("folders");
    const folder = folders.find((f) => f.id === folderId)!;

    const descendantIds = getDescendantIds(folders, folderId);
    const disabledIds = new Set([folderId, ...descendantIds]);

    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-folder-move";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Ordner verschieben");

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";
    const title = document.createElement("h3");
    title.className = "dialog-title";
    title.textContent = `"${folder.name}" verschieben nach\u2026`;
    header.appendChild(title);
    const closeBtn = document.createElement("button");
    closeBtn.className = "dialog-close-btn";
    closeBtn.textContent = "\u00D7";
    closeBtn.title = "Schließen";
    closeBtn.setAttribute("aria-label", "Dialog schließen");
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    // Body: tree list
    const body = document.createElement("div");
    body.className = "dialog-body dialog-folder-move-body";

    let selectedTarget: number | null | undefined = undefined;

    // Root option
    const rootItem = document.createElement("div");
    rootItem.className = "folder-move-item";
    rootItem.textContent = "Stammverzeichnis (Kein Elternordner)";
    if (folder.parentId === null) {
      rootItem.classList.add("disabled");
    } else {
      rootItem.addEventListener("click", () => {
        body.querySelectorAll(".folder-move-item.selected").forEach((el) =>
          el.classList.remove("selected")
        );
        rootItem.classList.add("selected");
        selectedTarget = null;
      });
    }
    body.appendChild(rootItem);

    // Build and flatten full tree (all expanded)
    const tree = buildFolderTree(folders);
    const allIds = new Set(folders.map((f) => f.id));
    const allVisible = flattenVisibleTree(tree, allIds);

    for (const entry of allVisible) {
      const item = document.createElement("div");
      item.className = "folder-move-item";
      item.style.paddingLeft = `${entry.depth * 16 + 24}px`;
      item.textContent = entry.folder.name;

      if (disabledIds.has(entry.folder.id) || entry.folder.id === folder.parentId) {
        item.classList.add("disabled");
      } else {
        item.addEventListener("click", () => {
          body.querySelectorAll(".folder-move-item.selected").forEach((el) =>
            el.classList.remove("selected")
          );
          item.classList.add("selected");
          selectedTarget = entry.folder.id;
        });
      }

      body.appendChild(item);
    }

    dialog.appendChild(body);

    // Footer
    const footer = document.createElement("div");
    footer.className = "dialog-footer";
    const cancelBtn = document.createElement("button");
    cancelBtn.className = "btn btn-secondary";
    cancelBtn.textContent = "Abbrechen";
    cancelBtn.addEventListener("click", () => this.close());

    const moveBtn = document.createElement("button");
    moveBtn.className = "btn btn-primary";
    moveBtn.textContent = "Verschieben";
    moveBtn.addEventListener("click", async () => {
      if (selectedTarget === undefined) {
        ToastContainer.show("error", "Bitte einen Zielordner auswählen");
        return;
      }
      moveBtn.disabled = true;
      try {
        await FolderService.moveFolder(folderId, selectedTarget);
        const updated = await FolderService.getAll();
        appState.set("folders", updated);
        // Auto-expand target parent
        if (selectedTarget !== null) {
          const expanded = appState.get("expandedFolderIds");
          if (!expanded.includes(selectedTarget)) {
            appState.set("expandedFolderIds", [...expanded, selectedTarget]);
          }
        }
        ToastContainer.show("success", `"${folder.name}" verschoben`);
        this.close();
      } catch (e) {
        const msg = extractBackendMessage(e, "Fehler");
        ToastContainer.show("error", `Verschieben fehlgeschlagen: ${msg}`);
        moveBtn.disabled = false;
      }
    });

    footer.appendChild(cancelBtn);
    footer.appendChild(moveBtn);
    dialog.appendChild(footer);

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);

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
    FolderMoveDialog.instance = null;
  }
}
