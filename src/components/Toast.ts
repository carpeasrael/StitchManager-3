import { appState } from "../state/AppState";
import type { Toast as ToastType, ToastLevel } from "../types/index";

let idCounter = 0;
const pendingTimers = new Set<ReturnType<typeof setTimeout>>();

export class ToastContainer {
  private el: HTMLElement;
  private unsubscribe: () => void;

  constructor() {
    this.el = document.createElement("div");
    this.el.className = "toast-container";
    this.el.setAttribute("aria-live", "polite");
    this.el.setAttribute("role", "status");
    document.body.appendChild(this.el);

    this.unsubscribe = appState.on("toasts", (toasts) => this.render(toasts));
  }

  destroy(): void {
    this.unsubscribe();
    this.el.remove();
    for (const timer of pendingTimers) {
      clearTimeout(timer);
    }
    pendingTimers.clear();
    appState.set("toasts", []);
  }

  private render(toasts: ToastType[]): void {
    // Remove toast elements no longer in state
    const existingIds = new Set(toasts.map((t) => t.id));
    for (const child of Array.from(this.el.children)) {
      const el = child as HTMLElement;
      if (!existingIds.has(el.dataset.toastId ?? "") && !el.classList.contains("toast-exit")) {
        el.classList.add("toast-exit");
        const exitTimer = setTimeout(() => {
          pendingTimers.delete(exitTimer);
          el.remove();
        }, 300);
        pendingTimers.add(exitTimer);
      }
    }

    // Add new toast elements (skip elements still animating out)
    for (const toast of toasts) {
      const existing = this.el.querySelector(`[data-toast-id="${toast.id}"]`) as HTMLElement | null;
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

      this.el.appendChild(el);
    }
  }

  static show(level: ToastLevel, message: string, duration = 4000): void {
    const id = `toast-${++idCounter}`;
    const toast: ToastType = { id, level, message };
    let current = appState.get("toasts");
    // Limit to 5 concurrent toasts — remove oldest if at capacity
    if (current.length >= 5) {
      current = current.slice(current.length - 4);
    }
    appState.set("toasts", [...current, toast]);

    const timer = setTimeout(() => {
      pendingTimers.delete(timer);
      const toasts = appState.get("toasts");
      appState.set(
        "toasts",
        toasts.filter((t) => t.id !== id)
      );
    }, duration);
    pendingTimers.add(timer);
  }
}
