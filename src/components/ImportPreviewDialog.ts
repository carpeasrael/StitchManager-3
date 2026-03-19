import { ToastContainer } from "./Toast";
import { trapFocus } from "../utils/focus-trap";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { formatSize } from "../utils/format";
import { buildFolderTree, flattenVisibleTree } from "../utils/tree";
import * as ScannerService from "../services/ScannerService";
import * as FolderService from "../services/FolderService";
import * as FileService from "../services/FileService";
import type { ScannedFileInfo, BulkImportMetadata } from "../types/index";

type SortKey = "filename" | "fileSize" | "fileType";
type SortDir = "asc" | "desc";

export class ImportPreviewDialog {
  private static instance: ImportPreviewDialog | null = null;

  static open(
    files: ScannedFileInfo[],
    folderId: number | null,
    sourcePath?: string
  ): void {
    if (ImportPreviewDialog.instance) return;
    const dialog = new ImportPreviewDialog();
    ImportPreviewDialog.instance = dialog;
    dialog.show(files, folderId, sourcePath);
  }

  private overlay: HTMLElement | null = null;
  private releaseFocusTrap: (() => void) | null = null;
  private checked = new Set<string>();
  private sortKey: SortKey = "filename";
  private sortDir: SortDir = "asc";

  private show(
    files: ScannedFileInfo[],
    folderId: number | null,
    sourcePath?: string
  ): void {
    // Initialize checked state: all non-imported files checked
    this.checked.clear();
    for (const f of files) {
      if (!f.alreadyImported) {
        this.checked.add(f.filepath);
      }
    }

    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-import-preview";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Import Vorschau");

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";
    const title = document.createElement("h3");
    title.className = "dialog-title";
    title.textContent = "Import Vorschau";
    header.appendChild(title);
    const closeBtn = document.createElement("button");
    closeBtn.className = "dialog-close-btn";
    closeBtn.textContent = "\u00D7";
    closeBtn.title = "Schliessen";
    closeBtn.setAttribute("aria-label", "Dialog schliessen");
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    // Body
    const body = document.createElement("div");
    body.className = "dialog-body import-preview-body";

    // Summary bar
    const summaryBar = document.createElement("div");
    summaryBar.className = "import-preview-summary";
    body.appendChild(summaryBar);

    // Select all / none buttons
    const selectRow = document.createElement("div");
    selectRow.className = "import-preview-select-row";
    const selectAllBtn = document.createElement("button");
    selectAllBtn.className = "btn btn-small";
    selectAllBtn.textContent = "Alle auswaehlen";
    selectAllBtn.addEventListener("click", () => {
      for (const f of files) {
        if (!f.alreadyImported) this.checked.add(f.filepath);
      }
      renderList();
      updateSummary();
      updateImportBtn();
    });
    const selectNoneBtn = document.createElement("button");
    selectNoneBtn.className = "btn btn-small";
    selectNoneBtn.textContent = "Keine auswaehlen";
    selectNoneBtn.addEventListener("click", () => {
      this.checked.clear();
      renderList();
      updateSummary();
      updateImportBtn();
    });
    selectRow.appendChild(selectAllBtn);
    selectRow.appendChild(selectNoneBtn);
    body.appendChild(selectRow);

    // Column headers
    const headerRow = document.createElement("div");
    headerRow.className = "import-preview-header-row";
    const cbHeader = document.createElement("span");
    cbHeader.className = "import-col-cb";
    headerRow.appendChild(cbHeader);

    const makeHeader = (label: string, key: SortKey, cls: string) => {
      const span = document.createElement("span");
      span.className = `import-col-${cls} import-col-sortable`;
      span.textContent = label;
      if (this.sortKey === key) {
        span.textContent += this.sortDir === "asc" ? " \u25B2" : " \u25BC";
      }
      span.addEventListener("click", () => {
        if (this.sortKey === key) {
          this.sortDir = this.sortDir === "asc" ? "desc" : "asc";
        } else {
          this.sortKey = key;
          this.sortDir = "asc";
        }
        renderList();
        // Update header indicators
        headerRow.querySelectorAll(".import-col-sortable").forEach((el) => {
          const text = el.textContent?.replace(/ [▲▼]$/, "") ?? "";
          el.textContent = text;
        });
        span.textContent = label + (this.sortDir === "asc" ? " \u25B2" : " \u25BC");
      });
      headerRow.appendChild(span);
    };
    makeHeader("Dateiname", "filename", "name");
    makeHeader("Groesse", "fileSize", "size");
    makeHeader("Typ", "fileType", "type");

