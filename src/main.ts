import "./styles.css";
import { appState } from "./state/AppState";
import { EventBus } from "./state/EventBus";
import { Component } from "./components/Component";
import { Sidebar } from "./components/Sidebar";
import { SearchBar } from "./components/SearchBar";
import { FilterChips } from "./components/FilterChips";
import { FileList } from "./components/FileList";
import { MetadataPanel } from "./components/MetadataPanel";
import { Toolbar } from "./components/Toolbar";
import { StatusBar } from "./components/StatusBar";
import { AiPreviewDialog } from "./components/AiPreviewDialog";
import { AiResultDialog } from "./components/AiResultDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { BatchDialog } from "./components/BatchDialog";
import { ToastContainer } from "./components/Toast";
import { Splitter } from "./components/Splitter";
import { Dashboard } from "./components/Dashboard";
import { EditDialog } from "./components/EditDialog";
import { DocumentViewer } from "./components/DocumentViewer";
import { ImageViewerDialog } from "./components/ImageViewerDialog";
import { PrintPreviewDialog } from "./components/PrintPreviewDialog";
import * as ProjectService from "./services/ProjectService";
import { ProjectListDialog } from "./components/ProjectListDialog";
import { ManufacturingDialog } from "./components/ManufacturingDialog";
import * as BackupService from "./services/BackupService";
import { initShortcuts } from "./shortcuts";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import * as FileService from "./services/FileService";
import * as BatchService from "./services/BatchService";
import * as AiService from "./services/AiService";
import * as ScannerService from "./services/ScannerService";
import * as SettingsService from "./services/SettingsService";
import * as FolderService from "./services/FolderService";
import { applyFontSize } from "./utils/theme";
import type { ThemeMode, UsbDevice } from "./types/index";
import { LICENSE_TEXT, README_TEXT } from "./utils/app-texts";

async function initTheme(): Promise<void> {
  try {
    const settings = await SettingsService.getAllSettings();
    const theme: ThemeMode =
      settings.theme_mode === "dunkel" ? "dunkel" : "hell";
    applyTheme(theme);

    // Apply persisted font size
    const fontSize = settings.font_size || "medium";
    applyFontSize(fontSize);

    // Apply background image
    await applyBackground(settings);
  } catch (e) {
    console.warn("Failed to load theme from DB, using default:", e);
    applyTheme("hell");
    applyFontSize("medium");
  }
}

export async function applyBackground(
  settings?: Record<string, string>
): Promise<void> {
  const s = settings || (await SettingsService.getAllSettings());
  const opacity = s.bg_opacity || "0.15";
  const blur = s.bg_blur || "0";

  document.documentElement.style.setProperty("--bg-opacity", opacity);
  document.documentElement.style.setProperty("--bg-blur", blur + "px");

  if (s.bg_image_path) {
    try {
      const dataUri = await SettingsService.getBackgroundImage();
      if (dataUri) {
        document.documentElement.style.setProperty(
          "--bg-image",
          `url("${dataUri}")`
        );
      } else {
        document.documentElement.style.setProperty("--bg-image", "none");
      }
    } catch {
      document.documentElement.style.setProperty("--bg-image", "none");
    }
  } else {
    document.documentElement.style.setProperty("--bg-image", "none");
  }
}

function applyTheme(theme: ThemeMode): void {
  document.documentElement.setAttribute("data-theme", theme);
  appState.set("theme", theme);
}

async function toggleTheme(): Promise<void> {
  const current = appState.get("theme");
  const next: ThemeMode = current === "hell" ? "dunkel" : "hell";
  applyTheme(next);

  try {
    await SettingsService.setSetting("theme_mode", next);
  } catch (e) {
    console.warn("Failed to persist theme to DB:", e);
  }
}

type UnlistenFn = () => void;
let tauriBridgeCleanup: UnlistenFn[] = [];

async function initTauriBridge(): Promise<void> {
  tauriBridgeCleanup = await Promise.all([
    listen("scan:progress", (e) => EventBus.emit("scan:progress", e.payload)),
    listen("scan:file-found", (e) =>
      EventBus.emit("scan:file-found", e.payload)
    ),
    listen("scan:complete", (e) => EventBus.emit("scan:complete", e.payload)),
    listen("batch:progress", (e) =>
      EventBus.emit("batch:progress", e.payload)
    ),
    listen("import:discovery", (e) =>
      EventBus.emit("import:discovery", e.payload)
    ),
    listen("import:progress", (e) =>
      EventBus.emit("import:progress", e.payload)
    ),
    listen("watcher:status", (e) =>
      EventBus.emit("watcher:status", e.payload)
    ),
    listen("fs:new-files", (e) =>
      EventBus.emit("fs:new-files", e.payload)
    ),
    listen("fs:files-removed", (e) =>
      EventBus.emit("fs:files-removed", e.payload)
    ),
    listen("usb:connected", (e) =>
      EventBus.emit("usb:connected", e.payload)
    ),
    listen("usb:disconnected", (e) =>
      EventBus.emit("usb:disconnected", e.payload)
    ),
    listen("ai:start", (e) => EventBus.emit("ai:start", e.payload)),
    listen("ai:complete", (e) => EventBus.emit("ai:complete", e.payload)),
    listen("ai:error", (e) => EventBus.emit("ai:error", e.payload)),
  ]);
}

