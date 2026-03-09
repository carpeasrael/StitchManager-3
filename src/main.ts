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
import { listen } from "@tauri-apps/api/event";
import Database from "@tauri-apps/plugin-sql";
import * as FileService from "./services/FileService";
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
  } catch (e) {
    console.warn("Failed to load theme from DB, using default:", e);
    applyTheme("hell");
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
      // Reload files to reflect AI badge changes
      const folderId = appState.get("selectedFolderId");
      const updatedFiles = await FileService.getFiles(folderId);
      appState.set("files", updatedFiles);
    });
  });

  EventBus.on("toolbar:settings", () => {
    SettingsDialog.open();
  });

  EventBus.on("file:updated", async () => {
    const folderId = appState.get("selectedFolderId");
    const updatedFiles = await FileService.getFiles(folderId);
    appState.set("files", updatedFiles);
    // Signal MetadataPanel to refresh the currently selected file
    EventBus.emit("file:refresh");
  });
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
}

async function init(): Promise<void> {
  await initTheme();
  await initTauriBridge();
  setupThemeToggle();
  initEventHandlers();
  initComponents();
}

init();
