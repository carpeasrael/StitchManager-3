import { trapFocus } from "../utils/focus-trap";

/**
 * Audit Wave 5 (deferred from Wave 3 #8): in-app keyboard-shortcut help.
 * Triggered by `?` (or via the burger menu). Aurora-styled dialog,
 * focus-trapped, Esc-closable.
 */
interface ShortcutGroup {
  title: string;
  items: { keys: string; description: string }[];
}

const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
const MOD = isMac ? "⌘" : "Strg";

const GROUPS: ShortcutGroup[] = [
  {
    title: "Allgemein",
    items: [
      { keys: "Esc", description: "Dialog schließen / Auswahl aufheben" },
      { keys: `${MOD}+,`, description: "Einstellungen öffnen" },
      { keys: "?", description: "Diese Hilfe anzeigen" },
    ],
  },
  {
    title: "Datei",
    items: [
      { keys: `${MOD}+S`, description: "Speichern" },
      { keys: `${MOD}+P`, description: "Drucken" },
      { keys: `${MOD}+F`, description: "Suchen" },
      { keys: `${MOD}+K`, description: "KI-Analyse für ausgewählte Datei" },
      { keys: `${MOD}+A`, description: "Alle Dateien auswählen" },
      { keys: `${MOD}+N`, description: "Neuer Ordner" },
      { keys: "Entf / Backspace", description: "Datei in Papierkorb verschieben" },
      { keys: "↑ / ↓", description: "Vorherige / nächste Datei" },
      { keys: `${MOD}+⇧+R`, description: "Im Finder/Explorer anzeigen" },
      { keys: `${MOD}+⇧+U`, description: "USB-Export" },
    ],
  },
  {
    title: "Splitter",
    items: [
      { keys: "Tab → ← / →", description: "Spaltenbreite anpassen (16 px)" },
      { keys: "Tab → ⇧+← / →", description: "Spaltenbreite anpassen (64 px)" },
      { keys: "Tab → Pos1 / Ende", description: "Minimale / maximale Breite" },
    ],
  },
];

export class HelpDialog {
  private static instance: HelpDialog | null = null;

  static open(): void {
    if (HelpDialog.instance) {
      HelpDialog.dismiss();
      return;
    }
    HelpDialog.instance = new HelpDialog();
    HelpDialog.instance.show();
  }

  static dismiss(): void {
    HelpDialog.instance?.close();
    HelpDialog.instance = null;
  }

  private overlay: HTMLElement;
  private releaseFocusTrap: (() => void) | null = null;
  private keyHandler: ((e: KeyboardEvent) => void) | null = null;

  private constructor() {
    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";

    const dialog = document.createElement("div");
    dialog.className = "dialog";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Tastaturkürzel");
    dialog.tabIndex = -1;

    const header = document.createElement("div");
    header.className = "dialog-header";
    const title = document.createElement("h2");
    title.className = "dialog-title";
    title.textContent = "Tastaturkürzel";
    header.appendChild(title);

    const closeBtn = document.createElement("button");
    closeBtn.type = "button";
    closeBtn.className = "dialog-close";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Schließen");
    closeBtn.addEventListener("click", () => HelpDialog.dismiss());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    const body = document.createElement("div");
    body.className = "dialog-body help-dialog-body";
    for (const group of GROUPS) {
      const section = document.createElement("section");
      section.className = "help-section";

      const heading = document.createElement("h3");
      heading.className = "help-section-title";
      heading.textContent = group.title;
      section.appendChild(heading);

      const list = document.createElement("dl");
      list.className = "help-list";
      for (const item of group.items) {
        const dt = document.createElement("dt");
        dt.className = "help-keys";
        dt.textContent = item.keys;
        const dd = document.createElement("dd");
        dd.className = "help-description";
        dd.textContent = item.description;
        list.appendChild(dt);
        list.appendChild(dd);
      }
      section.appendChild(list);
      body.appendChild(section);
    }
    dialog.appendChild(body);

    this.overlay.appendChild(dialog);
  }

  private show(): void {
    document.body.appendChild(this.overlay);
    const dialogEl = this.overlay.querySelector<HTMLElement>(".dialog");
    if (dialogEl) this.releaseFocusTrap = trapFocus(dialogEl);
    this.keyHandler = (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        HelpDialog.dismiss();
      }
    };
    document.addEventListener("keydown", this.keyHandler);
  }

  private close(): void {
    if (this.keyHandler) {
      document.removeEventListener("keydown", this.keyHandler);
      this.keyHandler = null;
    }
    if (this.releaseFocusTrap) {
      this.releaseFocusTrap();
      this.releaseFocusTrap = null;
    }
    this.overlay.remove();
  }
}
