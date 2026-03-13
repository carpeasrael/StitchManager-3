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
import type { ThemeMode } from "./types/index";

async function initTheme(): Promise<void> {
  try {
    const settings = await SettingsService.getAllSettings();
    const theme: ThemeMode =
      settings.theme_mode === "dunkel" ? "dunkel" : "hell";
    applyTheme(theme);

    // Apply persisted font size
    const fontSize = settings.font_size || "medium";
    applyFontSize(fontSize);
  } catch (e) {
    console.warn("Failed to load theme from DB, using default:", e);
    applyTheme("hell");
    applyFontSize("medium");
  }
}

function applyFontSize(size: string): void {
  const map: Record<string, string> = {
    small: "12px",
    medium: "13px",
    large: "15px",
  };
  document.documentElement.style.setProperty(
    "--font-size-body",
    map[size] || map.medium
  );
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
    listen("ai:start", (e) => EventBus.emit("ai:start", e.payload)),
    listen("ai:complete", (e) => EventBus.emit("ai:complete", e.payload)),
    listen("ai:error", (e) => EventBus.emit("ai:error", e.payload)),
    listen("batch:progress", (e) =>
      EventBus.emit("batch:progress", e.payload)
    ),
    listen("import:discovery", (e) =>
      EventBus.emit("import:discovery", e.payload)
    ),
    listen("import:progress", (e) =>
      EventBus.emit("import:progress", e.payload)
    ),
    listen("fs:new-files", (e) =>
      EventBus.emit("fs:new-files", e.payload)
    ),
    listen("fs:files-removed", (e) =>
      EventBus.emit("fs:files-removed", e.payload)
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
  btn.textContent = "\u25D0";
  btn.title = "Theme wechseln";
  btn.style.cssText =
    "margin-left:auto;background:none;border:1px solid var(--color-border);border-radius:var(--radius-button);padding:2px 8px;cursor:pointer;color:var(--color-text);font-size:var(--font-size-body);";
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

    EventBus.on("toolbar:mass-import", async () => {
      // Guard against concurrent imports
      const importBtn = document.querySelector<HTMLButtonElement>(".toolbar-btn-mass-import");
      if (importBtn?.disabled) return;

      const selected = await open({
        directory: true,
        multiple: false,
        title: "Ordner f\u00FCr Massenimport w\u00E4hlen",
      });
      if (!selected) return;

      const path = typeof selected === "string" ? selected : String(selected);
      if (!path) return;

      if (importBtn) importBtn.disabled = true;
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
      } finally {
        if (importBtn) importBtn.disabled = false;
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

  const toolbarEl = document.querySelector<HTMLElement>(".app-toolbar");
  if (toolbarEl) {
    toolbarEl.innerHTML = "";
    const searchContainer = document.createElement("div");
    searchContainer.className = "toolbar-search";
    toolbarEl.appendChild(searchContainer);
    components.push(new SearchBar(searchContainer));

    const filterContainer = document.createElement("div");
    filterContainer.className = "toolbar-filters";
    toolbarEl.appendChild(filterContainer);
    components.push(new FilterChips(filterContainer));

    const actionsContainer = document.createElement("div");
    actionsContainer.className = "toolbar-actions-container";
    toolbarEl.appendChild(actionsContainer);
    components.push(new Toolbar(actionsContainer));
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
