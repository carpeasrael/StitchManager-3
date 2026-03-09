import { Component } from "./Component";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";

export class StatusBar extends Component {
  private lastAction = "Bereit";

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(appState.on("files", () => this.render()));
    this.subscribe(appState.on("selectedFolderId", () => this.render()));
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
    this.render();
  }

  render(): void {
    this.el.innerHTML = "";

    const left = document.createElement("span");
    left.className = "status-left";

    const folders = appState.get("folders");
    const selectedId = appState.get("selectedFolderId");
    const folder = folders.find((f) => f.id === selectedId);
    left.textContent = folder ? folder.name : "Kein Ordner ausgewählt";

    this.el.appendChild(left);

    const center = document.createElement("span");
    center.className = "status-center";

    const files = appState.get("files");
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

    const right = document.createElement("span");
    right.className = "status-right";
    right.textContent = this.lastAction;
    this.el.appendChild(right);
  }
}
