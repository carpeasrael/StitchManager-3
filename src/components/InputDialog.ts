import { trapFocus } from "../utils/focus-trap";

/**
 * Audit Wave 3 usability: Aurora-styled input dialog that replaces the
 * sites that used the browser-native `prompt()`. Focus-trapped, themed,
 * supports an optional validator that returns a German error string when
 * the value is invalid (input then keeps focus until corrected).
 */
export interface InputDialogOptions {
  title: string;
  label: string;
  placeholder?: string;
  initialValue?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Return null/undefined for OK, an error message string for invalid input. */
  validate?: (value: string) => string | null | undefined;
}

export class InputDialog {
  /** Resolves to the trimmed value, or `null` if cancelled. */
  static open(opts: InputDialogOptions): Promise<string | null> {
    return new Promise((resolve) => {
      const overlay = document.createElement("div");
      overlay.className = "dialog-overlay";

      const dialog = document.createElement("div");
      dialog.className = "dialog";
      dialog.setAttribute("role", "dialog");
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

      const fieldId = `input-dialog-${Math.random().toString(36).slice(2, 8)}`;
      const label = document.createElement("label");
      label.className = "dialog-label";
      label.htmlFor = fieldId;
      label.textContent = opts.label;
      body.appendChild(label);

      const input = document.createElement("input");
      input.type = "text";
      input.id = fieldId;
      input.className = "dialog-input";
      if (opts.placeholder) input.placeholder = opts.placeholder;
      if (opts.initialValue) input.value = opts.initialValue;
      body.appendChild(input);

      const errorEl = document.createElement("p");
      errorEl.className = "dialog-error";
      errorEl.setAttribute("role", "alert");
      errorEl.style.display = "none";
      body.appendChild(errorEl);

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
      confirmBtn.className = "dialog-btn dialog-btn-primary";
      confirmBtn.textContent = opts.confirmLabel ?? "OK";
      footer.appendChild(confirmBtn);

      dialog.appendChild(footer);
      overlay.appendChild(dialog);
      document.body.appendChild(overlay);

      const release = trapFocus(dialog);
      // Focus the input rather than the first focusable (which is the field
      // by virtue of source order, but be explicit).
      requestAnimationFrame(() => input.focus());

      const tryConfirm = () => {
        const value = input.value.trim();
        if (opts.validate) {
          const err = opts.validate(value);
          if (err) {
            errorEl.textContent = err;
            errorEl.style.display = "";
            input.focus();
            return;
          }
        }
        close(value);
      };

      const close = (result: string | null) => {
        release();
        document.removeEventListener("keydown", onKey);
        overlay.remove();
        resolve(result);
      };
      const onKey = (e: KeyboardEvent) => {
        if (e.key === "Escape") {
          e.preventDefault();
          close(null);
        } else if (e.key === "Enter" && document.activeElement !== cancelBtn) {
          e.preventDefault();
          tryConfirm();
        }
      };

      cancelBtn.addEventListener("click", () => close(null));
      confirmBtn.addEventListener("click", tryConfirm);
      overlay.addEventListener("click", (e) => {
        if (e.target === overlay) close(null);
      });
      document.addEventListener("keydown", onKey);
    });
  }
}
