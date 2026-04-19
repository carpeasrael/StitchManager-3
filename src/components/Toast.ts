import { appState } from "../state/AppState";
import type { Toast as ToastType, ToastLevel } from "../types/index";

let idCounter = 0;
const pendingTimers = new Map<string, ReturnType<typeof setTimeout>>();
const exitTimers = new Set<ReturnType<typeof setTimeout>>();

/**
 * Audit Wave 3 usability:
 * - Errors persist until manually dismissed (no auto-timer); success/info
 *   default to 4s. Callers can still override via `duration`.
 * - Each toast renders a × close button (with `aria-label="Schließen"`).
 * - Errors get `role="alert"` + `aria-live="assertive"` so screen readers
 *   interrupt; success/info stay in the polite container.
 * - When the cap (5) is reached, prefer to drop the oldest non-error
 *   toast first so unread errors are not silently displaced.
 */
export class ToastContainer {
  private el: HTMLElement;
  private alertEl: HTMLElement;
  private unsubscribe: () => void;

  constructor() {
    this.el = document.createElement("div");
    this.el.className = "toast-container";
    this.el.setAttribute("aria-live", "polite");
    this.el.setAttribute("role", "status");
    document.body.appendChild(this.el);

    // Separate assertive container for error toasts so screen readers
    // interrupt rather than queue.
    this.alertEl = document.createElement("div");
    this.alertEl.className = "toast-container toast-container--alert";
    this.alertEl.setAttribute("aria-live", "assertive");
    this.alertEl.setAttribute("role", "alert");
    document.body.appendChild(this.alertEl);

    this.unsubscribe = appState.on("toasts", (toasts) => this.render(toasts));
  }

  destroy(): void {
    this.unsubscribe();
    this.el.remove();
    this.alertEl.remove();
    for (const timer of pendingTimers.values()) clearTimeout(timer);
    pendingTimers.clear();
    for (const timer of exitTimers) clearTimeout(timer);
    exitTimers.clear();
    appState.set("toasts", []);
  }

  private render(toasts: ToastType[]): void {
    const existingIds = new Set(toasts.map((t) => t.id));
    // Remove toast elements no longer in state from BOTH containers.
    for (const root of [this.el, this.alertEl]) {
      for (const child of Array.from(root.children)) {
        const el = child as HTMLElement;
        if (!existingIds.has(el.dataset.toastId ?? "") && !el.classList.contains("toast-exit")) {
          el.classList.add("toast-exit");
          const exitTimer = setTimeout(() => {
            exitTimers.delete(exitTimer);
            el.remove();
          }, 300);
          exitTimers.add(exitTimer);
        }
      }
    }

    for (const toast of toasts) {
      const target = toast.level === "error" ? this.alertEl : this.el;
      const existing = target.querySelector(`[data-toast-id="${toast.id}"]`) as HTMLElement | null;
      if (existing && !existing.classList.contains("toast-exit")) continue;

      const el = document.createElement("div");
      el.className = `toast toast-${toast.level}`;
      el.dataset.toastId = toast.id;

      const icon = document.createElement("span");
      icon.className = "toast-icon";
      icon.textContent =
        toast.level === "success"
          ? "\u2713"
          : toast.level === "error"
            ? "\u2717"
            : "\u2139";
      el.appendChild(icon);

      const msg = document.createElement("span");
      msg.className = "toast-message";
      msg.textContent = toast.message;
      el.appendChild(msg);

      // Audit Wave 3 usability: every toast gets an explicit close button.
      const closeBtn = document.createElement("button");
      closeBtn.type = "button";
      closeBtn.className = "toast-close";
      closeBtn.textContent = "\u00D7";
      closeBtn.setAttribute("aria-label", "Schließen");
      closeBtn.addEventListener("click", () => ToastContainer.dismiss(toast.id));
      el.appendChild(closeBtn);

      target.appendChild(el);
    }
  }

  static show(level: ToastLevel, message: string, duration?: number): void {
    const id = `toast-${++idCounter}`;
    const toast: ToastType = { id, level, message };

    // Audit Wave 3 usability: when capped, prefer dropping the oldest
    // non-error toast so unread errors are not silently lost.
    let current = appState.get("toasts");
    if (current.length >= 5) {
      const idxNonError = current.findIndex((t) => t.level !== "error");
      if (idxNonError >= 0) {
        current = [...current.slice(0, idxNonError), ...current.slice(idxNonError + 1)];
      } else {
        current = current.slice(1); // all are errors — drop oldest anyway
      }
    }
    appState.set("toasts", [...current, toast]);

    // Errors persist until dismissed unless an explicit duration was passed.
    const auto = duration ?? (level === "error" ? Infinity : 4000);
    if (Number.isFinite(auto)) {
      const timer = setTimeout(() => {
        pendingTimers.delete(id);
        ToastContainer.dismiss(id);
      }, auto as number);
      pendingTimers.set(id, timer);
    }
  }

  /** Remove a specific toast immediately. */
  static dismiss(id: string): void {
    const t = pendingTimers.get(id);
    if (t) {
      clearTimeout(t);
      pendingTimers.delete(id);
    }
    const toasts = appState.get("toasts");
    appState.set("toasts", toasts.filter((t) => t.id !== id));
  }
}
