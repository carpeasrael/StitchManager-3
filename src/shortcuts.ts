import { EventBus } from "./state/EventBus";

function isInputFocused(): boolean {
  const el = document.activeElement;
  if (!el) return false;
  const tag = el.tagName.toLowerCase();
  // Audit Wave 5: also treat contenteditable elements as inputs so the new
  // Ctrl+A / `?` shortcuts don't hijack the rich-text editors in
  // MetadataPanel and PatternUploadDialog.
  if (el instanceof HTMLElement && el.isContentEditable) return true;
  return tag === "input" || tag === "textarea" || tag === "select";
}

export function initShortcuts(): () => void {
  const handler = (e: KeyboardEvent) => {
    const mod = e.metaKey || e.ctrlKey;

    // Always handle Escape regardless of focus — but skip if a singleton dialog is open
    // (singleton dialogs handle their own Escape via document-level keydown)
    if (e.key === "Escape") {
      const hasOverlay = document.querySelector(
        ".document-viewer-overlay, .image-viewer-overlay, .print-preview-overlay, .project-list-overlay"
      );
      if (!hasOverlay) {
        EventBus.emit("shortcut:escape");
      }
      return;
    }

    // Cmd+S always works (save should work even when editing fields)
    if (mod && e.key === "s") {
      e.preventDefault();
      EventBus.emit("shortcut:save");
      return;
    }

    // Cmd/Ctrl+P — print
    if (mod && e.key === "p") {
      e.preventDefault();
      EventBus.emit("toolbar:print");
      return;
    }

    // Cmd/Ctrl+Shift+R — reveal in folder
    if (mod && e.shiftKey && (e.key === "r" || e.key === "R")) {
      e.preventDefault();
      EventBus.emit("shortcut:reveal-in-folder");
      return;
    }

    // Cmd/Ctrl+Shift+U — USB export
    if (mod && e.shiftKey && (e.key === "u" || e.key === "U")) {
      e.preventDefault();
      EventBus.emit("shortcut:usb-export");
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
        case "k":
        case "K":
          // Audit Wave 5 (deferred from Wave 3 #8): wire the README-promised
          // Ctrl+K — opens AI analyse for the selected file.
          e.preventDefault();
          EventBus.emit("shortcut:ai");
          return;
        case "n":
        case "N":
          // Audit Wave 5: Ctrl+N → new folder.
          e.preventDefault();
          EventBus.emit("shortcut:new-folder");
          return;
        case "a":
        case "A":
          // Audit Wave 5: Ctrl+A → select all in file list.
          e.preventDefault();
          EventBus.emit("shortcut:select-all");
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
      case "?":
        // Audit Wave 5: ? → keyboard-shortcut help dialog.
        e.preventDefault();
        EventBus.emit("shortcut:help");
        break;
    }
  };
  document.addEventListener("keydown", handler);
  return () => document.removeEventListener("keydown", handler);
}
