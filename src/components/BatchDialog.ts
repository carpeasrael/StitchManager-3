import { EventBus } from "../state/EventBus";

export class BatchDialog {
  private overlay: HTMLElement | null = null;
  private progressFill: HTMLElement | null = null;
  private progressText: HTMLElement | null = null;
  private logContainer: HTMLElement | null = null;
  private stepLabel: HTMLElement | null = null;
  private cancelBtn: HTMLButtonElement | null = null;
  private total: number;
  private unsubscribe: (() => void) | null = null;

  constructor(
    private operation: string,
    total: number
  ) {
    this.total = total;
  }

  static open(operation: string, total: number): BatchDialog {
    const dialog = new BatchDialog(operation, total);
    dialog.show();
    return dialog;
  }

  private show(): void {
    this.overlay = document.createElement("div");
    this.overlay.className = "dialog-overlay";
    this.overlay.addEventListener("dialog-dismiss", () => this.close());

    const dialog = document.createElement("div");
    dialog.className = "dialog dialog-batch";

    // Header
    const header = document.createElement("div");
    header.className = "dialog-header";
    header.innerHTML = `<span class="dialog-title">${this.escapeHtml(this.operation)}</span>`;
    dialog.appendChild(header);

    // Body
    const body = document.createElement("div");
    body.className = "dialog-body";

    // Step indicator
    this.stepLabel = document.createElement("div");
    this.stepLabel.className = "batch-step-label";
    this.stepLabel.textContent = `${this.operation} — 0 von ${this.total} Dateien`;
    body.appendChild(this.stepLabel);

    // Progress bar
    const progressBar = document.createElement("div");
    progressBar.className = "batch-progress-bar";

    this.progressFill = document.createElement("div");
    this.progressFill.className = "batch-progress-fill";
    this.progressFill.style.width = "0%";
    progressBar.appendChild(this.progressFill);

    body.appendChild(progressBar);

    this.progressText = document.createElement("div");
    this.progressText.className = "batch-progress-text";
    this.progressText.textContent = "0 / " + this.total;
    body.appendChild(this.progressText);

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
    this.cancelBtn.textContent = "Schliessen";
    this.cancelBtn.addEventListener("click", () => {
      this.close();
    });
    footer.appendChild(this.cancelBtn);

    dialog.appendChild(footer);

    this.overlay.appendChild(dialog);
    document.body.appendChild(this.overlay);

    // Listen for batch:progress events
    this.unsubscribe = EventBus.on("batch:progress", (payload: unknown) => {
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
    });
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
      this.progressFill.style.width = `${pct}%`;
    }
    if (this.progressText) {
      this.progressText.textContent = `${current} / ${total}`;
    }
    if (this.stepLabel) {
      this.stepLabel.textContent = `${this.operation} — ${current} von ${total} Dateien`;
    }

    // Auto-close on completion
    if (current >= total) {
      this.onComplete();
    }
  }

  private onComplete(): void {
    if (this.cancelBtn) {
      this.cancelBtn.textContent = "Schliessen";
      this.cancelBtn.disabled = false;
      this.cancelBtn.onclick = () => this.close();
    }

    // Auto-close after 2 seconds
    setTimeout(() => {
      this.close();
    }, 2000);
  }

  close(): void {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }

  private escapeHtml(text: string): string {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }
}