export function destroyTauriBridge(): void {
  tauriBridgeCleanup.forEach((unlisten) => unlisten());
  tauriBridgeCleanup = [];
}

function setupThemeToggle(): () => void {
  const menuEl = document.querySelector(".app-menu");
  if (!menuEl) return () => {};

  const btn = document.createElement("button");
  btn.className = "menu-theme-btn";
  btn.textContent = "\u25D0";
  btn.title = "Theme wechseln";
  const onClick = () => toggleTheme();
  btn.addEventListener("click", onClick);
  menuEl.appendChild(btn);
  return () => {
    btn.removeEventListener("click", onClick);
    btn.remove();
  };
}

async function revealSelectedFile(): Promise<void> {
  const fileId = appState.get("selectedFileId");
  if (fileId === null) return;

  const files = appState.getRef("files");
  const file = files.find((f) => f.id === fileId);
  if (!file?.filepath) return;

  try {
    await revealItemInDir(file.filepath);
  } catch (e) {
    console.warn("Failed to reveal file in folder:", e);
    ToastContainer.show("error", "Datei konnte nicht im Ordner angezeigt werden");
  }
}

function showTextPopup(title: string, text: string): void {
  const overlay = document.createElement("div");
  overlay.className = "dialog-overlay";
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) overlay.remove();
  });
  overlay.addEventListener("dialog-dismiss", () => overlay.remove());

  const dialog = document.createElement("div");
  dialog.className = "dialog dialog-text-popup";
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("aria-modal", "true");
  dialog.setAttribute("aria-label", title);

  const header = document.createElement("div");
  header.className = "text-popup-header";
  const h3 = document.createElement("h3");
  h3.textContent = title;
  header.appendChild(h3);
  const closeX = document.createElement("button");
  closeX.className = "text-popup-close-x";
  closeX.textContent = "\u00D7";
  closeX.addEventListener("click", () => overlay.remove());
  header.appendChild(closeX);
  dialog.appendChild(header);

  const content = document.createElement("pre");
  content.className = "text-popup-content";
  content.textContent = text;
  dialog.appendChild(content);

  overlay.appendChild(dialog);
  document.body.appendChild(overlay);
}

function showInfoDialog(): void {
  const overlay = document.createElement("div");
  overlay.className = "dialog-overlay";
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) overlay.remove();
  });
  overlay.addEventListener("dialog-dismiss", () => overlay.remove());

  const dialog = document.createElement("div");
  dialog.className = "dialog dialog-info";
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("aria-modal", "true");
  dialog.setAttribute("aria-label", "Info");

  dialog.innerHTML = `
    <h3 class="info-title">Stitch Manager</h3>
    <div class="info-subtitle">Stickdateien-Verwaltung</div>
    <div class="info-version">Version 26.4.1 (26.04-a1)</div>
    <div class="info-details">
      <div class="info-row"><span class="info-label">Autor</span><span>carpeasrael</span></div>
      <div class="info-row"><span class="info-label">E-Mail</span><a href="mailto:carpeasrael@chaostribunal.de">carpeasrael@chaostribunal.de</a></div>
      <div class="info-row"><span class="info-label">GitHub</span><a href="https://github.com/carpeasrael/StitchManager-3" target="_blank">carpeasrael/StitchManager-3</a></div>
      <div class="info-row"><span class="info-label">Lizenz</span><span>GPL-3.0</span></div>
      <div class="info-row"><span class="info-label">Technologie</span><span>Tauri v2 + Rust + TypeScript</span></div>
    </div>
    <div class="info-links"></div>
    <button class="info-close-btn">Schliessen</button>
  `;

  const linksEl = dialog.querySelector(".info-links")!;

  const readmeBtn = document.createElement("button");
  readmeBtn.className = "info-link-btn";
  readmeBtn.textContent = "README anzeigen";
  readmeBtn.addEventListener("click", () => {
    showTextPopup("README", README_TEXT);
  });
  linksEl.appendChild(readmeBtn);

  const licenseBtn = document.createElement("button");
  licenseBtn.className = "info-link-btn";
  licenseBtn.textContent = "Lizenz anzeigen";
  licenseBtn.addEventListener("click", () => {
    showTextPopup("LICENSE \u2014 GPL-3.0", LICENSE_TEXT);
  });
  linksEl.appendChild(licenseBtn);

  dialog.querySelector(".info-close-btn")!.addEventListener("click", () => overlay.remove());

  overlay.appendChild(dialog);
  document.body.appendChild(overlay);
}