    body.appendChild(headerRow);

    // File list container
    const listContainer = document.createElement("div");
    listContainer.className = "import-preview-list";
    body.appendChild(listContainer);

    const renderList = () => {
      listContainer.innerHTML = "";
      const sorted = [...files].sort((a, b) => {
        let cmp = 0;
        switch (this.sortKey) {
          case "filename":
            cmp = a.filename.localeCompare(b.filename);
            break;
          case "fileSize":
            cmp = (a.fileSize ?? 0) - (b.fileSize ?? 0);
            break;
          case "fileType":
            cmp = a.fileType.localeCompare(b.fileType);
            break;
        }
        return this.sortDir === "asc" ? cmp : -cmp;
      });
      for (const file of sorted) {
        const row = document.createElement("div");
        row.className = "import-preview-row";
        if (file.alreadyImported) row.classList.add("imported");

        const cb = document.createElement("input");
        cb.type = "checkbox";
        cb.className = "import-col-cb";
        cb.checked = this.checked.has(file.filepath);
        cb.disabled = file.alreadyImported;
        cb.addEventListener("change", () => {
          if (cb.checked) {
            this.checked.add(file.filepath);
          } else {
            this.checked.delete(file.filepath);
          }
          updateSummary();
          updateImportBtn();
        });
        row.appendChild(cb);

        const nameSpan = document.createElement("span");
        nameSpan.className = "import-col-name";
        nameSpan.textContent = file.filename;
        row.appendChild(nameSpan);

        const sizeSpan = document.createElement("span");
        sizeSpan.className = "import-col-size";
        sizeSpan.textContent = file.fileSize ? formatSize(file.fileSize) : "-";
        row.appendChild(sizeSpan);

        const typeSpan = document.createElement("span");
        typeSpan.className = "import-col-type";
        const ext = file.extension?.toUpperCase() ?? "";
        const typeLabel = file.fileType === "sewing_pattern" ? "Schnittm." : "Stickm.";
        typeSpan.textContent = `${typeLabel} ${ext}`;
        row.appendChild(typeSpan);

        listContainer.appendChild(row);
      }
    };

    // Bulk metadata section
    const metaSection = document.createElement("div");
    metaSection.className = "import-preview-meta";

    const metaToggle = document.createElement("button");
    metaToggle.className = "btn btn-small";
    metaToggle.textContent = "Metadaten zuweisen \u25BC";
    let metaExpanded = false;
    const metaFields = document.createElement("div");
    metaFields.className = "import-preview-meta-fields";
    metaFields.style.display = "none";
    metaToggle.addEventListener("click", () => {
      metaExpanded = !metaExpanded;
      metaFields.style.display = metaExpanded ? "" : "none";
      metaToggle.textContent = metaExpanded
        ? "Metadaten zuweisen \u25B2"
        : "Metadaten zuweisen \u25BC";
    });
    metaSection.appendChild(metaToggle);

    // Tags
    const tagsGroup = document.createElement("div");
    tagsGroup.className = "settings-form-group";
    const tagsLabel = document.createElement("label");
    tagsLabel.className = "settings-label";
    tagsLabel.textContent = "Tags (kommasepariert)";
    const tagsInput = document.createElement("input");
    tagsInput.className = "settings-input";
    tagsInput.type = "text";
    tagsInput.placeholder = "z.B. Weihnachten, Blumen";
    tagsGroup.appendChild(tagsLabel);
    tagsGroup.appendChild(tagsInput);
    metaFields.appendChild(tagsGroup);

    // Theme
    const themeGroup = document.createElement("div");
    themeGroup.className = "settings-form-group";
    const themeLabel = document.createElement("label");
    themeLabel.className = "settings-label";
    themeLabel.textContent = "Thema";
    const themeInput = document.createElement("input");
    themeInput.className = "settings-input";
    themeInput.type = "text";
    themeInput.placeholder = "z.B. Weihnachten";
    themeGroup.appendChild(themeLabel);
    themeGroup.appendChild(themeInput);
    metaFields.appendChild(themeGroup);

    // Rating
    const ratingGroup = document.createElement("div");
    ratingGroup.className = "settings-form-group";
    const ratingLabel = document.createElement("label");
    ratingLabel.className = "settings-label";
    ratingLabel.textContent = "Bewertung";
    const ratingSelect = document.createElement("select");
    ratingSelect.className = "settings-input";
    const noRating = document.createElement("option");
    noRating.value = "";
    noRating.textContent = "-- Keine --";
    ratingSelect.appendChild(noRating);
    for (let i = 1; i <= 5; i++) {
      const opt = document.createElement("option");
      opt.value = String(i);
      opt.textContent = "\u2605".repeat(i);
      ratingSelect.appendChild(opt);
    }
    ratingGroup.appendChild(ratingLabel);
    ratingGroup.appendChild(ratingSelect);
    metaFields.appendChild(ratingGroup);

