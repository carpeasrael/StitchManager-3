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
import { initShortcuts } from "./shortcuts";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import * as FileService from "./services/FileService";
import * as BatchService from "./services/BatchService";
import * as AiService from "./services/AiService";
import * as ScannerService from "./services/ScannerService";
import * as SettingsService from "./services/SettingsService";
import * as FolderService from "./services/FolderService";
import { applyFontSize } from "./utils/theme";
import type { ThemeMode, UsbDevice } from "./types/index";

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

  const files = appState.get("files");
  const file = files.find((f) => f.id === fileId);
  if (!file?.filepath) return;

  try {
    await revealItemInDir(file.filepath);
  } catch (e) {
    console.warn("Failed to reveal file in folder:", e);
    ToastContainer.show("error", "Datei konnte nicht im Ordner angezeigt werden");
  }
}

function initEventHandlers(): () => void {
  const unsubs = [
    EventBus.on("toolbar:ai-analyze", async () => {
      const fileId = appState.get("selectedFileId");
      if (fileId === null) return;

      const files = appState.get("files");
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

    EventBus.on("toolbar:save", () => {
      EventBus.emit("metadata:save");
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
        await BatchService.rename(fileIds, pattern);
      } catch (e) {
        console.warn("Batch rename failed:", e);
      }
      await reloadFiles();
    }),

    EventBus.on("toolbar:batch-organize", async () => {
      const fileIds = appState.get("selectedFileIds");
      if (fileIds.length === 0) return;

      const settings = await SettingsService.getAllSettings();
      const pattern = settings.organize_pattern || "{theme}/{name}";

      BatchDialog.open("Batch Organisieren", fileIds.length);
      try {
        await BatchService.organize(fileIds, pattern);
      } catch (e) {
        console.warn("Batch organize failed:", e);
      }
      await reloadFiles();
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
        await AiService.analyzeBatch(fileIds);
      } catch (e) {
        console.warn("Batch AI analysis failed:", e);
      }
      await reloadFiles();
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
          await reloadFiles();
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
          await reloadFiles();
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

    EventBus.on("shortcut:delete", async () => {
      const fileId = appState.get("selectedFileId");
      if (fileId === null) return;

      const files = appState.get("files");
      const file = files.find((f) => f.id === fileId);
      if (!file) return;

      if (!confirm(`Datei "${file.name || file.filename}" wirklich loeschen?`)) return;

      try {
        await invoke("delete_file", { fileId });
        ToastContainer.show("success", "Datei geloescht");
        await reloadFiles();
      } catch (e) {
        console.warn("Failed to delete file:", e);
        ToastContainer.show("error", "Datei konnte nicht geloescht werden");
      }
    }),

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

async function reloadFiles(): Promise<void> {
  const folderId = appState.get("selectedFolderId");
  const search = appState.get("searchQuery");
  const formatFilter = appState.get("formatFilter");
  const searchParams = appState.get("searchParams");
  const updatedFiles = await FileService.getFiles(folderId, search, formatFilter, searchParams);
  appState.set("files", updatedFiles);
}

function navigateFile(direction: number): void {
  const files = appState.get("files");
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
    components.push(new FileList(centerEl));
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

  const destroyThemeToggle = setupThemeToggle();
  const destroyShortcuts = initShortcuts();
  const destroyEventHandlers = initEventHandlers();
  const { components, splitters, toast } = initComponents();

  hmrCleanup = [
    destroyTauriBridge,
    destroyEventHandlers,
    destroyShortcuts,
    destroyThemeToggle,
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