async function deleteSelectedFiles(): Promise<void> {
  const multiIds = appState.get("selectedFileIds");
  const singleId = appState.get("selectedFileId");
  const fileIds = multiIds.length > 1 ? multiIds : singleId !== null ? [singleId] : [];
  if (fileIds.length === 0) return;

  const files = appState.getRef("files");

  if (fileIds.length === 1) {
    const file = files.find((f) => f.id === fileIds[0]);
    const label = file ? (file.name || file.filename) : `ID ${fileIds[0]}`;
    if (!confirm(`Datei "${label}" wirklich loeschen?`)) return;
  } else {
    if (!confirm(`${fileIds.length} Dateien wirklich loeschen?`)) return;
  }

  // Soft-delete (move to trash) instead of hard delete
  let deleted = 0;
  for (const id of fileIds) {
    try {
      await BackupService.softDeleteFile(id);
      deleted++;
    } catch (e) {
      console.warn(`Failed to soft-delete file ${id}:`, e);
    }
  }

  appState.set("selectedFileIds", []);
  appState.set("selectedFileId", null);
  await reloadFilesAndCounts();

  if (deleted === fileIds.length) {
    ToastContainer.show("success", deleted === 1 ? "Datei in Papierkorb verschoben" : `${deleted} Dateien in Papierkorb verschoben`);
  } else if (deleted > 0) {
    ToastContainer.show("info", `${deleted} von ${fileIds.length} Dateien in Papierkorb verschoben`);
  } else {
    ToastContainer.show("error", "Dateien konnten nicht geloescht werden");
  }
}