    // Sewing pattern fields (shown if any sewing patterns exist)
    const hasSewingPatterns = files.some((f) => f.fileType === "sewing_pattern");
    if (hasSewingPatterns) {
      const sewingHeader = document.createElement("div");
      sewingHeader.className = "settings-label";
      sewingHeader.textContent = "Schnittmuster-Felder";
      sewingHeader.style.marginTop = "8px";
      sewingHeader.style.fontWeight = "600";
      metaFields.appendChild(sewingHeader);

      const authorGroup = document.createElement("div");
      authorGroup.className = "settings-form-group";
      const authorLabel = document.createElement("label");
      authorLabel.className = "settings-label";
      authorLabel.textContent = "Designer";
      const authorInput = document.createElement("input");
      authorInput.className = "settings-input";
      authorInput.id = "import-preview-author";
      authorInput.type = "text";
      authorLabel.htmlFor = "import-preview-author";
      authorGroup.appendChild(authorLabel);
      authorGroup.appendChild(authorInput);
      metaFields.appendChild(authorGroup);

      const skillGroup = document.createElement("div");
      skillGroup.className = "settings-form-group";
      const skillLabel = document.createElement("label");
      skillLabel.className = "settings-label";
      skillLabel.textContent = "Schwierigkeitsgrad";
      const skillSelect = document.createElement("select");
      skillSelect.className = "settings-input";
      skillSelect.id = "import-preview-skill";
      skillLabel.htmlFor = "import-preview-skill";
      const noSkill = document.createElement("option");
      noSkill.value = "";
      noSkill.textContent = "-- Keine Angabe --";
      skillSelect.appendChild(noSkill);
      for (const level of ["beginner", "easy", "intermediate", "advanced", "expert"]) {
        const opt = document.createElement("option");
        opt.value = level;
        opt.textContent = level.charAt(0).toUpperCase() + level.slice(1);
        skillSelect.appendChild(opt);
      }
      skillGroup.appendChild(skillLabel);
      skillGroup.appendChild(skillSelect);
      metaFields.appendChild(skillGroup);
    }

    metaSection.appendChild(metaFields);
    body.appendChild(metaSection);

    // Target folder selector
    const folderGroup = document.createElement("div");
    folderGroup.className = "settings-form-group";
    const folderLabel = document.createElement("label");
    folderLabel.className = "settings-label";
    folderLabel.textContent = "Zielordner";
    folderLabel.htmlFor = "import-preview-folder";
    const folderSelect = document.createElement("select");
    folderSelect.className = "settings-input";
    folderSelect.id = "import-preview-folder";

    // "Neuer Ordner" option for creating a folder from the scanned path (only if sourcePath is available)
    const sourceBasename = sourcePath
      ? (sourcePath.replace(/\\/g, "/").split("/").filter(Boolean).pop() ?? "Neuer Ordner")
      : "";
    if (sourcePath) {
      const newOpt = document.createElement("option");
      newOpt.value = "__new__";
      newOpt.textContent = `+ Neuer Ordner (${sourceBasename})`;
      folderSelect.appendChild(newOpt);
    }

    const folders = appState.get("folders");
    const tree = buildFolderTree(folders);
    const allIds = new Set(folders.map((f) => f.id));
    const flatTree = flattenVisibleTree(tree, allIds);
    for (const entry of flatTree) {
      const opt = document.createElement("option");
      opt.value = String(entry.folder.id);
      opt.textContent = "\u00A0\u00A0".repeat(entry.depth) + entry.folder.name;
      folderSelect.appendChild(opt);
    }

    if (folderId !== null) {
      folderSelect.value = String(folderId);
    } else if (sourcePath) {
      // For mass-import: try to match existing folder by path, else default to "new"
      const match = folders.find((f) => f.path === sourcePath);
      if (match) {
        folderSelect.value = String(match.id);
      } else {
        folderSelect.value = "__new__";
      }
    } else if (folders.length > 0) {
      folderSelect.value = String(folders[0].id);
    }

    folderGroup.appendChild(folderLabel);
    folderGroup.appendChild(folderSelect);
    body.appendChild(folderGroup);

    dialog.appendChild(body);

