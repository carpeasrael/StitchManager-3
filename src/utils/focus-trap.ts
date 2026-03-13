const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function trapFocus(dialogEl: HTMLElement): () => void {
  const previousFocus = document.activeElement as HTMLElement | null;

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== "Tab") return;

    const focusable = Array.from(dialogEl.querySelectorAll<HTMLElement>(FOCUSABLE));
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey) {
      if (document.activeElement === first) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  };

  dialogEl.addEventListener("keydown", onKeyDown);

  // Focus the first focusable element in the dialog
  const firstFocusable = dialogEl.querySelector<HTMLElement>(FOCUSABLE);
  if (firstFocusable) {
    requestAnimationFrame(() => firstFocusable.focus());
  }

  return () => {
    dialogEl.removeEventListener("keydown", onKeyDown);
    if (previousFocus && previousFocus.focus) {
      previousFocus.focus();
    }
  };
}
