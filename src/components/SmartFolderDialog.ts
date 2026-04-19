import { ToastContainer } from "./Toast";
import { trapFocus } from "../utils/focus-trap";
import { appState } from "../state/AppState";
import * as SmartFolderService from "../services/SmartFolderService";
import type { SearchParams } from "../types/index";

export class SmartFolderDialog {
  private static instance: SmartFolderDialog | null = null;

  static open(): void {
    if (SmartFolderDialog.instance) return;
    const dialog = new SmartFolderDialog();
    SmartFolderDialog.instance = dialog;
    dialog.show();
  }

  private overlay: HTMLElement | null = null;
  private releaseFocusTrap: (() => void) | null = null;

  private show(): void {
    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("click", (e) => {
      if (e.target === this.overlay) this.close();
    });

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-smart-folder";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Intelligenter Ordner");

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";
    const title = document.createElement("h3");
    title.className = "dialog-title";
    title.textContent = "Neuer intelligenter Ordner";
    header.appendChild(title);
    const closeBtn = document.createElement("button");
    closeBtn.className = "dialog-close-btn";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Dialog schließen");
    closeBtn.addEventListener("click", () => this.close());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    // Body
    const body = document.createElement("div");
    body.className = "dialog-body dialog-smart-folder-body";

    // Name
    const nameGroup = document.createElement("div");
    nameGroup.className = "settings-form-group";
    const nameLabel = document.createElement("label");
    nameLabel.className = "settings-label";
    nameLabel.textContent = "Name";
    nameLabel.htmlFor = "sf-name";
    const nameInput = document.createElement("input");
    nameInput.className = "settings-input";
    nameInput.id = "sf-name";
    nameInput.type = "text";
    nameInput.placeholder = "z.B. Weihnachtsmuster";
    nameGroup.appendChild(nameLabel);
    nameGroup.appendChild(nameInput);
    body.appendChild(nameGroup);

    // Icon
    const iconGroup = document.createElement("div");
    iconGroup.className = "settings-form-group";
    const iconLabel = document.createElement("label");
    iconLabel.className = "settings-label";
    iconLabel.textContent = "Symbol";
    iconLabel.htmlFor = "sf-icon";
    const iconInput = document.createElement("input");
    iconInput.className = "settings-input";
    iconInput.id = "sf-icon";
    iconInput.type = "text";
    iconInput.value = "\uD83D\uDD0D";
    iconInput.style.width = "60px";
    iconGroup.appendChild(iconLabel);
    iconGroup.appendChild(iconInput);
    body.appendChild(iconGroup);

    // Filter fields
    const filterHeader = document.createElement("div");
    filterHeader.className = "settings-label";
    filterHeader.textContent = "Filterkriterien";
    filterHeader.style.fontWeight = "600";
    filterHeader.style.marginTop = "8px";
    body.appendChild(filterHeader);

    // Text search
    const textGroup = this.makeTextInput("Textsuche", "sf-text", "");
    body.appendChild(textGroup.group);

    // Tags
    const tagsGroup = this.makeTextInput("Tags (kommasepariert)", "sf-tags", "z.B. Weihnachten, Blumen");
    body.appendChild(tagsGroup.group);

    // File type
    const typeGroup = document.createElement("div");
    typeGroup.className = "settings-form-group";
    const typeLabel = document.createElement("label");
    typeLabel.className = "settings-label";
    typeLabel.textContent = "Dateityp";
    const typeSelect = document.createElement("select");
    typeSelect.className = "settings-input";
    typeSelect.id = "sf-filetype";
    for (const [val, label] of [["", "-- Alle --"], ["embroidery", "Stickmuster"], ["sewing_pattern", "Schnittmuster"]]) {
      const opt = document.createElement("option");
      opt.value = val;
      opt.textContent = label;
      typeSelect.appendChild(opt);
    }
    typeGroup.appendChild(typeLabel);
    typeGroup.appendChild(typeSelect);
    body.appendChild(typeGroup);

    // AI status
    const aiGroup = document.createElement("div");
    aiGroup.className = "settings-form-group";
    const aiLabel = document.createElement("label");
    aiLabel.className = "settings-label";
    aiLabel.textContent = "KI-Status";
    const aiSelect = document.createElement("select");
    aiSelect.className = "settings-input";
    aiSelect.id = "sf-ai";
    for (const [val, label] of [["", "-- Alle --"], ["false", "Nicht analysiert"], ["true", "Analysiert"], ["confirmed", "Bestätigt"]]) {
      const opt = document.createElement("option");
      opt.value = val;
      opt.textContent = label;
      aiSelect.appendChild(opt);
    }
    aiGroup.appendChild(aiLabel);
    aiGroup.appendChild(aiSelect);
    body.appendChild(aiGroup);

