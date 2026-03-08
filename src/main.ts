import "./styles.css";
import { appState } from "./state/AppState";
import { EventBus } from "./state/EventBus";
import { Sidebar } from "./components/Sidebar";
import { listen } from "@tauri-apps/api/event";
import Database from "@tauri-apps/plugin-sql";
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
    listen("ai:complete", (e) => EventBus.emit("ai:complete", e.payload)),
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

function initComponents(): void {
  const sidebarEl = document.querySelector<HTMLElement>(".app-sidebar");
  if (sidebarEl) {
    new Sidebar(sidebarEl);
  }
}

async function init(): Promise<void> {
  await initTheme();
  await initTauriBridge();
  setupThemeToggle();
  initComponents();
}

init();
