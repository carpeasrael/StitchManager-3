import { Component } from "./Component";
import { appState } from "../state/AppState";
import * as SettingsService from "../services/SettingsService";

const SORT_OPTIONS = [
  { value: "filename", label: "Dateiname" },
  { value: "name", label: "Name" },
  { value: "created_at", label: "Hinzugefuegt" },
  { value: "updated_at", label: "Geaendert" },
  { value: "author", label: "Designer" },
  { value: "category", label: "Kategorie" },
  { value: "stitch_count", label: "Stichanzahl" },
  { value: "color_count", label: "Farbanzahl" },
  { value: "file_type", label: "Dateityp" },
  { value: "status", label: "Status" },
];

export class SortControl extends Component {
  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("searchParams", () => this.render())
    );
    this.render();
  }

  render(): void {
    const sp = appState.get("searchParams");
    const currentField = sp.sortField || "filename";
    const currentDir = sp.sortDirection || "asc";

    this.el.innerHTML = "";

    const wrapper = document.createElement("div");
    wrapper.className = "sort-control";
    wrapper.setAttribute("role", "toolbar");
    wrapper.setAttribute("aria-label", "Sortierung");

    const select = document.createElement("select");
    select.className = "sort-select";
    select.title = "Sortierung";
    select.setAttribute("aria-label", "Sortierfeld");

    for (const opt of SORT_OPTIONS) {
      const o = document.createElement("option");
      o.value = opt.value;
      o.textContent = opt.label;
      if (opt.value === currentField) o.selected = true;
      select.appendChild(o);
    }

    select.addEventListener("change", () => {
      const updated = { ...appState.get("searchParams") };
      updated.sortField = select.value;
      appState.set("searchParams", updated);
      this.persistSort(select.value, currentDir);
    });
    wrapper.appendChild(select);

    const dirBtn = document.createElement("button");
    dirBtn.className = "sort-dir-btn";
    dirBtn.textContent = currentDir === "desc" ? "\u2193" : "\u2191";
    dirBtn.title = currentDir === "desc" ? "Absteigend" : "Aufsteigend";
    dirBtn.setAttribute("aria-label", currentDir === "desc" ? "Absteigend" : "Aufsteigend");
    dirBtn.addEventListener("click", () => {
      const updated = { ...appState.get("searchParams") };
      const newDir = updated.sortDirection === "desc" ? "asc" : "desc";
      updated.sortDirection = newDir;
      appState.set("searchParams", updated);
      this.persistSort(updated.sortField || "filename", newDir);
    });
    wrapper.appendChild(dirBtn);

    this.el.appendChild(wrapper);
  }

  private persistSort(field: string, direction: string): void {
    SettingsService.setSetting("file_sort_field", field).catch(() => {});
    SettingsService.setSetting("file_sort_direction", direction).catch(() => {});
  }
}