function initEventHandlers(): () => void {
  const unsubs = [
    EventBus.on("toolbar:ai-analyze", async () => {
      const fileId = appState.get("selectedFileId");
      if (fileId === null) return;

      const files = appState.getRef("files");
      const file = files.find((f) => f.id === fileId);
      if (!file) return;

      await AiPreviewDialog.open(fileId, file, async (result) => {
        await AiResultDialog.open(result, fileId);
        await reloadFiles();
      });
    }),

    EventBus.on("toolbar:settings", () => {
      SettingsDialog.open();
    }),

    EventBus.on("toolbar:info", () => {
      showInfoDialog();
    }),

    EventBus.on("toolbar:save", () => {
      EventBus.emit("metadata:save");
    }),

    EventBus.on("viewer:open", (data) => {
      const { filePath, fileId, fileName } = data as { filePath: string; fileId: number; fileName: string };
      const ext = filePath.split(".").pop()?.toLowerCase() || "";
      if (ext === "pdf") {
        DocumentViewer.open(filePath, fileId, fileName);
      } else if (["png", "jpg", "jpeg", "svg", "gif", "webp", "bmp"].includes(ext)) {
        ImageViewerDialog.open([{ filePath, displayName: fileName }]);
      }
    }),

    EventBus.on("toolbar:print", async () => {
      const fileId = appState.get("selectedFileId");
      if (fileId === null) return;
      const files = appState.getRef("files");
      const file = files.find((f) => f.id === fileId);
      if (!file?.filepath) return;
      const ext = file.filepath.split(".").pop()?.toLowerCase() || "";
      if (ext !== "pdf") {
        ToastContainer.show("info", "Nur PDF-Dateien koennen gedruckt werden");
        return;
      }
      await PrintPreviewDialog.open(file.filepath, fileId, file.name || file.filename);
    }),

    EventBus.on("toolbar:backup", async () => {
      try {
        const result = await BackupService.createBackup(false);
        ToastContainer.show("success", `Backup erstellt: ${result.fileCount} Dateien (${(result.sizeBytes / 1024 / 1024).toFixed(1)} MB)`);
      } catch (e) {
        console.warn("Backup failed:", e);
        ToastContainer.show("error", "Backup fehlgeschlagen");
      }
    }),

    EventBus.on("toolbar:trash", async () => {
      try {
        const items = await BackupService.getTrash();
        if (items.length === 0) {
          ToastContainer.show("info", "Papierkorb ist leer");
          return;
        }
        // Show info and offer restore only. Purge is a separate toolbar action.
        const restoreAll = confirm(
          `${items.length} Dateien im Papierkorb.\n\nAlle wiederherstellen?`
        );
        if (restoreAll) {
          for (const [id] of items) await BackupService.restoreFile(id);
          ToastContainer.show("success", `${items.length} Dateien wiederhergestellt`);
        }
        EventBus.emit("file:refresh");
      } catch (e) {
        console.warn("Trash operation failed:", e);
        ToastContainer.show("error", "Papierkorb-Aktion fehlgeschlagen");
      }
    }),

    EventBus.on("toolbar:purge-trash", async () => {
      try {
        const items = await BackupService.getTrash();
        if (items.length === 0) {
          ToastContainer.show("info", "Papierkorb ist leer");
          return;
        }
        if (confirm(`${items.length} Dateien endgueltig loeschen?\n\nDiese Aktion kann nicht rueckgaengig gemacht werden.`)) {
          for (const [id] of items) await BackupService.purgeFile(id);
          ToastContainer.show("success", "Papierkorb geleert");
          EventBus.emit("file:refresh");
        }
      } catch (e) {
        console.warn("Purge failed:", e);
        ToastContainer.show("error", "Papierkorb leeren fehlgeschlagen");
      }
    }),

    EventBus.on("toolbar:export-metadata", async () => {
      const fileIds = appState.get("selectedFileIds");
      const singleId = appState.get("selectedFileId");
      const ids = fileIds.length > 0 ? fileIds : singleId ? [singleId] : [];
      if (ids.length === 0) {
        ToastContainer.show("info", "Keine Dateien ausgewaehlt");
        return;
      }
      try {
        const json = await BackupService.exportMetadataJson(ids);
        // Download as file using blob
        const blob = new Blob([json], { type: "application/json" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "stitchmanager_export.json";
        a.click();
        URL.revokeObjectURL(url);
        ToastContainer.show("success", `${ids.length} Dateien exportiert`);
      } catch (e) {
        console.warn("Export failed:", e);
        ToastContainer.show("error", "Export fehlgeschlagen");
      }
    }),

    EventBus.on("toolbar:show-projects", () => {
      ProjectListDialog.open();
    }),

    EventBus.on("toolbar:manufacturing", () => {
      ManufacturingDialog.open();
    }),

    EventBus.on("pattern:upload", async () => {
      const { PatternUploadDialog } = await import("./components/PatternUploadDialog");
      PatternUploadDialog.open();
    }),

    EventBus.on("collection:selected", async (data) => {
      const { collectionId } = data as { collectionId: number; collectionName: string };
      try {
        const fileIds = await ProjectService.getCollectionFiles(collectionId);
        if (fileIds.length === 0) {
          ToastContainer.show("info", "Sammlung ist leer");
          return;
        }
        // Fetch full file objects from backend (not in-memory filter)
        const files = await FileService.getFilesByIds(fileIds);
        appState.set("selectedFileId", null);
        appState.set("selectedFileIds", []);
        appState.set("selectedFolderId", null);
        appState.set("files", files);
        ToastContainer.show("info", `${files.length} Dateien in Sammlung`);
      } catch (e) {
        console.warn("Failed to load collection files:", e);
      }
    }),

    EventBus.on("project:create-from-pattern", async (data) => {
      const { patternFileId, patternName } = data as { patternFileId: number; patternName: string };
      try {
        const project = await ProjectService.createProject({
          name: `Projekt: ${patternName}`,
          patternFileId,
        });
        ToastContainer.show("success", `Projekt "${project.name}" erstellt`);
      } catch (e) {
        console.warn("Failed to create project:", e);
        ToastContainer.show("error", "Projekt konnte nicht erstellt werden");
      }
    }),

    EventBus.on("toolbar:reveal-in-folder", () => revealSelectedFile()),
    EventBus.on("shortcut:reveal-in-folder", () => revealSelectedFile()),
    EventBus.on("shortcut:usb-export", () => EventBus.emit("toolbar:batch-export")),

    EventBus.on("toolbar:batch-rename", async () => {
      const fileIds = appState.get("selectedFileIds");
      if (fileIds.length === 0) return;

      const settings = await SettingsService.getAllSettings();
      const pattern = settings.rename_pattern || "{name}_{theme}";

      BatchDialog.open("Batch Umbenennen", fileIds.length);
      try {
        const result = await BatchService.rename(fileIds, pattern);
        if (result.failed > 0) {
          ToastContainer.show("error", `${result.success} umbenannt, ${result.failed} fehlgeschlagen`);
        } else {
          ToastContainer.show("success", `${result.success} Dateien umbenannt`);
        }
      } catch (e) {
        console.warn("Batch rename failed:", e);
        ToastContainer.show("error", "Batch-Umbenennung fehlgeschlagen");
      }
      await reloadFilesAndCounts();
    }),

    EventBus.on("toolbar:batch-organize", async () => {
      const fileIds = appState.get("selectedFileIds");
      if (fileIds.length === 0) return;

      const settings = await SettingsService.getAllSettings();
      const pattern = settings.organize_pattern || "{theme}/{name}";

      BatchDialog.open("Batch Organisieren", fileIds.length);
      try {
        const result = await BatchService.organize(fileIds, pattern);
        if (result.failed > 0) {
          ToastContainer.show("error", `${result.success} organisiert, ${result.failed} fehlgeschlagen`);
        } else {
          ToastContainer.show("success", `${result.success} Dateien organisiert`);
        }
      } catch (e) {
        console.warn("Batch organize failed:", e);
        ToastContainer.show("error", "Batch-Organisation fehlgeschlagen");
      }
      await reloadFilesAndCounts();
    }),

    EventBus.on("toolbar:batch-export", async () => {
      let fileIds = appState.get("selectedFileIds");
      if (fileIds.length === 0) {
        const singleId = appState.get("selectedFileId");
        if (singleId === null) return;
        fileIds = [singleId];
      }

      const selected = await open({
        directory: true,
        multiple: false,
        title: "Zielordner f\u00FCr USB-Export w\u00E4hlen",
      });
      if (!selected) return;

      const targetPath = typeof selected === "string" ? selected : String(selected);
      if (!targetPath) return;

      if (fileIds.length === 1) {
        try {
          await BatchService.exportUsb(fileIds, targetPath);
          ToastContainer.show("success", "Datei exportiert");
        } catch (e) {
          console.warn("USB export failed:", e);
          ToastContainer.show("error", "Export fehlgeschlagen");
        }
      } else {
        BatchDialog.open("USB-Export", fileIds.length);
        try {
          await BatchService.exportUsb(fileIds, targetPath);
          ToastContainer.show("success", `${fileIds.length} Dateien exportiert`);
        } catch (e) {
          console.warn("Batch export failed:", e);
          ToastContainer.show("error", "Export fehlgeschlagen");
        }
      }
    }),

    EventBus.on("toolbar:delete-folder", async () => {
      const folderId = appState.get("selectedFolderId");
      if (folderId === null) return;

      const folders = appState.get("folders");
      const folder = folders.find((f) => f.id === folderId);
      if (!folder) return;

      // Get file count for confirmation message
      let fileCount = 0;
      try {
        fileCount = await FolderService.getFileCount(folderId);
      } catch {
        // Fall back to zero if count query fails
      }

      // Check for subfolders
      const hasSubfolders = folders.some((f) => f.parentId === folderId);

      let msg = `Ordner "${folder.name}"`;
      if (hasSubfolders) msg += " und Unterordner";
      if (fileCount > 0) msg += ` mit ${fileCount} Datei(en)`;
      msg += " wirklich l\u00F6schen?";
      if (!confirm(msg)) return;

      try {
        await FolderService.remove(folderId);
        appState.set("selectedFileIds", []);
        appState.set("selectedFileId", null);
        appState.set("selectedFolderId", null);
        const updatedFolders = await FolderService.getAll();
        appState.set("folders", updatedFolders);
        appState.set("files", []);
        ToastContainer.show("success", `Ordner "${folder.name}" gel\u00F6scht`);
      } catch (e) {
        console.warn("Failed to delete folder:", e);
        ToastContainer.show("error", "Ordner konnte nicht gel\u00F6scht werden");
      }
    }),

    EventBus.on("toolbar:mass-import", async () => {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Ordner f\u00FCr Massenimport w\u00E4hlen",
      });
      if (!selected) return;

      const path = typeof selected === "string" ? selected : String(selected);
      if (!path) return;

      BatchDialog.open("Massenimport", 0, "import");

      try {
        const result = await ScannerService.massImport(path);

        // Reload folders (new folder may have been created)
        const folders = await FolderService.getAll();
        appState.set("folders", folders);

        // Select the imported folder and reload files
        appState.set("selectedFileIds", []);
        appState.set("selectedFileId", null);
        appState.set("selectedFolderId", result.folderId);
        await reloadFiles();

        const elapsed = (result.elapsedMs / 1000).toFixed(1);
        ToastContainer.show(
          "success",
          `${result.importedCount} Dateien importiert, ${result.skippedCount} übersprungen (${elapsed}s)`
        );
      } catch (e) {
        console.warn("Mass import failed:", e);
        ToastContainer.show("error", "Massenimport fehlgeschlagen");
      }
    }),

    EventBus.on("migration:2stitch", async () => {
      const dialog = BatchDialog.open("2stitch Import", 0, "import");

      try {
        const result = await ScannerService.migrateFrom2Stitch();
        dialog.close();

        // Reload folders and files
        const folders = await FolderService.getAll();
        appState.set("folders", folders);
        appState.set("selectedFileIds", []);
        appState.set("selectedFileId", null);
        appState.set("selectedFolderId", null);
        await reloadFiles();

        const elapsed = (result.elapsedMs / 1000).toFixed(1);
        ToastContainer.show(
          "success",
          `${result.filesImported} Dateien, ${result.foldersCreated} Ordner, ${result.tagsImported} Tags importiert (${elapsed}s)`
        );
      } catch (e) {
        dialog.close();
        console.warn("2stitch migration failed:", e);
        ToastContainer.show("error", "2stitch Import fehlgeschlagen");
      }
    }),

    EventBus.on("toolbar:batch-ai", async () => {
      const fileIds = appState.get("selectedFileIds");
      if (fileIds.length === 0) return;

      BatchDialog.open("Batch KI-Analyse", fileIds.length);
      try {
        const results = await AiService.analyzeBatch(fileIds);
        if (results && results.length > 0) {
          ToastContainer.show("success", `${results.length} Dateien analysiert`);
        }
      } catch (e) {
        console.warn("Batch AI analysis failed:", e);
        ToastContainer.show("error", "Batch-KI-Analyse fehlgeschlagen");
      }
      await reloadFiles();
    }),

    EventBus.on("toolbar:versions", async () => {
      const fileId = appState.get("selectedFileId");
      if (fileId === null) return;
      const files = appState.getRef("files");
      const file = files.find((f) => f.id === fileId);
      if (!file) return;

      try {
        const versions = await FileService.getFileVersions(fileId);
        if (versions.length === 0) {
          ToastContainer.show("info", "Keine Versionen vorhanden");
          return;
        }
        const lines = versions.map(
          (v) => `v${v.versionNumber}: ${v.operation} — ${v.createdAt} (${(v.fileSize / 1024).toFixed(0)} KB)`
        );
        showTextPopup(`Versionshistorie — ${file.name || file.filename}`, lines.join("\n"));
      } catch (e) {
        console.warn("Failed to load versions:", e);
        ToastContainer.show("error", "Versionshistorie konnte nicht geladen werden");
      }
    }),

    EventBus.on("toolbar:edit-transform", async () => {
      const fileId = appState.get("selectedFileId");
      if (fileId === null) return;
      const files = appState.getRef("files");
      const file = files.find((f) => f.id === fileId);
      if (!file) return;
      await EditDialog.open(fileId, file.name || file.filename);
    }),

    EventBus.on("toolbar:convert", async () => {
      let fileIds = appState.get("selectedFileIds");
      if (fileIds.length === 0) {
        const singleId = appState.get("selectedFileId");
        if (singleId === null) return;
        fileIds = [singleId];
      }

      // Get supported formats
      let formats: string[];
      try {
        formats = await FileService.getSupportedFormats();
      } catch {
        ToastContainer.show("error", "Formate konnten nicht geladen werden");
        return;
      }

      const format = prompt(`Zielformat waehlen (${formats.join(", ")}):`);
      if (!format) return;
      const upper = format.trim().toUpperCase();
      if (!formats.includes(upper)) {
        ToastContainer.show("error", `Unbekanntes Format: ${format}`);
        return;
      }

      const selected = await open({
        directory: true,
        multiple: false,
        title: "Zielordner fuer Konvertierung waehlen",
      });
      if (!selected) return;
      const outputDir = typeof selected === "string" ? selected : String(selected);

      if (fileIds.length === 1) {
        try {
          const path = await FileService.convertFile(fileIds[0], upper, outputDir);
          ToastContainer.show("success", `Konvertiert: ${path.split(/[\\/]/).pop()}`);
        } catch (e) {
          console.warn("Convert failed:", e);
          ToastContainer.show("error", "Konvertierung fehlgeschlagen");
        }
      } else {
        try {
          const result = await FileService.convertFilesBatch(fileIds, upper, outputDir);
          ToastContainer.show(
            result.failed > 0 ? "error" : "success",
            `${result.success} von ${result.total} Dateien konvertiert`
          );
        } catch (e) {
          console.warn("Batch convert failed:", e);
          ToastContainer.show("error", "Batch-Konvertierung fehlgeschlagen");
        }
      }
    }),

    EventBus.on("toolbar:transfer", async () => {
      let fileIds = appState.get("selectedFileIds");
      if (fileIds.length === 0) {
        const singleId = appState.get("selectedFileId");
        if (singleId === null) return;
        fileIds = [singleId];
      }

      // Get configured machines
      let machines: { id: number; name: string }[];
      try {
        machines = await FileService.listMachines();
      } catch {
        ToastContainer.show("error", "Maschinen konnten nicht geladen werden");
        return;
      }

      if (machines.length === 0) {
        ToastContainer.show("info", "Keine Maschinen konfiguriert. Bitte in den Einstellungen hinzufuegen.");
        return;
      }

      const machineNames = machines.map((m, i) => `${i + 1}. ${m.name}`).join("\n");
      const choice = prompt(`Maschine waehlen:\n${machineNames}\n\nNummer eingeben:`);
      if (!choice) return;

      const idx = parseInt(choice, 10) - 1;
      if (isNaN(idx) || idx < 0 || idx >= machines.length) {
        ToastContainer.show("error", "Ungueltige Auswahl");
        return;
      }

      try {
        const result = await FileService.transferFiles(machines[idx].id, fileIds);
        ToastContainer.show(
          result.failed > 0 ? "error" : "success",
          `${result.success} von ${result.total} Dateien uebertragen`
        );
      } catch (e) {
        console.warn("Transfer failed:", e);
        ToastContainer.show("error", "Uebertragung fehlgeschlagen");
      }
    }),

    EventBus.on("toolbar:pdf-export", async () => {
      const multiIds = appState.get("selectedFileIds");
      const singleId = appState.get("selectedFileId");
      const fileIds = multiIds.length > 0 ? multiIds : singleId !== null ? [singleId] : [];
      if (fileIds.length === 0) return;

      try {
        const pdfPath = await FileService.generatePdfReport(fileIds);
        ToastContainer.show("success", `PDF erstellt: ${pdfPath.split(/[\\/]/).pop()}`);
        await revealItemInDir(pdfPath);
      } catch (e) {
        console.warn("PDF export failed:", e);
        ToastContainer.show("error", "PDF-Export fehlgeschlagen");
      }
    }),

    EventBus.on("usb:connected", (data) => {
      const device = data as UsbDevice;
      appState.update("usbDevices", (current) => [
        ...current.filter((d) => d.mountPoint !== device.mountPoint),
        device,
      ]);
    }),

    EventBus.on("usb:disconnected", (data) => {
      const device = data as UsbDevice;
      appState.update("usbDevices", (current) =>
        current.filter((d) => d.mountPoint !== device.mountPoint)
      );
    }),

    EventBus.on("usb:quick-export", async () => {
      const devices = appState.get("usbDevices");
      if (devices.length === 0) return;

      let fileIds = appState.get("selectedFileIds");
      if (fileIds.length === 0) {
        const singleId = appState.get("selectedFileId");
        if (singleId === null) {
          ToastContainer.show("info", "Keine Dateien ausgewaehlt");
          return;
        }
        fileIds = [singleId];
      }

      const targetPath = devices[0].mountPoint;

      if (fileIds.length === 1) {
        try {
          await BatchService.exportUsb(fileIds, targetPath);
          ToastContainer.show("success", `Datei auf ${devices[0].name} exportiert`);
        } catch {
          ToastContainer.show("error", "Export fehlgeschlagen");
        }
      } else {
        BatchDialog.open("USB-Export", fileIds.length);
        try {
          await BatchService.exportUsb(fileIds, targetPath);
          ToastContainer.show("success", `${fileIds.length} Dateien auf ${devices[0].name} exportiert`);
        } catch {
          ToastContainer.show("error", "Export fehlgeschlagen");
        }
      }
    }),

    EventBus.on("file:updated", async () => {
      await reloadFiles();
      EventBus.emit("file:refresh");
    }),

    // Filesystem watcher events
    EventBus.on("fs:new-files", async (payload) => {
      const data = payload as { paths: string[] };
      try {
        const imported = await invoke<number>("watcher_auto_import", { filePaths: data.paths });
        if (imported > 0) {
          ToastContainer.show("info", `${imported} neue Datei(en) importiert`);
          await reloadFilesAndCounts();
        }
      } catch (e) {
        console.warn("Watcher auto-import failed:", e);
      }
    }),

    EventBus.on("fs:files-removed", async (payload) => {
      const data = payload as { paths: string[] };
      try {
        const removed = await invoke<number>("watcher_remove_by_paths", { filePaths: data.paths });
        if (removed > 0) {
          ToastContainer.show("info", `${removed} Datei(en) entfernt`);
          await reloadFilesAndCounts();
        }
      } catch (e) {
        console.warn("Watcher remove failed:", e);
      }
    }),

    // Keyboard shortcut handlers
    EventBus.on("shortcut:save", () => {
      EventBus.emit("metadata:save");
    }),

    EventBus.on("shortcut:search", () => {
      const input = document.querySelector<HTMLInputElement>(".search-bar-input");
      if (input) input.focus();
    }),

    EventBus.on("shortcut:settings", () => {
      SettingsDialog.open();
    }),

    EventBus.on("shortcut:delete", () => deleteSelectedFiles()),
    EventBus.on("toolbar:delete-file", () => deleteSelectedFiles()),

    EventBus.on("shortcut:prev-file", () => {
      navigateFile(-1);
    }),

    EventBus.on("shortcut:next-file", () => {
      navigateFile(1);
    }),

    EventBus.on("shortcut:escape", () => {
      // Close burger menu if open
      const burgerMenu = document.querySelector(".burger-menu");
      if (burgerMenu) {
        EventBus.emit("burger:close");
        return;
      }
      // Close any open dialog via its own close method (to revert live previews)
      if (SettingsDialog.isOpen()) {
        SettingsDialog.dismiss();
        return;
      }
      // Dispatch dismiss event so dialogs can clean up properly
      const overlay = document.querySelector(".dialog-overlay");
      if (overlay) {
        overlay.dispatchEvent(new CustomEvent("dialog-dismiss"));
        return;
      }
      // Clear selection
      appState.set("selectedFileIds", []);
      appState.set("selectedFileId", null);
    }),
  ];
  return () => unsubs.forEach((fn) => fn());
}

