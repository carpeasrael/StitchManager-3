import "./styles.css";
import { appState } from "./state/AppState";
import { EventBus } from "./state/EventBus";
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
import Database from "@tauri-apps/plugin-sql";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import * as FileService from "./services/FileService";
import * as BatchService from "./services/BatchService";
import * as AiService from "./services/AiService";
import * as SettingsService from "./services/SettingsService";
import type { ThemeMode } from "./types/index";

let dbInstance: Awaited<ReturnType<typeof Database.load>> | null = null;

async function getDb(): Promise<Awaited<ReturnType<typeof Database.load>>> {
  if (!dbInstance) {
    dbInstance = await Database.load("sqlite:stitch_manager.db");
  }
  return dbInstance;
}

async function initTheme(): Promise<void> {
  try {
    const db = await getDb();
    const result = await db.select<Array<{ value: string }>>(
      "SELECT value FROM settings WHERE key = 'theme_mode'"
    );
    const theme: ThemeMode =
      result.length > 0 && result[0].value === "dunkel" ? "dunkel" : "hell";
    applyTheme(theme);

    // Apply persisted font size
    const fontResult = await db.select<Array<{ value: string }>>(
      "SELECT value FROM settings WHERE key = 'font_size'"
    );
    const fontSize = fontResult.length > 0 ? fontResult[0].value : "medium";
    applyFontSize(fontSize);
  } catch (e) {
    console.warn("Failed to load theme from DB, using default:", e);
    applyTheme("hell");
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
    const db = await getDb();
    await db.execute(
      "UPDATE settings SET value = $1, updated_at = datetime('now') WHERE key = 'theme_mode'",
      [next]
    );
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

function setupThemeToggle(): void {
  const menuEl = document.querySelector(".app-menu");
  if (!menuEl) return;

  const btn = document.createElement("button");
  btn.textContent = "\u25D0";
  btn.title = "Theme wechseln";
  btn.style.cssText =
    "margin-left:auto;background:none;border:1px solid var(--color-border);border-radius:var(--radius-button);padding:2px 8px;cursor:pointer;color:var(--color-text);font-size:var(--font-size-body);";
  btn.addEventListener("click", () => {
    toggleTheme();
  });
  menuEl.appendChild(btn);
}

function initEventHandlers(): void {
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
  });

  EventBus.on("toolbar:settings", () => {
    SettingsDialog.open();
  });

  EventBus.on("toolbar:save", () => {
    EventBus.emit("metadata:save");
  });

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
  });

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
  });

  EventBus.on("toolbar:batch-export", async () => {
    const fileIds = appState.get("selectedFileIds");
    if (fileIds.length === 0) return;

    const selected = await open({
      directory: true,
      multiple: false,
      title: "Zielordner f\u00FCr USB-Export w\u00E4hlen",
    });
    if (!selected) return;

    const targetPath = typeof selected === "string" ? selected : String(selected);
    if (!targetPath) return;

    BatchDialog.open("USB-Export", fileIds.length);
    try {
      await BatchService.exportUsb(fileIds, targetPath);
    } catch (e) {
      console.warn("Batch export failed:", e);
    }
  });

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
  });

  EventBus.on("file:updated", async () => {
    await reloadFiles();
    EventBus.emit("file:refresh");
  });

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
  });

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
  });

  // Keyboard shortcut handlers
  EventBus.on("shortcut:save", () => {
    EventBus.emit("metadata:save");
  });

  EventBus.on("shortcut:search", () => {
    const input = document.querySelector<HTMLInputElement>(".search-bar-input");
    if (input) input.focus();
  });

  EventBus.on("shortcut:settings", () => {
    SettingsDialog.open();
  });

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
      reloadFiles();
    } catch (e) {
      console.warn("Failed to delete file:", e);
      ToastContainer.show("error", "Datei konnte nicht geloescht werden");
    }
  });

  EventBus.on("shortcut:prev-file", () => {
    navigateFile(-1);
  });

  EventBus.on("shortcut:next-file", () => {
    navigateFile(1);
  });

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
  });
}

async function reloadFiles(): Promise<void> {
  const folderId = appState.get("selectedFolderId");
  const search = appState.get("searchQuery");
  const formatFilter = appState.get("formatFilter");
  const updatedFiles = await FileService.getFiles(folderId, search, formatFilter);
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

function initComponents(): void {
  const sidebarEl = document.querySelector<HTMLElement>(".app-sidebar");
  if (sidebarEl) {
    new Sidebar(sidebarEl);
  }

  const toolbarEl = document.querySelector<HTMLElement>(".app-toolbar");
  if (toolbarEl) {
    toolbarEl.innerHTML = "";
    const searchContainer = document.createElement("div");
    searchContainer.className = "toolbar-search";
    toolbarEl.appendChild(searchContainer);
    new SearchBar(searchContainer);

    const filterContainer = document.createElement("div");
    filterContainer.className = "toolbar-filters";
    toolbarEl.appendChild(filterContainer);
    new FilterChips(filterContainer);

    const actionsContainer = document.createElement("div");
    actionsContainer.className = "toolbar-actions-container";
    toolbarEl.appendChild(actionsContainer);
    new Toolbar(actionsContainer);
  }

  const centerEl = document.querySelector<HTMLElement>(".app-center");
  if (centerEl) {
    new FileList(centerEl);
  }

  const rightEl = document.querySelector<HTMLElement>(".app-right");
  if (rightEl) {
    new MetadataPanel(rightEl);
  }

  const statusEl = document.querySelector<HTMLElement>(".app-status");
  if (statusEl) {
    new StatusBar(statusEl);
  }

  // Initialize splitters
  const splitterL = document.querySelector<HTMLElement>(".app-splitter-l");
  if (splitterL) {
    new Splitter(splitterL, "--sidebar-width", 180, 400, 240);
  }

  const splitterR = document.querySelector<HTMLElement>(".app-splitter-r");
  if (splitterR) {
    new Splitter(splitterR, "--center-width", 300, 800, 480);
  }

  // Initialize toast container
  new ToastContainer();
}

async function init(): Promise<void> {
  await initTheme();
  await initTauriBridge();
  setupThemeToggle();
  initShortcuts();
  initEventHandlers();
  initComponents();
}

init();
