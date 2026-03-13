import { Component } from "./Component";
import { appState } from "../state/AppState";
import { ToastContainer } from "./Toast";
import { TagInput } from "./TagInput";
import * as FileService from "../services/FileService";
import type { SearchParams, Tag } from "../types/index";

/** Count how many advanced filter fields are active. */
function activeFilterCount(sp: SearchParams): number {
  let count = 0;
  if (sp.tags && sp.tags.length > 0) count++;
  if (sp.stitchCountMin != null || sp.stitchCountMax != null) count++;
  if (sp.colorCountMin != null || sp.colorCountMax != null) count++;
  if (sp.widthMmMin != null || sp.widthMmMax != null) count++;
  if (sp.heightMmMin != null || sp.heightMmMax != null) count++;
  if (sp.fileSizeMin != null || sp.fileSizeMax != null) count++;
  if (sp.aiAnalyzed != null) count++;
  if (sp.aiConfirmed != null) count++;
  if (sp.colorSearch) count++;
  return count;
}

export class SearchBar extends Component {
  private input!: HTMLInputElement;
  private clearBtn!: HTMLButtonElement;
  private filterToggle!: HTMLButtonElement;
  private filterBadge!: HTMLSpanElement;
  private panelEl: HTMLElement | null = null;
  private panelOpen = false;
  private debounceTimer: ReturnType<typeof setTimeout> | null = null;
  private allTags: Tag[] = [];
  private _panelTagInput: TagInput | null = null;

  constructor(container: HTMLElement) {
    super(container);
    this.render();
    this.subscribe(
      appState.on("searchParams", () => this.updateBadge())
    );
  }

  render(): void {
    this.el.innerHTML = "";

    const row = document.createElement("div");
    row.className = "search-bar-row";

    const wrapper = document.createElement("div");
    wrapper.className = "search-bar";

    const icon = document.createElement("span");
    icon.className = "search-bar-icon";
    icon.textContent = "\u{1F50D}";
    wrapper.appendChild(icon);

    this.input = document.createElement("input");
    this.input.type = "text";
    this.input.className = "search-bar-input";
    this.input.placeholder = "Suchen\u2026";
    this.input.value = appState.get("searchQuery");
    this.input.addEventListener("input", () => this.onInput());
    wrapper.appendChild(this.input);

    this.clearBtn = document.createElement("button");
    this.clearBtn.className = "search-bar-clear";
    this.clearBtn.textContent = "\u00D7";
    this.clearBtn.title = "Suche leeren";
    this.clearBtn.setAttribute("aria-label", "Suche leeren");
    this.clearBtn.addEventListener("click", () => this.clear());
    this.updateClearVisibility();
    wrapper.appendChild(this.clearBtn);

    row.appendChild(wrapper);

    // Filter toggle button
    this.filterToggle = document.createElement("button");
    this.filterToggle.className = "search-filter-toggle";
    this.filterToggle.title = "Erweiterte Filter";
    this.filterToggle.setAttribute("aria-label", "Erweiterte Filter");
    this.filterToggle.innerHTML = `<span class="search-filter-toggle-icon">\u2699</span>`;
    this.filterBadge = document.createElement("span");
    this.filterBadge.className = "search-filter-badge";
    this.filterBadge.style.display = "none";
    this.filterToggle.appendChild(this.filterBadge);
    this.filterToggle.addEventListener("click", () => this.togglePanel());
    row.appendChild(this.filterToggle);

    this.el.appendChild(row);

    this.updateBadge();
  }