    // Footer
    const footer = document.createElement("div");
    footer.className = "dialog-footer";
    const cancelBtn = document.createElement("button");
    cancelBtn.className = "btn btn-secondary";
    cancelBtn.textContent = "Abbrechen";
    cancelBtn.addEventListener("click", () => this.close());

    const importBtn = document.createElement("button");
    importBtn.className = "btn btn-primary";
    footer.appendChild(cancelBtn);
    footer.appendChild(importBtn);
    dialog.appendChild(footer);

    // Update functions
    const updateSummary = () => {
      const embCount = files.filter(
        (f) => f.fileType === "embroidery" && this.checked.has(f.filepath)
      ).length;
      const sewCount = files.filter(
        (f) => f.fileType === "sewing_pattern" && this.checked.has(f.filepath)
      ).length;
      const alreadyCount = files.filter((f) => f.alreadyImported).length;
      summaryBar.textContent = `${embCount} Stickmuster \u00B7 ${sewCount} Schnittmuster \u00B7 ${alreadyCount} bereits importiert`;
    };

    const updateImportBtn = () => {
      const count = this.checked.size;
      importBtn.textContent = `${count} Dateien importieren`;
      importBtn.disabled = count === 0;
    };

    // Import action
    importBtn.addEventListener("click", async () => {
      let selectedFolderId: number;

      if (folderSelect.value === "__new__") {
        // Create new folder from source path
        if (!sourcePath) {
          ToastContainer.show("error", "Kein Quellpfad fuer neuen Ordner verfuegbar");
          return;
        }
        try {
          const newFolder = await FolderService.create(sourceBasename, sourcePath);
          selectedFolderId = newFolder.id;
          const updatedFolders = await FolderService.getAll();
          appState.set("folders", updatedFolders);
        } catch (e) {
          const msg =
            e && typeof e === "object" && "message" in e
              ? (e as { message: string }).message
              : String(e);
          ToastContainer.show("error", `Ordner konnte nicht erstellt werden: ${msg}`);
          return;
        }
      } else {
        selectedFolderId = Number(folderSelect.value);
        if (!selectedFolderId) {
          ToastContainer.show("error", "Bitte einen Zielordner auswaehlen");
          return;
        }
      }

      const checkedPaths = files
        .filter((f) => this.checked.has(f.filepath) && !f.alreadyImported)
        .map((f) => f.filepath);
      if (checkedPaths.length === 0) {
        ToastContainer.show("info", "Keine Dateien zum Importieren ausgewaehlt");
        return;
      }

      // Build bulk metadata
      const meta: BulkImportMetadata = {};
      const tagStr = tagsInput.value.trim();
      if (tagStr) {
        meta.tags = tagStr.split(",").map((t) => t.trim()).filter(Boolean);
      }
      const themeVal = themeInput.value.trim();
      if (themeVal) meta.theme = themeVal;
      const ratingVal = ratingSelect.value;
      if (ratingVal) meta.rating = Number(ratingVal);

      if (hasSewingPatterns) {
        const authorEl = document.getElementById("import-preview-author") as HTMLInputElement | null;
        const skillEl = document.getElementById("import-preview-skill") as HTMLSelectElement | null;
        if (authorEl?.value.trim()) meta.author = authorEl.value.trim();
        if (skillEl?.value) meta.skillLevel = skillEl.value;
      }

      const hasMeta = Object.keys(meta).length > 0;

      importBtn.disabled = true;
      importBtn.textContent = "Importiere\u2026";

      try {
        const imported = await ScannerService.importFiles(
          checkedPaths,
          selectedFolderId,
          hasMeta ? meta : undefined
        );

        // Refresh state
        const updatedFolders = await FolderService.getAll();
        appState.set("folders", updatedFolders);
        appState.set("selectedFolderId", selectedFolderId);
        const updatedFiles = await FileService.getFiles(selectedFolderId);
        appState.set("files", updatedFiles);

        EventBus.emit("scan:complete", {
          folderId: selectedFolderId,
          foundFiles: imported.length,
        });

        ToastContainer.show(
          "success",
          `${imported.length} Datei(en) importiert`
        );
        this.close();
      } catch (e) {
        const msg =
          e && typeof e === "object" && "message" in e
            ? (e as { message: string }).message
            : String(e);
        ToastContainer.show("error", `Import fehlgeschlagen: ${msg}`);
        importBtn.disabled = false;
        updateImportBtn();
      }
    });

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);

    dialog.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this.close();
      }
    });

    // Initial render
    renderList();
    updateSummary();
    updateImportBtn();

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
    ImportPreviewDialog.instance = null;
  }
}