let reloadGeneration = 0;

async function reloadFiles(): Promise<void> {
  const gen = ++reloadGeneration;
  const folderId = appState.get("selectedFolderId");
  const search = appState.get("searchQuery");
  const formatFilter = appState.get("formatFilter");
  const searchParams = appState.get("searchParams");
  const updatedFiles = await FileService.getFiles(folderId, search, formatFilter, searchParams);
  if (gen !== reloadGeneration) return; // Discard stale results
  appState.set("files", updatedFiles);
}

async function reloadFilesAndCounts(): Promise<void> {
  await reloadFiles();
  try {
    const folders = await FolderService.getAll();
    appState.set("folders", folders);
  } catch {
    // Non-critical: sidebar counts may be stale
  }
}

function navigateFile(direction: number): void {
  const files = appState.getRef("files");
  if (files.length === 0) return;

  const currentId = appState.get("selectedFileId");
  const currentIndex = currentId !== null
    ? files.findIndex((f) => f.id === currentId)
    : -1;

  let newIndex: number;
  if (currentIndex === -1) {
    newIndex = direction > 0 ? 0 : files.length - 1;
  } else {
    newIndex = currentIndex + direction;
    if (newIndex < 0) newIndex = 0;
    if (newIndex >= files.length) newIndex = files.length - 1;
  }

  appState.set("selectedFileIds", []);
  appState.set("selectedFileId", files[newIndex].id);
  EventBus.emit("filelist:scroll-to-index", newIndex);
}