  private onInput(): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }
    this.debounceTimer = setTimeout(() => {
      appState.set("searchQuery", this.input.value);
    }, 300);
    this.updateClearVisibility();
  }

  private clear(): void {
    this.input.value = "";
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }
    appState.set("searchQuery", "");
    this.updateClearVisibility();
    this.input.focus();
  }

  private updateClearVisibility(): void {
    if (this.clearBtn) {
      this.clearBtn.style.display = this.input.value ? "" : "none";
    }
  }

  private updateBadge(): void {
    const sp = appState.get("searchParams");
    const count = activeFilterCount(sp);
    if (count > 0) {
      this.filterBadge.textContent = String(count);
      this.filterBadge.style.display = "";
      this.filterToggle.classList.add("active");
    } else {
      this.filterBadge.style.display = "none";
      this.filterToggle.classList.remove("active");
    }
  }

  private async togglePanel(): Promise<void> {
    this.panelOpen = !this.panelOpen;
    if (this.panelOpen) {
      // Load all tags for autocomplete
      try {
        this.allTags = await FileService.getAllTags();
      } catch (e) {
        console.warn("Failed to load tags:", e);
        this.allTags = [];
        ToastContainer.show("error", "Tags konnten nicht geladen werden");
      }
      this.renderPanel();
    } else {
      this.closePanel();
    }
  }

  private closePanel(): void {
    this.panelOpen = false;
    if (this._panelTagInput) {
      this._panelTagInput.destroy();
      this._panelTagInput = null;
    }
    if (this.panelEl) {
      this.panelEl.remove();
      this.panelEl = null;
    }
  }

  private renderPanel(): void {
    if (this._panelTagInput) {
      this._panelTagInput.destroy();
      this._panelTagInput = null;
    }
    if (this.panelEl) {
      this.panelEl.remove();
    }

    const sp = appState.get("searchParams");

    this.panelEl = document.createElement("div");
    this.panelEl.className = "search-advanced-panel";

    // Header row
    const header = document.createElement("div");
    header.className = "search-advanced-header";
    const title = document.createElement("span");
    title.className = "search-advanced-title";
    title.textContent = "Erweiterte Filter";
    header.appendChild(title);

    const resetBtn = document.createElement("button");
    resetBtn.className = "search-advanced-reset";
    resetBtn.textContent = "Alle zur\u00FCcksetzen";
    resetBtn.addEventListener("click", () => {
      appState.set("searchParams", {});
      this.renderPanel();
    });
    header.appendChild(resetBtn);
    this.panelEl.appendChild(header);

    const grid = document.createElement("div");
    grid.className = "search-advanced-grid";

    // --- Tags ---
    grid.appendChild(this.buildTagFilter(sp));

    // --- Numeric ranges ---
    grid.appendChild(this.buildRangeRow(
      "Stiche", "stitchCountMin", "stitchCountMax", sp, true
    ));
    grid.appendChild(this.buildRangeRow(
      "Farben", "colorCountMin", "colorCountMax", sp, true
    ));
    grid.appendChild(this.buildRangeRow(
      "Breite (mm)", "widthMmMin", "widthMmMax", sp, false
    ));
    grid.appendChild(this.buildRangeRow(
      "H\u00F6he (mm)", "heightMmMin", "heightMmMax", sp, false
    ));
    grid.appendChild(this.buildRangeRow(
      "Dateigr\u00F6\u00DFe (KB)", "fileSizeMin", "fileSizeMax", sp, true, 1024
    ));

    // --- Boolean filters ---
    grid.appendChild(this.buildBoolFilter(sp));

    // --- Color/brand search ---
    grid.appendChild(this.buildColorSearch(sp));

    this.panelEl.appendChild(grid);

    // Active filter chips
    const chips = this.buildActiveChips(sp);
    if (chips) {
      this.panelEl.appendChild(chips);
    }

    this.el.appendChild(this.panelEl);
  }

  private buildTagFilter(sp: SearchParams): HTMLElement {
    const group = document.createElement("div");
    group.className = "search-advanced-group";

    const label = document.createElement("label");
    label.className = "search-advanced-label";
    label.textContent = "Tags";
    group.appendChild(label);

    const tagContainer = document.createElement("div");
    group.appendChild(tagContainer);

    const tagInput = new TagInput(tagContainer, {
      allTags: this.allTags.map((t) => t.name),
      selectedTags: sp.tags || [],
      placeholder: "Tag hinzufügen\u2026",
      onChange: (tags) => {
        const updated = { ...appState.get("searchParams") };
        if (tags.length > 0) {
          updated.tags = tags;
        } else {
          delete updated.tags;
        }
        appState.set("searchParams", updated);
        this.updateBadge();
      },
    });

    // Store reference for cleanup
    if (!this._panelTagInput) {
      this._panelTagInput = tagInput;
    }

    return group;
  }

  private buildRangeRow(
    label: string,
    minKey: keyof SearchParams,
    maxKey: keyof SearchParams,
    sp: SearchParams,
    integer: boolean,
    displayDivisor = 1
  ): HTMLElement {
    const group = document.createElement("div");
    group.className = "search-advanced-group";

    const lbl = document.createElement("label");
    lbl.className = "search-advanced-label";
    lbl.textContent = label;
    group.appendChild(lbl);

    const row = document.createElement("div");
    row.className = "search-range-row";

    const minInput = document.createElement("input");
    minInput.type = "number";
    minInput.min = "0";
    minInput.className = "search-range-input";
    minInput.placeholder = "Min";
    const rawMin = sp[minKey] as number | undefined;
    if (rawMin != null) {
      minInput.value = String(displayDivisor > 1 ? Math.round(rawMin / displayDivisor) : rawMin);
    }
    const applyRange = () => {
      const updated = { ...appState.get("searchParams") };
      const minVal = minInput.value.trim();
      const maxVal = maxInput.value.trim();
      let minNum: number | undefined;
      let maxNum: number | undefined;

      if (minVal) {
        const n = integer ? parseInt(minVal, 10) : parseFloat(minVal);
        if (!isNaN(n)) minNum = displayDivisor > 1 ? n * displayDivisor : n;
      }
      if (maxVal) {
        const n = integer ? parseInt(maxVal, 10) : parseFloat(maxVal);
        if (!isNaN(n)) maxNum = displayDivisor > 1 ? n * displayDivisor : n;
      }

      // Auto-swap if min > max
      if (minNum != null && maxNum != null && minNum > maxNum) {
        [minNum, maxNum] = [maxNum, minNum];
        minInput.value = String(displayDivisor > 1 ? Math.round(minNum / displayDivisor) : minNum);
        maxInput.value = String(displayDivisor > 1 ? Math.round(maxNum / displayDivisor) : maxNum);
      }

      if (minNum != null) {
        (updated as Record<string, unknown>)[minKey] = minNum;
      } else {
        delete (updated as Record<string, unknown>)[minKey];
      }
      if (maxNum != null) {
        (updated as Record<string, unknown>)[maxKey] = maxNum;
      } else {
        delete (updated as Record<string, unknown>)[maxKey];
      }
      appState.set("searchParams", updated);
    };

    minInput.addEventListener("change", applyRange);
    row.appendChild(minInput);

    const sep = document.createElement("span");
    sep.className = "search-range-sep";
    sep.textContent = "\u2013";
    row.appendChild(sep);

    const maxInput = document.createElement("input");
    maxInput.type = "number";
    maxInput.min = "0";
    maxInput.className = "search-range-input";
    maxInput.placeholder = "Max";
    const rawMax = sp[maxKey] as number | undefined;
    if (rawMax != null) {
      maxInput.value = String(displayDivisor > 1 ? Math.round(rawMax / displayDivisor) : rawMax);
    }
    maxInput.addEventListener("change", applyRange);
    row.appendChild(maxInput);

    group.appendChild(row);
    return group;
  }

  private buildBoolFilter(sp: SearchParams): HTMLElement {
    const group = document.createElement("div");
    group.className = "search-advanced-group";

    const lbl = document.createElement("label");
    lbl.className = "search-advanced-label";
    lbl.textContent = "Status";
    group.appendChild(lbl);

    const row = document.createElement("div");
    row.className = "search-bool-row";

    row.appendChild(this.buildCheckbox("KI-analysiert", "aiAnalyzed", sp));
    row.appendChild(this.buildCheckbox("KI-best\u00E4tigt", "aiConfirmed", sp));

    group.appendChild(row);
    return group;
  }

  private buildCheckbox(label: string, key: keyof SearchParams, sp: SearchParams): HTMLElement {
    const wrap = document.createElement("label");
    wrap.className = "search-bool-label";

    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.className = "search-bool-checkbox";
    cb.checked = sp[key] === true;
    cb.indeterminate = sp[key] == null;

    cb.addEventListener("click", () => {
      const updated = { ...appState.get("searchParams") };
      const current = updated[key] as boolean | undefined;
      if (current == null) {
        // null -> true
        (updated as Record<string, unknown>)[key] = true;
      } else if (current === true) {
        // true -> false
        (updated as Record<string, unknown>)[key] = false;
      } else {
        // false -> null (unset)
        delete (updated as Record<string, unknown>)[key];
      }
      appState.set("searchParams", updated);
      // Update UI
      const newVal = appState.get("searchParams")[key] as boolean | undefined;
      cb.checked = newVal === true;
      cb.indeterminate = newVal == null;
    });

    wrap.appendChild(cb);
    const text = document.createElement("span");
    text.textContent = label;
    wrap.appendChild(text);
    return wrap;
  }

  private buildColorSearch(sp: SearchParams): HTMLElement {
    const group = document.createElement("div");
    group.className = "search-advanced-group";

    const lbl = document.createElement("label");
    lbl.className = "search-advanced-label";
    lbl.textContent = "Farbe / Marke";
    group.appendChild(lbl);

    const input = document.createElement("input");
    input.type = "text";
    input.className = "search-color-input";
    input.placeholder = "z.B. Rot, Madeira\u2026";
    input.value = sp.colorSearch || "";
    input.addEventListener("change", () => {
      const updated = { ...appState.get("searchParams") };
      const val = input.value.trim();
      if (val) {
        updated.colorSearch = val;
      } else {
        delete updated.colorSearch;
      }
      appState.set("searchParams", updated);
    });
    group.appendChild(input);
    return group;
  }

  private buildActiveChips(sp: SearchParams): HTMLElement | null {
    const count = activeFilterCount(sp);
    if (count === 0) return null;

    const container = document.createElement("div");
    container.className = "search-active-chips";

    const labels: { label: string; clearFn: () => void }[] = [];

    if (sp.tags && sp.tags.length > 0) {
      labels.push({
        label: `Tags: ${sp.tags.join(", ")}`,
        clearFn: () => {
          const u = { ...appState.get("searchParams") };
          delete u.tags;
          appState.set("searchParams", u);
          this.renderPanel();
        },
      });
    }
    if (sp.stitchCountMin != null || sp.stitchCountMax != null) {
      labels.push({
        label: `Stiche: ${sp.stitchCountMin ?? "*"}\u2013${sp.stitchCountMax ?? "*"}`,
        clearFn: () => {
          const u = { ...appState.get("searchParams") };
          delete u.stitchCountMin;
          delete u.stitchCountMax;
          appState.set("searchParams", u);
          this.renderPanel();
        },
      });
    }
    if (sp.colorCountMin != null || sp.colorCountMax != null) {
      labels.push({
        label: `Farben: ${sp.colorCountMin ?? "*"}\u2013${sp.colorCountMax ?? "*"}`,
        clearFn: () => {
          const u = { ...appState.get("searchParams") };
          delete u.colorCountMin;
          delete u.colorCountMax;
          appState.set("searchParams", u);
          this.renderPanel();
        },
      });
    }
    if (sp.widthMmMin != null || sp.widthMmMax != null) {
      labels.push({
        label: `Breite: ${sp.widthMmMin ?? "*"}\u2013${sp.widthMmMax ?? "*"} mm`,
        clearFn: () => {
          const u = { ...appState.get("searchParams") };
          delete u.widthMmMin;
          delete u.widthMmMax;
          appState.set("searchParams", u);
          this.renderPanel();
        },
      });
    }
    if (sp.heightMmMin != null || sp.heightMmMax != null) {
      labels.push({
        label: `H\u00F6he: ${sp.heightMmMin ?? "*"}\u2013${sp.heightMmMax ?? "*"} mm`,
        clearFn: () => {
          const u = { ...appState.get("searchParams") };
          delete u.heightMmMin;
          delete u.heightMmMax;
          appState.set("searchParams", u);
          this.renderPanel();
        },
      });
    }
    if (sp.fileSizeMin != null || sp.fileSizeMax != null) {
      const minKb = sp.fileSizeMin != null ? Math.round(sp.fileSizeMin / 1024) : "*";
      const maxKb = sp.fileSizeMax != null ? Math.round(sp.fileSizeMax / 1024) : "*";
      labels.push({
        label: `Gr\u00F6\u00DFe: ${minKb}\u2013${maxKb} KB`,
        clearFn: () => {
          const u = { ...appState.get("searchParams") };
          delete u.fileSizeMin;
          delete u.fileSizeMax;
          appState.set("searchParams", u);
          this.renderPanel();
        },
      });
    }
    if (sp.aiAnalyzed != null) {
      labels.push({
        label: `KI-analysiert: ${sp.aiAnalyzed ? "Ja" : "Nein"}`,
        clearFn: () => {
          const u = { ...appState.get("searchParams") };
          delete u.aiAnalyzed;
          appState.set("searchParams", u);
          this.renderPanel();
        },
      });
    }
    if (sp.aiConfirmed != null) {
      labels.push({
        label: `KI-best\u00E4tigt: ${sp.aiConfirmed ? "Ja" : "Nein"}`,
        clearFn: () => {
          const u = { ...appState.get("searchParams") };
          delete u.aiConfirmed;
          appState.set("searchParams", u);
          this.renderPanel();
        },
      });
    }
    if (sp.colorSearch) {
      labels.push({
        label: `Farbe: ${sp.colorSearch}`,
        clearFn: () => {
          const u = { ...appState.get("searchParams") };
          delete u.colorSearch;
          appState.set("searchParams", u);
          this.renderPanel();
        },
      });
    }

    for (const item of labels) {
      const chip = document.createElement("span");
      chip.className = "search-active-chip";
      chip.textContent = item.label;
      const x = document.createElement("button");
      x.className = "search-active-chip-remove";
      x.textContent = "\u00D7";
      x.addEventListener("click", item.clearFn);
      chip.appendChild(x);
      container.appendChild(chip);
    }

    return container;
  }

  destroy(): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }
    this.closePanel();
    super.destroy();
  }
}
