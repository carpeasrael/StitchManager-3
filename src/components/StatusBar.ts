import { invoke } from "@tauri-apps/api/core";
import { Component } from "./Component";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { formatSize } from "../utils/format";

export class StatusBar extends Component {
  private lastAction = "Bereit";
  private watcherActive = true;

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(appState.on("files", () => this.render()));
    this.subscribe(appState.on("folders", () => this.render()));
    this.subscribe(appState.on("selectedFolderId", () => this.render()));
    this.subscribe(appState.on("usbDevices", () => this.render()));
    this.subscribe(
      EventBus.on("scan:complete", (data) => {
        const payload = data as { foundFiles?: number } | undefined;
        const count = payload?.foundFiles ?? 0;
        this.lastAction = `Scan abgeschlossen: ${count} Dateien gefunden`;
        this.render();
      })
    );
    this.subscribe(
      EventBus.on("file:saved", () => {
        this.lastAction = "Gespeichert";
        this.render();
      })
    );
    this.subscribe(
      EventBus.on("watcher:status", (data) => {
        const payload = data as { active: boolean; error?: string } | undefined;
        this.watcherActive = payload?.active ?? true;
        if (!this.watcherActive) {
          this.lastAction = "Automatischer Import deaktiviert";
        }
        this.render();
      })
    );
    this.render();
    this.queryWatcherStatus();
  }

  private async queryWatcherStatus(): Promise<void> {
    try {
      const active = await invoke<boolean>("watcher_get_status");
      if (!active && this.watcherActive) {
        this.watcherActive = false;
        this.lastAction = "Automatischer Import deaktiviert";
        this.render();
      }
    } catch {
      // Watcher status unavailable — keep default
    }
  }

  render(): void {
    this.el.innerHTML = "";

    const left = document.createElement("span");
    left.className = "status-left";

    const folders = appState.get("folders");
    const selectedId = appState.get("selectedFolderId");
    const folder = folders.find((f) => f.id === selectedId);
    left.textContent = folder ? folder.name : "Alle Ordner";

    this.el.appendChild(left);

    const center = document.createElement("span");
    center.className = "status-center";

    const files = appState.getRef("files");
    if (files.length > 0) {
      const formatCounts = new Map<string, number>();
      for (const file of files) {
        const ext = file.filename.split(".").pop()?.toUpperCase() || "?";
        formatCounts.set(ext, (formatCounts.get(ext) || 0) + 1);
      }

      const parts = [`${files.length} Dateien`];
      const formatParts: string[] = [];
      for (const [fmt, count] of Array.from(formatCounts.entries()).sort()) {
        formatParts.push(`${count} ${fmt}`);
      }
      if (formatParts.length > 0) {
        parts.push(formatParts.join(", "));
      }

      center.textContent = parts.join(" \u2014 ");
    } else {
      center.textContent = "Keine Dateien";
    }

    this.el.appendChild(center);

    if (!this.watcherActive) {
      const watcherIndicator = document.createElement("span");
      watcherIndicator.className = "status-watcher-inactive";
      watcherIndicator.textContent = "Watcher inaktiv";
      watcherIndicator.title = "Automatischer Import deaktiviert";
      this.el.appendChild(watcherIndicator);
    }

    // USB device indicator
    const usbDevices = appState.get("usbDevices");
    if (usbDevices.length > 0) {
      const usbIndicator = document.createElement("span");
      usbIndicator.className = "status-usb";

      const icon = document.createElement("span");
      icon.className = "status-usb-icon";
      icon.textContent = "\u{1F50C}";
      usbIndicator.appendChild(icon);

      const label = document.createElement("span");
      if (usbDevices.length === 1) {
        const dev = usbDevices[0];
        label.textContent = `${dev.name} ${formatSize(dev.freeSpaceBytes)} frei`;
      } else {
        label.textContent = `${usbDevices.length} USB-Geraete`;
        usbIndicator.title = usbDevices
          .map((d) => `${d.name}: ${formatSize(d.freeSpaceBytes)} frei`)
          .join("\n");
      }
      usbIndicator.appendChild(label);

      usbIndicator.addEventListener("click", () => {
        EventBus.emit("usb:quick-export");
      });

      this.el.appendChild(usbIndicator);
    }

    const right = document.createElement("span");
    right.className = "status-right";
    right.textContent = this.lastAction;
    this.el.appendChild(right);

    const version = document.createElement("span");
    version.className = "status-version";
    version.textContent = "v26.4.1";
    this.el.appendChild(version);
  }
}