interface AppInstances {
  components: Component[];
  splitters: Splitter[];
  toast: ToastContainer;
}

function initComponents(): AppInstances {
  const components: Component[] = [];

  const sidebarEl = document.querySelector<HTMLElement>(".app-sidebar");
  if (sidebarEl) {
    components.push(new Sidebar(sidebarEl));
  }

  const menuEl = document.querySelector<HTMLElement>(".app-menu");
  if (menuEl) {
    const titleEl = menuEl.querySelector(".app-title");

    // Burger menu before the title
    const burgerContainer = document.createElement("div");
    burgerContainer.className = "burger-container";
    if (titleEl) {
      menuEl.insertBefore(burgerContainer, titleEl);
    } else {
      menuEl.prepend(burgerContainer);
    }
    components.push(new Toolbar(burgerContainer));

    // Search bar in the menu
    const searchContainer = document.createElement("div");
    searchContainer.className = "toolbar-search";
    menuEl.appendChild(searchContainer);
    components.push(new SearchBar(searchContainer));

    // Format filter chips in the menu
    const filterContainer = document.createElement("div");
    filterContainer.className = "toolbar-filters";
    menuEl.appendChild(filterContainer);
    components.push(new FilterChips(filterContainer));
  }

  const centerEl = document.querySelector<HTMLElement>(".app-center");
  if (centerEl) {
    // Dashboard and FileList are siblings in the center panel
    const dashboardEl = document.createElement("div");
    const fileListEl = document.createElement("div");
    centerEl.appendChild(dashboardEl);
    centerEl.appendChild(fileListEl);
    components.push(new Dashboard(dashboardEl));
    components.push(new FileList(fileListEl));
  }

  const rightEl = document.querySelector<HTMLElement>(".app-right");
  if (rightEl) {
    components.push(new MetadataPanel(rightEl));
  }

  const statusEl = document.querySelector<HTMLElement>(".app-status");
  if (statusEl) {
    components.push(new StatusBar(statusEl));
  }

  const splitters: Splitter[] = [];
  const splitterL = document.querySelector<HTMLElement>(".app-splitter-l");
  if (splitterL) {
    splitters.push(new Splitter(splitterL, "--sidebar-width", 180, 400, 240));
  }

  const splitterR = document.querySelector<HTMLElement>(".app-splitter-r");
  if (splitterR) {
    splitters.push(new Splitter(splitterR, "--center-width", 300, 800, 480));
  }

  const toast = new ToastContainer();

  return { components, splitters, toast };
}

