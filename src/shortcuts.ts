import { EventBus } from "./state/EventBus";

function isInputFocused(): boolean {
  const el = document.activeElement;
  if (!el) return false;
  const tag = el.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || tag === "select";
}

export function initShortcuts(): () => void {
  const handler = (e: KeyboardEvent) => {
    const mod = e.metaKey || e.ctrlKey;

    // Always handle Escape regardless of focus
    if (e.key === "Escape") {
      EventBus.emit("shortcut:escape");
      return;
    }

    // Cmd+S always works (save should work even when editing fields)
    if (mod && e.key === "s") {
      e.preventDefault();
      EventBus.emit("shortcut:save");
      return;
    }

    // Cmd/Ctrl+Shift+R — reveal in folder
    if (mod && e.shiftKey && (e.key === "r" || e.key === "R")) {
      e.preventDefault();
      EventBus.emit("shortcut:reveal-in-folder");
      return;
    }

    // Other modifier shortcuts — skip when typing in inputs
    if (mod && !isInputFocused()) {
      switch (e.key) {
        case "f":
          e.preventDefault();
          EventBus.emit("shortcut:search");
          return;
        case ",":
          e.preventDefault();
          EventBus.emit("shortcut:settings");
          return;
      }
    }

    // Non-modifier shortcuts — skip when typing in inputs
    if (isInputFocused()) return;

    switch (e.key) {
      case "Delete":
      case "Backspace":
        e.preventDefault();
        EventBus.emit("shortcut:delete");
        break;
      case "ArrowUp":
        e.preventDefault();
        EventBus.emit("shortcut:prev-file");
        break;
      case "ArrowDown":
        e.preventDefault();
        EventBus.emit("shortcut:next-file");
        break;
    }
  };
  document.addEventListener("keydown", handler);
  return () => document.removeEventListener("keydown", handler);
}
