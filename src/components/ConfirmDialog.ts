import { trapFocus } from "../utils/focus-trap";

/**
 * Audit Wave 3 usability: Aurora-styled confirm dialog that replaces the
 * 19 sites that used the browser-native `confirm()`. Focus-trapped,
 * Esc-closable, themed, German wording, and supports a destructive
 * variant (red primary button + optional permanence hint).
 */
export interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
  /** Extra hint shown muted under the message — e.g. "Diese Aktion kann nicht rückgängig gemacht werden." */
  hint?: string;
}

export class ConfirmDialog {
  /** Resolves to `true` if the user confirms, `false` otherwise. */
  static open(opts: ConfirmOptions): Promise<boolean> {
    return new Promise((resolve) => {
      const overlay = document.createElement("div");
      overlay.className = "dialog-overlay";

      const dialog = document.createElement("div");
      dialog.className = "dialog";
      dialog.setAttribute("role", "alertdialog");
      dialog.setAttribute("aria-modal", "true");
      dialog.setAttribute("aria-label", opts.title);
      dialog.tabIndex = -1;

      const header = document.createElement("div");
      header.className = "dialog-header";
      const title = document.createElement("h2");
      title.className = "dialog-title";
      title.textContent = opts.title;
      header.appendChild(title);
      dialog.appendChild(header);

      const body = document.createElement("div");
      body.className = "dialog-body";
      const msg = document.createElement("p");
      msg.className = "dialog-message";
      msg.textContent = opts.message;
      body.appendChild(msg);
      if (opts.hint) {
        const hint = document.createElement("p");
        hint.className = "dialog-hint";
        hint.textContent = opts.hint;
        body.appendChild(hint);
      }
      dialog.appendChild(body);

      const footer = document.createElement("div");
      footer.className = "dialog-footer";

      const cancelBtn = document.createElement("button");
      cancelBtn.type = "button";
      cancelBtn.className = "dialog-btn dialog-btn-secondary";
      cancelBtn.textContent = opts.cancelLabel ?? "Abbrechen";
      footer.appendChild(cancelBtn);

      const confirmBtn = document.createElement("button");
      confirmBtn.type = "button";
      confirmBtn.className = opts.destructive
        ? "dialog-btn dialog-btn-danger"
        : "dialog-btn dialog-btn-primary";
      confirmBtn.textContent = opts.confirmLabel ?? (opts.destructive ? "Löschen" : "Bestätigen");
      footer.appendChild(confirmBtn);

      dialog.appendChild(footer);
      overlay.appendChild(dialog);
      document.body.appendChild(overlay);

      const release = trapFocus(dialog);

      const close = (result: boolean) => {
        release();
        document.removeEventListener("keydown", onKey);
        overlay.remove();
        resolve(result);
      };
      const onKey = (e: KeyboardEvent) => {
        if (e.key === "Escape") {
          e.preventDefault();
          close(false);
        } else if (e.key === "Enter" && document.activeElement !== cancelBtn) {
          e.preventDefault();
          close(true);
        }
      };

      cancelBtn.addEventListener("click", () => close(false));
      confirmBtn.addEventListener("click", () => close(true));
      overlay.addEventListener("click", (e) => {
        if (e.target === overlay) close(false);
      });
      document.addEventListener("keydown", onKey);
    });
  }
}