// --- Drag-and-drop file import ---
function setupDragDrop(): () => void {
  let overlay: HTMLElement | null = null;

  function showOverlay() {
    if (overlay) return;
    overlay = document.createElement("div");
    overlay.className = "drop-zone-overlay";
    overlay.innerHTML = '<div class="drop-zone-content"><div class="drop-zone-icon">\uD83D\uDCC1</div><div class="drop-zone-text">Dateien hier ablegen zum Importieren</div></div>';
    document.body.appendChild(overlay);
  }

  function hideOverlay() {
    if (overlay) {
      overlay.remove();
      overlay = null;
    }
  }

  async function handleDrop(paths: string[]) {
    hideOverlay();
    if (paths.length === 0) return;

    const folderId = appState.get("selectedFolderId");
    if (!folderId) {
      ToastContainer.show("error", "Bitte zuerst einen Ordner auswählen");
      return;
    }

    try {
      const result = await ScannerService.importFiles(paths, folderId);
      ToastContainer.show("success", `${result.length} Datei(en) importiert`);
      await reloadFilesAndCounts();
    } catch (e) {
      console.warn("Drop import failed:", e);
      ToastContainer.show("error", "Import fehlgeschlagen");
    }
  }

  let unlisten: (() => void) | null = null;

  getCurrentWebviewWindow()
    .onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        showOverlay();
      } else if (event.payload.type === "drop") {
        handleDrop(event.payload.paths);
      } else {
        hideOverlay();
      }
    })
    .then((fn) => { unlisten = fn; });

  return () => {
    hideOverlay();
    if (unlisten) unlisten();
  };
}

