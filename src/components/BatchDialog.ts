import { EventBus } from "../state/EventBus";
import { trapFocus } from "../utils/focus-trap";
import { escapeHtml } from "../utils/escape";
import type { ImportProgress } from "../types/index";

function formatTime(ms: number): string {
  const totalSec = Math.round(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

export type BatchMode = "batch" | "import";

export class BatchDialog {
  private overlay: HTMLElement | null = null;
  private progressFill: HTMLElement | null = null;
  private progressText: HTMLElement | null = null;
  private logContainer: HTMLElement | null = null;
  private stepLabel: HTMLElement | null = null;
  private timeLabel: HTMLElement | null = null;
  private cancelBtn: HTMLButtonElement | null = null;
  private total: number;
  private mode: BatchMode;
  private unsubscribers: (() => void)[] = [];
  private releaseFocusTrap: (() => void) | null = null;
  private autoCloseTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private operation: string,
    total: number,
    mode: BatchMode = "batch"
  ) {
    this.total = total;
    this.mode = mode;
  }

  static open(operation: string, total: number, mode: BatchMode = "batch"): BatchDialog {
    const dialog = new BatchDialog(operation, total, mode);
    dialog.show();
    return dialog;
  }

  private show(): void {
    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("dialog-dismiss", () => this.close());

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-batch";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", this.operation);

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";
    header.innerHTML = `<span class="dialog-title">${escapeHtml(this.operation)}</span>`;
    dialog.appendChild(header);

    // Body
    const body = document.createElement("div");
    body.className = "dialog-body";

    // Step indicator
    this.stepLabel = document.createElement("div");
    this.stepLabel.className = "batch-step-label";
    if (this.mode === "import" && this.total === 0) {
      this.stepLabel.textContent = `${this.operation} — Dateien werden gesucht...`;
    } else {
      this.stepLabel.textContent = `${this.operation} — 0 von ${this.total} Dateien`;
    }
    body.appendChild(this.stepLabel);

    // Progress bar
    const progressBar = document.createElement("div");
    progressBar.className = "batch-progress-bar";

    this.progressFill = document.createElement("div");
    this.progressFill.className = "batch-progress-fill";
    if (this.mode === "import" && this.total === 0) {
      this.progressFill.classList.add("batch-progress-indeterminate");
    }
    this.progressFill.style.width = this.mode === "import" && this.total === 0 ? "100%" : "0%";
    progressBar.appendChild(this.progressFill);

    progressBar.setAttribute("role", "progressbar");
    progressBar.setAttribute("aria-valuemin", "0");
    progressBar.setAttribute("aria-valuemax", "100");
    if (!(this.mode === "import" && this.total === 0)) {
      progressBar.setAttribute("aria-valuenow", "0");
    }

    body.appendChild(progressBar);

    this.progressText = document.createElement("div");
    this.progressText.className = "batch-progress-text";
    this.progressText.textContent = this.mode === "import" && this.total === 0
      ? "Suche..."
      : "0 / " + this.total;
    body.appendChild(this.progressText);

    // Time display (import mode only)
    if (this.mode === "import") {
      this.timeLabel = document.createElement("div");
      this.timeLabel.className = "batch-time-label";
      this.timeLabel.textContent = "";
      body.appendChild(this.timeLabel);
    }

    // Log view
    this.logContainer = document.createElement("div");
    this.logContainer.className = "batch-log";
    body.appendChild(this.logContainer);

    dialog.appendChild(body);

    // Footer
    const footer = document.createElement("div");
    footer.className = "dialog-footer";

    this.cancelBtn = document.createElement("button");
    this.cancelBtn.className = "dialog-btn dialog-btn-secondary";
    this.cancelBtn.textContent = "Schließen";
    this.cancelBtn.addEventListener("click", () => {
      this.close();
    });
    footer.appendChild(this.cancelBtn);

    dialog.appendChild(footer);

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);
    this.releaseFocusTrap = trapFocus(dialog);

    // Listen for progress events
    if (this.mode === "import") {
      this.unsubscribers.push(
        EventBus.on("import:discovery", (payload: unknown) => {
          const p = payload as { scannedFiles: number; foundFiles: number };
          this.updateDiscovery(p.scannedFiles, p.foundFiles);
        })
      );
      this.unsubscribers.push(
        EventBus.on("import:progress", (payload: unknown) => {
          const p = payload as ImportProgress;
          const isError = p.status.startsWith("error");
          const isSkipped = p.status === "skipped";
          if (!isSkipped) {
            this.addLogEntry(
              p.filename,
              isError ? "error" : "success",
              isError ? p.status : undefined
            );
          }
          this.setProgress(p.current, p.total);
          this.updateTime(p.elapsedMs, p.estimatedRemainingMs);
        })
      );
    } else {
      this.unsubscribers.push(
        EventBus.on("batch:progress", (payload: unknown) => {
          const p = payload as {
            current: number;
            total: number;
            filename: string;
            status: string;
          };
          const isError = p.status.startsWith("error");
          this.addLogEntry(
            p.filename,
            isError ? "error" : "success",
            isError ? p.status : undefined
          );
          this.setProgress(p.current, p.total);
        })
      );
    }
  }

  private updateDiscovery(scannedFiles: number, foundFiles: number): void {
    if (this.stepLabel) {
      this.stepLabel.textContent = `${this.operation} — ${foundFiles} Stickdateien gefunden (${scannedFiles} Dateien durchsucht)`;
    }
  }

  private updateTime(elapsedMs: number, estimatedRemainingMs: number): void {
    if (!this.timeLabel) return;
    const elapsed = formatTime(elapsedMs);
    const remaining = formatTime(estimatedRemainingMs);
    this.timeLabel.textContent = `Laufzeit: ${elapsed} — Verbleibend: ~${remaining}`;
  }

  addLogEntry(
    filename: string,
    status: "success" | "error",
    message?: string
  ): void {
    if (!this.logContainer) return;

    const entry = document.createElement("div");
    entry.className = `batch-log-entry batch-log-${status}`;

    const icon = document.createElement("span");
    icon.className = "batch-log-icon";
    icon.textContent = status === "success" ? "\u2713" : "\u2717";
    entry.appendChild(icon);

    const text = document.createElement("span");
    text.className = "batch-log-text";
    text.textContent = message ? `${filename} — ${message}` : filename;
    entry.appendChild(text);

    this.logContainer.appendChild(entry);
    this.logContainer.scrollTop = this.logContainer.scrollHeight;
  }

  setProgress(current: number, total: number): void {
    this.total = total;
    const pct = total > 0 ? Math.round((current / total) * 100) : 0;

    if (this.progressFill) {
      this.progressFill.classList.remove("batch-progress-indeterminate");
      this.progressFill.style.width = `${pct}%`;
      const bar = this.progressFill.parentElement;
      if (bar) bar.setAttribute("aria-valuenow", String(pct));
    }
    if (this.progressText) {
      this.progressText.textContent = `${current} / ${total}`;
    }
    if (this.stepLabel) {
      this.stepLabel.textContent = `${this.operation} — ${current} von ${total} Dateien`;
    }

    // Auto-close on completion
    if (current >= total && total > 0) {
      this.onComplete();
    }
  }

  private onComplete(): void {
    if (this.cancelBtn) {
      this.cancelBtn.textContent = "Schließen";
      this.cancelBtn.disabled = false;
    }

    // Show final time
    if (this.timeLabel) {
      const text = this.timeLabel.textContent || "";
      const elapsed = text.split(" — ")[0] || "";
      this.timeLabel.textContent = elapsed ? `${elapsed} — Abgeschlossen` : "Abgeschlossen";
    }

    // Auto-close after 2 seconds
    this.autoCloseTimer = setTimeout(() => {
      this.close();
    }, 2000);
  }

  close(): void {
    if (this.autoCloseTimer) {
      clearTimeout(this.autoCloseTimer);
      this.autoCloseTimer = null;
    }
    for (const unsub of this.unsubscribers) {
      unsub();
    }
    this.unsubscribers = [];
    if (this.releaseFocusTrap) {
      this.releaseFocusTrap();
      this.releaseFocusTrap = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }

}
