import { appState } from "../state/AppState";
import type { Toast as ToastType, ToastLevel } from "../types/index";

let idCounter = 0;

export class ToastContainer {
  private el: HTMLElement;

  constructor() {
    this.el = document.createElement("div");
    this.el.className = "toast-container";
    document.body.appendChild(this.el);

    appState.on("toasts", (toasts) => this.render(toasts));
  }

  private render(toasts: ToastType[]): void {
    // Remove toast elements no longer in state
    const existingIds = new Set(toasts.map((t) => t.id));
    for (const child of Array.from(this.el.children)) {
      const el = child as HTMLElement;
      if (!existingIds.has(el.dataset.toastId ?? "") && !el.classList.contains("toast-exit")) {
        el.classList.add("toast-exit");
        setTimeout(() => el.remove(), 300);
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

    setTimeout(() => {
      const toasts = appState.get("toasts");
      appState.set(
        "toasts",
        toasts.filter((t) => t.id !== id)
      );
    }, duration);
  }
}
