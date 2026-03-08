import { Component } from "./Component";
import { appState } from "../state/AppState";

export class SearchBar extends Component {
  private input!: HTMLInputElement;
  private clearBtn!: HTMLButtonElement;
  private debounceTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(container: HTMLElement) {
    super(container);
    this.render();
  }

  render(): void {
    this.el.innerHTML = "";

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
    this.clearBtn.addEventListener("click", () => this.clear());
    this.updateClearVisibility();
    wrapper.appendChild(this.clearBtn);

    this.el.appendChild(wrapper);
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

  destroy(): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }
    super.destroy();
  }
}
