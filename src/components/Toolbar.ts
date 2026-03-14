import { Component } from "./Component";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { ToastContainer } from "./Toast";
import { open } from "@tauri-apps/plugin-dialog";
import * as FolderService from "../services/FolderService";
import * as ScannerService from "../services/ScannerService";
import * as FileService from "../services/FileService";

interface MenuItem {
  className: string;
  icon: string;
  label: string;
  shortcut?: string;
  onClick: () => void;
}

interface MenuGroup {
  label: string;
  items: MenuItem[];
}

export class Toolbar extends Component {
  private menuOpen = false;
  private panel: HTMLElement | null = null;
  private outsideClickHandler: ((e: MouseEvent) => void) | null = null;

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("selectedFolderId", () => this.updateItemStates())
    );
    this.subscribe(
      appState.on("selectedFileId", () => this.updateItemStates())
    );
    this.subscribe(
      appState.on("selectedFileIds", () => this.updateItemStates())
    );
    this.subscribe(
      EventBus.on("burger:close", () => this.closeMenu())
    );
    this.render();
  }

  render(): void {
    this.el.innerHTML = "";

    const btn = document.createElement("button");
    btn.className = "burger-btn";
    btn.title = "Menue";
    btn.setAttribute("aria-label", "Menue oeffnen");
    btn.setAttribute("aria-haspopup", "true");
    btn.setAttribute("aria-expanded", "false");
    btn.textContent = "\u2630";
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      this.toggleMenu(btn);
    });
    this.el.appendChild(btn);
  }

  private getMenuGroups(): MenuGroup[] {
    return [
      {
        label: "Ordner",
        items: [
          {
            className: "menu-item-add",
            icon: "\uD83D\uDCC1",
            label: "Ordner hinzufuegen",
            onClick: () => this.addFolder(),
          },
          {
            className: "menu-item-scan",
            icon: "\uD83D\uDD0D",
            label: "Ordner scannen",
            onClick: () => this.scanFolder(),
          },
          {
            className: "menu-item-mass-import",
            icon: "\uD83D\uDCE5",
            label: "Massenimport",
            onClick: () => EventBus.emit("toolbar:mass-import"),
          },
        ],
      },
      {
        label: "Datei",
        items: [
          {
            className: "menu-item-save",
            icon: "\uD83D\uDCBE",
            label: "Speichern",
            shortcut: "Ctrl+S",
            onClick: () => EventBus.emit("toolbar:save"),
          },
          {
            className: "menu-item-reveal",
            icon: "\uD83D\uDCCD",
            label: "Im Ordner anzeigen",
            shortcut: "Ctrl+Shift+R",
            onClick: () => EventBus.emit("toolbar:reveal-in-folder"),
          },
          {
            className: "menu-item-pdf",
            icon: "\uD83D\uDCC4",
            label: "PDF Export",
            onClick: () => EventBus.emit("toolbar:pdf-export"),
          },
          {
            className: "menu-item-convert",
            icon: "\uD83D\uDD04",
            label: "Format konvertieren",
            onClick: () => EventBus.emit("toolbar:convert"),
          },
          {
            className: "menu-item-transfer",
            icon: "\uD83D\uDCE4",
            label: "An Maschine senden",
            onClick: () => EventBus.emit("toolbar:transfer"),
          },
          {
            className: "menu-item-edit-transform",
            icon: "\u2702",
            label: "Bearbeiten/Transformieren",
            onClick: () => EventBus.emit("toolbar:edit-transform"),
          },
          {
            className: "menu-item-versions",
            icon: "\uD83D\uDCCB",
            label: "Versionshistorie",
            onClick: () => EventBus.emit("toolbar:versions"),
          },
        ],
      },
      {
        label: "KI",
        items: [
          {
            className: "menu-item-ai",
            icon: "\u2728",
            label: "KI Analyse",
            onClick: () => EventBus.emit("toolbar:ai-analyze"),
          },
          {
            className: "menu-item-batch-ai",
            icon: "\u2728",
            label: "Batch KI",
            onClick: () => EventBus.emit("toolbar:batch-ai"),
          },
        ],
      },
      {
        label: "Batch",
        items: [
          {
            className: "menu-item-batch-rename",
            icon: "\u270F",
            label: "Batch Umbenennen",
            onClick: () => EventBus.emit("toolbar:batch-rename"),
          },
          {
            className: "menu-item-batch-organize",
            icon: "\uD83D\uDCC2",
            label: "Batch Organisieren",
            onClick: () => EventBus.emit("toolbar:batch-organize"),
          },
          {
            className: "menu-item-batch-export",
            icon: "\uD83D\uDCE4",
            label: "USB-Export",
            shortcut: "Ctrl+Shift+U",
            onClick: () => EventBus.emit("toolbar:batch-export"),
          },
        ],
      },
      {
        label: "System",
        items: [
          {
            className: "menu-item-settings",
            icon: "\u2699",
            label: "Einstellungen",
            shortcut: "Ctrl+,",
            onClick: () => EventBus.emit("toolbar:settings"),
          },
          {
            className: "menu-item-info",
            icon: "\u2139",
            label: "Info",
            onClick: () => EventBus.emit("toolbar:info"),
          },
        ],
      },
    ];
  }

  private toggleMenu(btn: HTMLButtonElement): void {
    if (this.menuOpen) {
      this.closeMenu();
    } else {
      this.openMenu(btn);
    }
  }

  private openMenu(btn: HTMLButtonElement): void {
    this.menuOpen = true;
    btn.setAttribute("aria-expanded", "true");
    btn.classList.add("burger-btn--open");

    this.panel = document.createElement("div");
    this.panel.className = "burger-menu";
    this.panel.setAttribute("role", "menu");

    const groups = this.getMenuGroups();
    for (let gi = 0; gi < groups.length; gi++) {
      const group = groups[gi];

      if (gi > 0) {
        const divider = document.createElement("div");
        divider.className = "burger-menu-divider";
        this.panel.appendChild(divider);
      }

      const header = document.createElement("div");
      header.className = "burger-menu-header";
      header.textContent = group.label;
      this.panel.appendChild(header);

      for (const item of group.items) {
        const row = document.createElement("button");
        row.className = `burger-menu-item ${item.className}`;
        row.setAttribute("role", "menuitem");

        const iconSpan = document.createElement("span");
        iconSpan.className = "burger-menu-item-icon";
        iconSpan.textContent = item.icon;
        row.appendChild(iconSpan);

        const labelSpan = document.createElement("span");
        labelSpan.className = "burger-menu-item-label";
        labelSpan.textContent = item.label;
        row.appendChild(labelSpan);

        if (item.shortcut) {
          const shortcutSpan = document.createElement("span");
          shortcutSpan.className = "burger-menu-item-shortcut";
          shortcutSpan.textContent = item.shortcut;
          row.appendChild(shortcutSpan);
        }

        row.addEventListener("click", () => {
          this.closeMenu();
          item.onClick();
        });

        this.panel.appendChild(row);
      }
    }

    this.el.appendChild(this.panel);
    this.updateItemStates();

    // Close on outside click (next tick to avoid immediate close)
    requestAnimationFrame(() => {
      if (!this.menuOpen) return;
      this.outsideClickHandler = (e: MouseEvent) => {
        if (this.panel && !this.el.contains(e.target as Node)) {
          this.closeMenu();
        }
      };
      document.addEventListener("click", this.outsideClickHandler);
    });
  }

  private closeMenu(): void {
    this.menuOpen = false;
    const btn = this.el.querySelector<HTMLButtonElement>(".burger-btn");
    if (btn) {
      btn.setAttribute("aria-expanded", "false");
      btn.classList.remove("burger-btn--open");
    }
    if (this.panel) {
      this.panel.remove();
      this.panel = null;
    }
    if (this.outsideClickHandler) {
      document.removeEventListener("click", this.outsideClickHandler);
      this.outsideClickHandler = null;
    }
  }

  private updateItemStates(): void {
    if (!this.panel) return;

    const hasFolder = appState.get("selectedFolderId") !== null;
    const hasFile = appState.get("selectedFileId") !== null;
    const multiCount = appState.get("selectedFileIds").length;
    const hasMulti = multiCount > 1;
    const hasAny = hasFile || hasMulti;

    const setDisabled = (cls: string, disabled: boolean) => {
      const el = this.panel?.querySelector<HTMLButtonElement>(`.${cls}`);
      if (el) el.disabled = disabled;
    };

    const setHidden = (cls: string, hidden: boolean) => {
      const el = this.panel?.querySelector<HTMLButtonElement>(`.${cls}`);
      if (el) el.style.display = hidden ? "none" : "";
    };

    setDisabled("menu-item-scan", !hasFolder);
    setDisabled("menu-item-reveal", !hasFile || hasMulti);
    setDisabled("menu-item-ai", !hasFile || hasMulti);
    setDisabled("menu-item-edit-transform", !hasFile || hasMulti);
    setDisabled("menu-item-versions", !hasFile || hasMulti);
    setDisabled("menu-item-convert", !hasAny);
    setDisabled("menu-item-transfer", !hasAny);

    setHidden("menu-item-pdf", !hasAny);
    setHidden("menu-item-batch-export", !hasAny);
    setHidden("menu-item-batch-rename", !hasMulti);
    setHidden("menu-item-batch-organize", !hasMulti);
    setHidden("menu-item-batch-ai", !hasMulti);
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

    try {
      const result = await ScannerService.scanDirectory(folder.path);

      if (result.foundFiles.length > 0) {
        await ScannerService.importFiles(result.foundFiles, folderId);
      }

      EventBus.emit("scan:complete", {
        folderId,
        foundFiles: result.foundFiles.length,
      });

      const files = await FileService.getFiles(folderId);
      appState.set("files", files);

      // Refresh folder counts after scan/import
      const updatedFolders = await FolderService.getAll();
      appState.set("folders", updatedFolders);
    } catch (e) {
      console.warn("Failed to scan folder:", e);
      ToastContainer.show("error", "Ordner konnte nicht gescannt werden");
    }
  }

  destroy(): void {
    this.closeMenu();
    super.destroy();
  }
}
