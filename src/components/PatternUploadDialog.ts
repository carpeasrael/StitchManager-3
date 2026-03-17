import { open } from "@tauri-apps/plugin-dialog";
import { ToastContainer } from "./Toast";
import { trapFocus } from "../utils/focus-trap";
import * as FileService from "../services/FileService";
import * as ProjectService from "../services/ProjectService";
import { EventBus } from "../state/EventBus";
import type { Collection } from "../types";

export class PatternUploadDialog {
  private static instance: PatternUploadDialog | null = null;

  private overlay: HTMLElement | null = null;
  private keyHandler: ((e: KeyboardEvent) => void) | null = null;
  private releaseFocusTrap: (() => void) | null = null;

  private selectedPath: string | null = null;
  private collections: Collection[] = [];
  private rating = 0;

  static async open(): Promise<void> {
    if (PatternUploadDialog.instance) PatternUploadDialog.dismiss();
    const dialog = new PatternUploadDialog();
    PatternUploadDialog.instance = dialog;
    await dialog.init();
  }

  static dismiss(): void {
    if (PatternUploadDialog.instance) {
      PatternUploadDialog.instance.close();
      PatternUploadDialog.instance = null;
    }
  }

  private async init(): Promise<void> {
    try {
      this.collections = await ProjectService.getCollections();
    } catch {
      this.collections = [];
    }
    this.overlay = this.buildUI();
    document.body.appendChild(this.overlay);
    const dialog = this.overlay.querySelector<HTMLElement>(".dialog") || this.overlay;
    this.releaseFocusTrap = trapFocus(dialog);
    this.keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        PatternUploadDialog.dismiss();
      }
    };
    document.addEventListener("keydown", this.keyHandler);
  }

  private close(): void {
    if (this.releaseFocusTrap) this.releaseFocusTrap();
    if (this.keyHandler) document.removeEventListener("keydown", this.keyHandler);
    this.overlay?.remove();
    this.overlay = null;
  }

  private buildUI(): HTMLElement {
    const overlay = document.createElement("div");
    overlay.className = "mfg-overlay";

    const dialog = document.createElement("div");
    dialog.className = "mfg-dialog pattern-upload-dialog";
    dialog.style.maxWidth = "620px";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Schnittmuster hochladen");

    // Header
    const header = document.createElement("div");
    header.className = "mfg-header";
    const title = document.createElement("h2");
    title.className = "mfg-title";
    title.textContent = "Schnittmuster hochladen";
    header.appendChild(title);
    const closeBtn = document.createElement("button");
    closeBtn.className = "dv-close-btn";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Schliessen");
    closeBtn.addEventListener("click", () => PatternUploadDialog.dismiss());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    // Content
    const content = document.createElement("div");
    content.style.padding = "var(--spacing-3)";
    content.style.overflowY = "auto";
    content.style.maxHeight = "70vh";

    const form = document.createElement("div");
    form.className = "mfg-form";

    // File picker
    const fileRow = document.createElement("div");
    fileRow.className = "mfg-field";
    fileRow.style.display = "flex";
    fileRow.style.gap = "var(--spacing-2)";
    fileRow.style.alignItems = "center";
    const fileLabel = document.createElement("label");
    fileLabel.className = "mfg-label";
    fileLabel.textContent = "Datei:";
    const fileDisplay = document.createElement("span");
    fileDisplay.style.flex = "1";
    fileDisplay.style.opacity = "0.6";
    fileDisplay.textContent = "Keine Datei gewaehlt";
    const browseBtn = document.createElement("button");
    browseBtn.className = "dialog-btn dialog-btn-primary";
    browseBtn.textContent = "Durchsuchen...";
    browseBtn.addEventListener("click", async () => {
      const result = await open({
        multiple: false,
        filters: [{ name: "Schnittmuster", extensions: ["pdf", "png", "jpg", "jpeg", "bmp"] }],
      });
      if (result) {
        this.selectedPath = result as string;
        const name = this.selectedPath.split(/[/\\]/).pop() || "";
        fileDisplay.textContent = name;
        fileDisplay.style.opacity = "1";
        // Auto-fill name if empty
        const nameInput = form.querySelector<HTMLInputElement>('[data-field="name"]');
        if (nameInput && !nameInput.value) {
          nameInput.value = name.replace(/\.[^.]+$/, "");
        }
        uploadBtn.disabled = false;
      }
    });
    fileRow.appendChild(fileLabel);
    fileRow.appendChild(fileDisplay);
    fileRow.appendChild(browseBtn);
    form.appendChild(fileRow);

    // Collection selector
    const colRow = document.createElement("div");
    colRow.className = "mfg-field";
    const colLabel = document.createElement("label");
    colLabel.className = "mfg-label";
    colLabel.textContent = "Sammlung:";
    const colSelect = document.createElement("select");
    colSelect.className = "mfg-input";
    colSelect.dataset.field = "collection";
    const noneOpt = document.createElement("option");
    noneOpt.value = "";
    noneOpt.textContent = "Keine Sammlung";
    colSelect.appendChild(noneOpt);
    for (const c of this.collections) {
      const opt = document.createElement("option");
      opt.value = String(c.id);
      opt.textContent = c.name;
      colSelect.appendChild(opt);
    }
    colRow.appendChild(colLabel);
    colRow.appendChild(colSelect);
    form.appendChild(colRow);

    // Text fields
    const textFields: { key: string; label: string; type?: string }[] = [
      { key: "name", label: "Name" },
      { key: "designer", label: "Designer" },
      { key: "license", label: "Lizenz" },
      { key: "source", label: "Quelle" },
      { key: "description", label: "Beschreibung" },
      { key: "patternDate", label: "Datum", type: "date" },
    ];
    for (const f of textFields) {
      const row = document.createElement("div");
      row.className = "mfg-field";
      const label = document.createElement("label");
      label.className = "mfg-label";
      label.textContent = f.label + ":";
      const input = document.createElement("input");
      input.className = "mfg-input";
      input.type = f.type || "text";
      input.dataset.field = f.key;
      if (f.key === "patternDate") {
        input.value = new Date().toISOString().split("T")[0];
      }
      row.appendChild(label);
      row.appendChild(input);
      form.appendChild(row);
    }

    // Difficulty
    const diffRow = document.createElement("div");
    diffRow.className = "mfg-field";
    const diffLabel = document.createElement("label");
    diffLabel.className = "mfg-label";
    diffLabel.textContent = "Schwierigkeitsgrad:";
    const diffSelect = document.createElement("select");
    diffSelect.className = "mfg-input";
    diffSelect.dataset.field = "skillLevel";
    const levels = [
      { value: "", label: "Keine Angabe" },
      { value: "beginner", label: "Anfaenger" },
      { value: "easy", label: "Einfach" },
      { value: "intermediate", label: "Mittel" },
      { value: "advanced", label: "Fortgeschritten" },
      { value: "expert", label: "Experte" },
    ];
    for (const l of levels) {
      const opt = document.createElement("option");
      opt.value = l.value;
      opt.textContent = l.label;
      diffSelect.appendChild(opt);
    }
    diffRow.appendChild(diffLabel);
    diffRow.appendChild(diffSelect);
    form.appendChild(diffRow);

    // Star rating
    const ratingRow = document.createElement("div");
    ratingRow.className = "mfg-field";
    const ratingLabel = document.createElement("label");
    ratingLabel.className = "mfg-label";
    ratingLabel.textContent = "Bewertung:";
    const starContainer = document.createElement("div");
    starContainer.className = "star-rating";
    starContainer.setAttribute("role", "radiogroup");
    starContainer.setAttribute("aria-label", "Bewertung");
    for (let i = 1; i <= 5; i++) {
      const star = document.createElement("span");
      star.className = "star";
      star.textContent = "\u2605";
      star.dataset.value = String(i);
      star.addEventListener("mouseenter", () => {
        starContainer.querySelectorAll(".star").forEach((s) => {
          (s as HTMLElement).classList.toggle("hover-fill", Number((s as HTMLElement).dataset.value) <= i);
        });
      });
      star.addEventListener("mouseleave", () => {
        starContainer.querySelectorAll(".star").forEach((s) => (s as HTMLElement).classList.remove("hover-fill"));
      });
      star.addEventListener("click", () => {
        this.rating = this.rating === i ? 0 : i;
        starContainer.querySelectorAll(".star").forEach((s) => {
          (s as HTMLElement).classList.toggle("filled", Number((s as HTMLElement).dataset.value) <= this.rating);
        });
      });
      starContainer.appendChild(star);
    }
    ratingRow.appendChild(ratingLabel);
    ratingRow.appendChild(starContainer);
    form.appendChild(ratingRow);

    // Rich text - Anleitung
    const instrRow = document.createElement("div");
    instrRow.className = "mfg-field";
    const instrLabel = document.createElement("label");
    instrLabel.className = "mfg-label";
    instrLabel.textContent = "Anleitung:";
    instrRow.appendChild(instrLabel);

    const toolbar = document.createElement("div");
    toolbar.className = "rt-toolbar";
    const toolbarBtns = [
      { cmd: "bold", label: "B", style: "font-weight:bold" },
      { cmd: "italic", label: "I", style: "font-style:italic" },
      { cmd: "insertUnorderedList", label: "\u2022 Liste", style: "" },
    ];
    for (const tb of toolbarBtns) {
      const btn = document.createElement("button");
      btn.className = "rt-btn";
      btn.type = "button";
      btn.innerHTML = `<span style="${tb.style}">${tb.label}</span>`;
      btn.addEventListener("click", (e) => {
        e.preventDefault();
        document.execCommand(tb.cmd, false);
      });
      toolbar.appendChild(btn);
    }
    instrRow.appendChild(toolbar);

    const editor = document.createElement("div");
    editor.className = "rt-editor";
    editor.contentEditable = "true";
    editor.dataset.field = "instructionsHtml";
    // Strip formatting on paste
    editor.addEventListener("paste", (e) => {
      e.preventDefault();
      const text = e.clipboardData?.getData("text/plain") || "";
      document.execCommand("insertText", false, text);
    });
    instrRow.appendChild(editor);
    form.appendChild(instrRow);

    content.appendChild(form);
    dialog.appendChild(content);

    // Footer
    const footer = document.createElement("div");
    footer.className = "mfg-actions";
    footer.style.padding = "var(--spacing-2) var(--spacing-3)";
    footer.style.borderTop = "1px solid var(--color-border-light)";

    const errorDisplay = document.createElement("div");
    errorDisplay.className = "mfg-tt-hint";
    errorDisplay.style.color = "var(--color-danger)";
    errorDisplay.style.flex = "1";
    footer.appendChild(errorDisplay);

    const cancelBtn = document.createElement("button");
    cancelBtn.className = "dialog-btn";
    cancelBtn.textContent = "Abbrechen";
    cancelBtn.addEventListener("click", () => PatternUploadDialog.dismiss());
    footer.appendChild(cancelBtn);

    const uploadBtn = document.createElement("button");
    uploadBtn.className = "dialog-btn dialog-btn-primary";
    uploadBtn.textContent = "Hochladen";
    uploadBtn.disabled = true;
    uploadBtn.addEventListener("click", async () => {
      if (!this.selectedPath) return;
      errorDisplay.textContent = "";
      uploadBtn.disabled = true;
      uploadBtn.textContent = "Wird hochgeladen...";

      const getValue = (key: string): string => {
        const el = form.querySelector<HTMLInputElement | HTMLSelectElement>(`[data-field="${key}"]`);
        return el?.value?.trim() || "";
      };

      try {
        const colId = colSelect.value ? Number(colSelect.value) : null;
        const file = await FileService.uploadSewingPattern(this.selectedPath, colId, {
          name: getValue("name") || undefined,
          designer: getValue("designer") || undefined,
          license: getValue("license") || undefined,
          source: getValue("source") || undefined,
          description: getValue("description") || undefined,
          patternDate: getValue("patternDate") || undefined,
          skillLevel: getValue("skillLevel") || undefined,
          rating: this.rating > 0 ? this.rating : undefined,
          instructionsHtml: editor.innerHTML.trim() || undefined,
        });

        ToastContainer.show("success", `"${file.name || file.filename}" hochgeladen`);

        if (colId) {
          const col = this.collections.find((c) => c.id === colId);
          EventBus.emit("collection:selected", { collectionId: colId, collectionName: col?.name || "" });
        } else {
          EventBus.emit("files:refresh");
        }

        PatternUploadDialog.dismiss();
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : typeof e === "object" && e !== null && "message" in e ? String((e as { message: string }).message) : "Upload fehlgeschlagen";
        errorDisplay.textContent = msg;
        uploadBtn.disabled = false;
        uploadBtn.textContent = "Hochladen";
      }
    });
    footer.appendChild(uploadBtn);

    dialog.appendChild(footer);
    overlay.appendChild(dialog);
    return overlay;
  }
}
