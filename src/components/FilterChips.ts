import { Component } from "./Component";
import { appState } from "../state/AppState";

const FORMATS = ["PES", "DST", "JEF", "VP3"];

export class FilterChips extends Component {
  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("formatFilter", () => this.render())
    );
    this.render();
  }

  render(): void {
    const current = appState.get("formatFilter");
    this.el.innerHTML = "";

    const wrapper = document.createElement("div");
    wrapper.className = "filter-chips";
    wrapper.setAttribute("role", "toolbar");
    wrapper.setAttribute("aria-label", "Formatfilter");

    // "Alle" chip
    const allChip = document.createElement("button");
    allChip.className = "filter-chip";
    allChip.setAttribute("aria-pressed", String(!current));
    if (!current) {
      allChip.classList.add("active");
    }
    allChip.textContent = "Alle";
    allChip.addEventListener("click", () => {
      appState.set("formatFilter", null);
    });
    wrapper.appendChild(allChip);

    for (const fmt of FORMATS) {
      const chip = document.createElement("button");
      chip.className = "filter-chip";
      chip.setAttribute("aria-pressed", String(current === fmt));
      if (current === fmt) {
        chip.classList.add("active");
      }
      chip.textContent = fmt;
      chip.addEventListener("click", () => {
        appState.set("formatFilter", current === fmt ? null : fmt);
      });
      wrapper.appendChild(chip);
    }

    this.el.appendChild(wrapper);
  }
}