    // Rating min
    const ratingGroup = document.createElement("div");
    ratingGroup.className = "settings-form-group";
    const ratingLabel = document.createElement("label");
    ratingLabel.className = "settings-label";
    ratingLabel.textContent = "Mindestbewertung";
    const ratingSelect = document.createElement("select");
    ratingSelect.className = "settings-input";
    ratingSelect.id = "sf-rating";
    const noR = document.createElement("option");
    noR.value = "";
    noR.textContent = "-- Keine --";
    ratingSelect.appendChild(noR);
    for (let i = 1; i <= 5; i++) {
      const opt = document.createElement("option");
      opt.value = String(i);
      opt.textContent = "\u2605".repeat(i);
      ratingSelect.appendChild(opt);
    }
    ratingGroup.appendChild(ratingLabel);
    ratingGroup.appendChild(ratingSelect);
    body.appendChild(ratingGroup);

    // Favorite
    const favGroup = document.createElement("div");
    favGroup.className = "settings-form-group";
    const favLabel = document.createElement("label");
    favLabel.className = "settings-label";
    favLabel.textContent = "Nur Favoriten";
    const favCheck = document.createElement("input");
    favCheck.type = "checkbox";
    favCheck.id = "sf-fav";
    favGroup.appendChild(favLabel);
    favGroup.appendChild(favCheck);
    body.appendChild(favGroup);

    // "From current filter" button
    const fromCurrentBtn = document.createElement("button");
    fromCurrentBtn.className = "btn btn-small";
    fromCurrentBtn.textContent = "Aus aktuellem Filter uebernehmen";
    fromCurrentBtn.addEventListener("click", () => {
      const sp = appState.get("searchParams");
      if (sp.text) textGroup.input.value = sp.text;
      if (sp.tags) tagsGroup.input.value = sp.tags.join(", ");
      if (sp.fileType) typeSelect.value = sp.fileType;
      if (sp.aiAnalyzed !== undefined) aiSelect.value = String(sp.aiAnalyzed);
      if (sp.ratingMin) ratingSelect.value = String(sp.ratingMin);
      if (sp.isFavorite) favCheck.checked = true;
    });
    body.appendChild(fromCurrentBtn);

    dialog.appendChild(body);

    // Footer
    const footer = document.createElement("div");
    footer.className = "dialog-footer";
    const cancelBtn = document.createElement("button");
    cancelBtn.className = "btn btn-secondary";
    cancelBtn.textContent = "Abbrechen";
    cancelBtn.addEventListener("click", () => this.close());

    const createBtn = document.createElement("button");
    createBtn.className = "btn btn-primary";
    createBtn.textContent = "Erstellen";
    createBtn.addEventListener("click", async () => {
      const name = nameInput.value.trim();
      if (!name) {
        ToastContainer.show("error", "Bitte einen Namen eingeben");
        nameInput.focus();
        return;
      }

      // Build filter JSON from form fields
      const filter: Partial<SearchParams> = {};
      const text = textGroup.input.value.trim();
      if (text) filter.text = text;
      const tags = tagsGroup.input.value.trim();
      if (tags) filter.tags = tags.split(",").map((t) => t.trim()).filter(Boolean);
      if (typeSelect.value) filter.fileType = typeSelect.value;
      if (aiSelect.value === "false") {
        filter.aiAnalyzed = false;
      } else if (aiSelect.value === "true") {
        filter.aiAnalyzed = true;
      } else if (aiSelect.value === "confirmed") {
        filter.aiAnalyzed = true;
        filter.aiConfirmed = true;
      }
      if (ratingSelect.value) filter.ratingMin = Number(ratingSelect.value);
      if (favCheck.checked) filter.isFavorite = true;

      const filterJson = JSON.stringify(filter);
      const icon = iconInput.value.trim() || "\uD83D\uDD0D";

      createBtn.disabled = true;
      try {
        await SmartFolderService.create(name, filterJson, icon);
        const all = await SmartFolderService.getAll();
        appState.set("smartFolders", all);
        ToastContainer.show("success", `"${name}" erstellt`);
        this.close();
      } catch (e) {
        const msg =
          e && typeof e === "object" && "message" in e
            ? (e as { message: string }).message
            : String(e);
        ToastContainer.show("error", `Fehler: ${msg}`);
        createBtn.disabled = false;
      }
    });

    footer.appendChild(cancelBtn);
    footer.appendChild(createBtn);
    dialog.appendChild(footer);

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);

    dialog.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this.close();
      }
    });

    this.releaseFocusTrap = trapFocus(dialog);
  }

  private makeTextInput(label: string, id: string, placeholder: string) {
    const group = document.createElement("div");
    group.className = "settings-form-group";
    const lbl = document.createElement("label");
    lbl.className = "settings-label";
    lbl.textContent = label;
    lbl.htmlFor = id;
    const input = document.createElement("input");
    input.className = "settings-input";
    input.id = id;
    input.type = "text";
    input.placeholder = placeholder;
    group.appendChild(lbl);
    group.appendChild(input);
    return { group, input };
  }

  private close(): void {
    if (this.releaseFocusTrap) {
      this.releaseFocusTrap();
      this.releaseFocusTrap = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
    SmartFolderDialog.instance = null;
  }
}