let hmrCleanup: (() => void)[] = [];
let initGeneration = 0;

async function init(): Promise<void> {
  const generation = ++initGeneration;

  const hmrData = import.meta.hot?.data as Record<string, unknown> | undefined;
  if (!hmrData?.["stitch_main.themeInitialized"]) {
    await initTheme();
    if (generation !== initGeneration) return;
    if (hmrData) hmrData["stitch_main.themeInitialized"] = true;
  }
  destroyTauriBridge();
  await initTauriBridge();
  if (generation !== initGeneration) return;

  // Seed initial USB device state
  try {
    const usbDevices = await invoke<UsbDevice[]>("get_usb_devices");
    appState.set("usbDevices", usbDevices);
  } catch {
    // USB detection not available — ignore
  }

  // Auto-purge trash on startup (background, non-blocking)
  BackupService.autoPurgeTrash().catch(() => {});

  const destroyThemeToggle = setupThemeToggle();
  const destroyShortcuts = initShortcuts();
  const destroyEventHandlers = initEventHandlers();
  const destroyDragDrop = setupDragDrop();
  const { components, splitters, toast } = initComponents();

  hmrCleanup = [
    destroyTauriBridge,
    destroyEventHandlers,
    destroyShortcuts,
    destroyThemeToggle,
    destroyDragDrop,
    () => components.forEach((c) => c.destroy()),
    () => splitters.forEach((s) => s.destroy()),
    () => toast.destroy(),
  ];
}

init();

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    initGeneration++;
    destroyTauriBridge();
    hmrCleanup.forEach((fn) => fn());
    hmrCleanup = [];
  });
}
