import { Component } from "./Component";

export interface TagInputOptions {
  allTags: string[];
  selectedTags: string[];
  placeholder?: string;
  onChange: (tags: string[]) => void;
}

export class TagInput extends Component {
  private options: TagInputOptions;
  private chipContainer!: HTMLElement;
  private input!: HTMLInputElement;
  private suggestions!: HTMLElement;
  private highlightIndex = -1;
  private blurTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(container: HTMLElement, options: TagInputOptions) {
    super(container);
    this.options = options;
    this.render();
  }

  render(): void {
    this.el.innerHTML = "";

    const editor = document.createElement("div");
    editor.className = "tag-editor";

    this.chipContainer = document.createElement("div");
    this.chipContainer.className = "tag-chip-container";

    for (const tag of this.options.selectedTags) {
      this.addChip(tag);
    }

    editor.appendChild(this.chipContainer);

    const inputWrapper = document.createElement("div");
    inputWrapper.className = "tag-input-wrapper";

    this.input = document.createElement("input");
    this.input.type = "text";
    this.input.className = "tag-input";
    this.input.placeholder = this.options.placeholder || "Tag hinzufügen...";

    this.suggestions = document.createElement("div");
    this.suggestions.className = "tag-suggestions";
    this.suggestions.style.display = "none";

    this.input.addEventListener("input", () => this.onInput());
    this.input.addEventListener("blur", () => this.onBlur());
    this.input.addEventListener("keydown", (e) => this.onKeydown(e));

    inputWrapper.appendChild(this.input);
    inputWrapper.appendChild(this.suggestions);
    editor.appendChild(inputWrapper);

    this.el.appendChild(editor);
  }

  private onInput(): void {
    const val = this.input.value.trim().toLowerCase();
    if (val.length === 0) {
      this.hideSuggestions();
      return;
    }

    const currentTags = this.getTagSet();
    const matches = this.options.allTags.filter(
      (t) => t.toLowerCase().includes(val) && !currentTags.has(t)
    );

    if (matches.length === 0) {
      this.hideSuggestions();
      return;
    }

    this.highlightIndex = -1;
    this.suggestions.innerHTML = "";
    for (const match of matches.slice(0, 8)) {
      const item = document.createElement("div");
      item.className = "tag-suggestion-item";
      item.textContent = match;
      item.addEventListener("mousedown", (e) => {
        e.preventDefault();
        this.addTag(match);
      });
      this.suggestions.appendChild(item);
    }
    this.suggestions.style.display = "block";
  }

  private onBlur(): void {
    this.blurTimer = setTimeout(() => {
      this.blurTimer = null;
      this.hideSuggestions();
    }, 150);
  }

  private onKeydown(e: KeyboardEvent): void {
    const items = this.suggestions.querySelectorAll<HTMLElement>(".tag-suggestion-item");

    if (e.key === "ArrowDown" && this.suggestions.style.display === "block") {
      e.preventDefault();
      this.highlightIndex = Math.min(this.highlightIndex + 1, items.length - 1);
      this.updateHighlight(items);
      return;
    }

    if (e.key === "ArrowUp" && this.suggestions.style.display === "block") {
      e.preventDefault();
      this.highlightIndex = Math.max(this.highlightIndex - 1, -1);
      this.updateHighlight(items);
      return;
    }

    if (e.key === "Escape") {
      e.stopPropagation();
      this.hideSuggestions();
      return;
    }

    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();

      // If a suggestion is highlighted, use it
      if (this.highlightIndex >= 0 && this.highlightIndex < items.length) {
        const selected = items[this.highlightIndex].textContent || "";
        this.addTag(selected);
        return;
      }

      const val = this.input.value.trim().replace(/,$/g, "");
      if (val) {
        const currentTags = this.getTagSet();
        if (!currentTags.has(val)) {
          this.addTag(val);
        } else {
          this.input.value = "";
          this.hideSuggestions();
        }
      }
    }
  }

  private updateHighlight(items: NodeListOf<HTMLElement>): void {
    items.forEach((item, i) => {
      item.classList.toggle("highlighted", i === this.highlightIndex);
    });
  }

  private addTag(name: string): void {
    const currentTags = this.getTagSet();
    if (currentTags.has(name)) return;

    this.addChip(name);
    this.input.value = "";
    this.hideSuggestions();
    this.options.onChange(this.getTags());
  }

  private addChip(tagName: string): void {
    const chip = document.createElement("span");
    chip.className = "tag-chip";
    chip.dataset.tag = tagName;

    const text = document.createElement("span");
    text.textContent = tagName;
    chip.appendChild(text);

    const removeBtn = document.createElement("button");
    removeBtn.className = "tag-chip-remove";
    removeBtn.textContent = "\u00D7";
    removeBtn.setAttribute("aria-label", `Tag ${tagName} entfernen`);
    removeBtn.addEventListener("click", () => {
      chip.remove();
      this.options.onChange(this.getTags());
    });
    chip.appendChild(removeBtn);

    this.chipContainer.appendChild(chip);
  }

  private hideSuggestions(): void {
    this.suggestions.style.display = "none";
    this.highlightIndex = -1;
  }

  private getTagSet(): Set<string> {
    return new Set(this.getTags());
  }

  getTags(): string[] {
    const chips = this.chipContainer.querySelectorAll<HTMLElement>(".tag-chip");
    const tags: string[] = [];
    chips.forEach((chip) => {
      const name = chip.dataset.tag;
      if (name) tags.push(name);
    });
    return tags;
  }

  setTags(tags: string[]): void {
    this.chipContainer.innerHTML = "";
    for (const tag of tags) {
      this.addChip(tag);
    }
  }

  setAllTags(allTags: string[]): void {
    this.options.allTags = allTags;
  }

  destroy(): void {
    if (this.blurTimer) {
      clearTimeout(this.blurTimer);
      this.blurTimer = null;
    }
    super.destroy();
  }
}
