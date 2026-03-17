import * as ProjectService from "../services/ProjectService";
import * as MfgService from "../services/ManufacturingService";
import * as ReportService from "../services/ReportService";
import * as SettingsService from "../services/SettingsService";
import { ToastContainer } from "./Toast";
import { trapFocus } from "../utils/focus-trap";
import type { Project, ProjectDetail, TimeEntry } from "../types";

export class ProjectListDialog {
  private static instance: ProjectListDialog | null = null;

  private overlay: HTMLElement | null = null;
  private projects: Project[] = [];
  private selectedProject: Project | null = null;
  private details: ProjectDetail[] = [];
  private timeEntries: TimeEntry[] = [];
  private statusFilter = "";
  private keyHandler: ((e: KeyboardEvent) => void) | null = null;
  private releaseFocusTrap: (() => void) | null = null;
  private fieldIdCounter = 0;
  private laborRate = 25.0;

  static async open(): Promise<void> {
    if (ProjectListDialog.instance) {
      ProjectListDialog.dismiss();
    }
    const dialog = new ProjectListDialog();
    ProjectListDialog.instance = dialog;
    await dialog.init();
  }

  static dismiss(): void {
    if (ProjectListDialog.instance) {
      ProjectListDialog.instance.close();
      ProjectListDialog.instance = null;
    }
  }

  private async init(): Promise<void> {
    await this.loadProjects();
    // Load labor rate from settings
    try {
      const settings = await SettingsService.getAllSettings();
      if (settings.labor_rate_per_hour) {
        this.laborRate = Number(settings.labor_rate_per_hour) || 25.0;
      }
    } catch {
      // use default
    }
    this.overlay = this.buildUI();
    document.body.appendChild(this.overlay);
    const dialog = this.overlay.querySelector<HTMLElement>(".dialog") || this.overlay;
    this.releaseFocusTrap = trapFocus(dialog);

    this.keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") { e.stopImmediatePropagation(); ProjectListDialog.dismiss(); }
    };
    document.addEventListener("keydown", this.keyHandler);
  }

  private async loadProjects(): Promise<void> {
    this.projects = await ProjectService.getProjects(
      this.statusFilter || undefined
    );
  }

  private nextFieldId(): string {
    return `pl-f-${++this.fieldIdCounter}`;
  }

  private buildUI(): HTMLElement {
    const overlay = document.createElement("div");
    overlay.className = "project-list-overlay";

    const dialog = document.createElement("div");
    dialog.className = "project-list-dialog";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Projektuebersicht");

    // Header
    const header = document.createElement("div");
    header.className = "pl-header";

    const title = document.createElement("h2");
    title.className = "pl-title";
    title.textContent = "Projekte";
    header.appendChild(title);

    // Status filter
    const filter = document.createElement("select");
    filter.className = "pp-setting-select";
    filter.innerHTML =
      '<option value="">Alle</option>' +
      '<option value="not_started">Nicht begonnen</option>' +
      '<option value="planned">Geplant</option>' +
      '<option value="in_progress">In Arbeit</option>' +
      '<option value="completed">Abgeschlossen</option>' +
      '<option value="archived">Archiviert</option>';
    filter.value = this.statusFilter;
    filter.addEventListener("change", async () => {
      this.statusFilter = filter.value;
      try {
        await this.loadProjects();
        this.renderList();
      } catch {
        ToastContainer.show("error", "Projekte konnten nicht geladen werden");
      }
    });
    header.appendChild(filter);

    const closeBtn = document.createElement("button");
    closeBtn.className = "dv-close-btn";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Schliessen");
    closeBtn.addEventListener("click", () => ProjectListDialog.dismiss());
    header.appendChild(closeBtn);

    dialog.appendChild(header);

    // Status dashboard
    const dashboard = document.createElement("div");
    dashboard.className = "pl-dashboard";
    this.renderDashboard(dashboard);
    dialog.appendChild(dashboard);

    // Content: list + detail
    const content = document.createElement("div");
    content.className = "pl-content";

    const listPane = document.createElement("div");
    listPane.className = "pl-list-pane";
    listPane.dataset.id = "pl-list";
    this.renderListInto(listPane);
    content.appendChild(listPane);

    const detailPane = document.createElement("div");
    detailPane.className = "pl-detail-pane";
    detailPane.dataset.id = "pl-detail";
    detailPane.textContent = "Projekt auswaehlen";
    content.appendChild(detailPane);

    dialog.appendChild(content);
    overlay.appendChild(dialog);
    return overlay;
  }

  private renderDashboard(container: HTMLElement): void {
    container.innerHTML = "";
    const statusCounts: Record<string, number> = {};
    for (const p of this.projects) {
      statusCounts[p.status] = (statusCounts[p.status] || 0) + 1;
    }
    const labels: Record<string, string> = {
      not_started: "Nicht begonnen",
      planned: "Geplant",
      in_progress: "In Arbeit",
      completed: "Abgeschlossen",
      archived: "Archiviert",
    };
    for (const [status, label] of Object.entries(labels)) {
      const count = statusCounts[status] || 0;
      const badge = document.createElement("span");
      badge.className = `pl-status-badge status-${status}`;
      badge.textContent = `${label}: ${count}`;
      container.appendChild(badge);
    }
  }

  private renderList(): void {
    const pane = this.overlay?.querySelector<HTMLElement>('[data-id="pl-list"]');
    if (pane) this.renderListInto(pane);
    const dashboard = this.overlay?.querySelector<HTMLElement>(".pl-dashboard");
    if (dashboard) this.renderDashboard(dashboard);
  }

  private renderListInto(container: HTMLElement): void {
    container.innerHTML = "";
    if (this.projects.length === 0) {
      container.textContent = "Keine Projekte";
      return;
    }

    for (const project of this.projects) {
      const item = document.createElement("div");
      item.className = "pl-item";
      if (this.selectedProject?.id === project.id) item.classList.add("selected");

      const name = document.createElement("span");
      name.className = "pl-item-name";
      name.textContent = project.name;
      item.appendChild(name);

      const status = document.createElement("span");
      status.className = `metadata-project-status status-${project.status}`;
      const statusLabels: Record<string, string> = {
        not_started: "Nicht begonnen", planned: "Geplant",
        in_progress: "In Arbeit", completed: "Abgeschlossen", archived: "Archiviert",
      };
      status.textContent = statusLabels[project.status] || project.status;
      item.appendChild(status);

      item.addEventListener("click", () => this.selectProject(project));
      container.appendChild(item);
    }
  }

  private async selectProject(project: Project): Promise<void> {
    this.selectedProject = project;
    this.details = [];
    this.timeEntries = [];
    try {
      const [details, timeEntries] = await Promise.all([
        ProjectService.getProjectDetails(project.id),
        MfgService.getTimeEntries(project.id).catch(() => [] as TimeEntry[]),
      ]);
      this.details = details;
      this.timeEntries = timeEntries;
    } catch {
      ToastContainer.show("error", "Projektdaten konnten nicht geladen werden");
    }
    this.renderList();
    this.renderDetail();
  }

  private renderDetail(): void {
    const pane = this.overlay?.querySelector<HTMLElement>('[data-id="pl-detail"]');
    if (!pane || !this.selectedProject) return;
    pane.innerHTML = "";

    const p = this.selectedProject;

    // Name
    const nameGroup = this.createField("Name", p.name, async (val) => {
      try {
        await ProjectService.updateProject(p.id, { name: val });
        await this.loadProjects();
        this.selectedProject = this.projects.find((pr) => pr.id === p.id) || null;
        this.renderList();
      } catch {
        ToastContainer.show("error", "Name konnte nicht gespeichert werden");
      }
    });
    pane.appendChild(nameGroup);

    // Status
    const statusId = this.nextFieldId();
    const statusGroup = document.createElement("div");
    statusGroup.className = "pl-field";
    const statusLabel = document.createElement("label");
    statusLabel.className = "pp-setting-label";
    statusLabel.textContent = "Status";
    statusLabel.htmlFor = statusId;
    const statusSelect = document.createElement("select");
    statusSelect.id = statusId;
    statusSelect.className = "pp-setting-select";
    statusSelect.innerHTML =
      '<option value="not_started">Nicht begonnen</option>' +
      '<option value="planned">Geplant</option>' +
      '<option value="in_progress">In Arbeit</option>' +
      '<option value="completed">Abgeschlossen</option>' +
      '<option value="archived">Archiviert</option>';
    statusSelect.value = p.status;
    statusSelect.addEventListener("change", async () => {
      try {
        await ProjectService.updateProject(p.id, { status: statusSelect.value });
        await this.loadProjects();
        this.selectedProject = this.projects.find((pr) => pr.id === p.id) || null;
        this.renderList();
      } catch {
        ToastContainer.show("error", "Status konnte nicht gespeichert werden");
      }
    });
    statusGroup.appendChild(statusLabel);
    statusGroup.appendChild(statusSelect);
    pane.appendChild(statusGroup);

    // Notes
    const notesId = this.nextFieldId();
    const notesGroup = document.createElement("div");
    notesGroup.className = "pl-field";
    const notesLabel = document.createElement("label");
    notesLabel.className = "pp-setting-label";
    notesLabel.textContent = "Notizen";
    notesLabel.htmlFor = notesId;
    const notesArea = document.createElement("textarea");
    notesArea.id = notesId;
    notesArea.className = "dv-note-text";
    notesArea.value = p.notes || "";
    notesArea.rows = 4;
    const notesSave = document.createElement("button");
    notesSave.className = "dv-btn";
    notesSave.textContent = "Speichern";
    notesSave.addEventListener("click", async () => {
      try {
        await ProjectService.updateProject(p.id, { notes: notesArea.value });
        ToastContainer.show("success", "Notizen gespeichert");
      } catch {
        ToastContainer.show("error", "Notizen konnten nicht gespeichert werden");
      }
    });
    notesGroup.appendChild(notesLabel);
    notesGroup.appendChild(notesArea);
    notesGroup.appendChild(notesSave);
    pane.appendChild(notesGroup);

    // Project details (key-value)
    const detailsHeader = document.createElement("h4");
    detailsHeader.className = "pp-settings-title";
    detailsHeader.textContent = "Projektdetails";
    pane.appendChild(detailsHeader);

    const knownFields = [
      { key: "chosen_size", label: "Gewaehlte Groesse" },
      { key: "fabric_used", label: "Stoff" },
      { key: "planned_modifications", label: "Geplante Aenderungen" },
      { key: "cut_version", label: "Schnittversion" },
    ];

    for (const field of knownFields) {
      const existing = this.details.find((d) => d.key === field.key);
      const fieldGroup = this.createField(
        field.label,
        existing?.value || "",
        async (val) => {
          try {
            await ProjectService.setProjectDetails(p.id, [
              { key: field.key, value: val || null },
            ]);
          } catch {
            ToastContainer.show("error", "Detail konnte nicht gespeichert werden");
          }
        }
      );
      pane.appendChild(fieldGroup);
    }

    // Time & Cost section
    if (this.timeEntries.length > 0) {
      const tcHeader = document.createElement("h4");
      tcHeader.className = "pp-settings-title";
      tcHeader.textContent = "Zeit & Kosten";
      pane.appendChild(tcHeader);

      const tcTable = document.createElement("table");
      tcTable.className = "pl-tc-table";
      const thead = document.createElement("thead");
      const headRow = document.createElement("tr");
      for (const h of ["Schritt", "Geplant", "Tatsaechlich", "Differenz"]) {
        const th = document.createElement("th");
        th.textContent = h;
        headRow.appendChild(th);
      }
      thead.appendChild(headRow);
      tcTable.appendChild(thead);

      const tbody = document.createElement("tbody");
      let totalPlanned = 0;
      let totalActual = 0;
      for (const e of this.timeEntries) {
        const planned = e.plannedMinutes ?? 0;
        const actual = e.actualMinutes ?? 0;
        const diff = actual - planned;
        totalPlanned += planned;
        totalActual += actual;

        const tr = document.createElement("tr");
        const tdStep = document.createElement("td");
        tdStep.textContent = e.stepName;
        const tdPlanned = document.createElement("td");
        tdPlanned.textContent = this.fmtMinutes(planned);
        const tdActual = document.createElement("td");
        tdActual.textContent = this.fmtMinutes(actual);
        const tdDiff = document.createElement("td");
        tdDiff.className = diff > 0 ? "pl-tc-over" : diff < 0 ? "pl-tc-under" : "";
        tdDiff.textContent =
          diff > 0
            ? `+${this.fmtMinutes(diff)}`
            : diff < 0
              ? `-${this.fmtMinutes(Math.abs(diff))}`
              : "-";
        tr.appendChild(tdStep);
        tr.appendChild(tdPlanned);
        tr.appendChild(tdActual);
        tr.appendChild(tdDiff);
        tbody.appendChild(tr);
      }

      // Totals row
      const totalDiff = totalActual - totalPlanned;
      const tfoot = document.createElement("tfoot");
      const totalRow = document.createElement("tr");
      const tdLabel = document.createElement("td");
      tdLabel.textContent = "Gesamt";
      const tdTotalP = document.createElement("td");
      tdTotalP.textContent = this.fmtMinutes(totalPlanned);
      const tdTotalA = document.createElement("td");
      tdTotalA.textContent = this.fmtMinutes(totalActual);
      const tdTotalD = document.createElement("td");
      tdTotalD.className =
        totalDiff > 0 ? "pl-tc-over" : totalDiff < 0 ? "pl-tc-under" : "";
      tdTotalD.textContent =
        totalDiff > 0
          ? `+${this.fmtMinutes(totalDiff)}`
          : totalDiff < 0
            ? `-${this.fmtMinutes(Math.abs(totalDiff))}`
            : "-";
      totalRow.appendChild(tdLabel);
      totalRow.appendChild(tdTotalP);
      totalRow.appendChild(tdTotalA);
      totalRow.appendChild(tdTotalD);
      tfoot.appendChild(totalRow);

      tcTable.appendChild(tbody);
      tcTable.appendChild(tfoot);
      pane.appendChild(tcTable);

      // Labor cost estimate
      const laborCost = (totalActual / 60) * this.laborRate;
      const costInfo = document.createElement("div");
      costInfo.className = "pl-tc-cost";
      costInfo.textContent = `Arbeitskosten (${this.laborRate.toFixed(2)} EUR/h): ${laborCost.toFixed(2)} EUR`;
      pane.appendChild(costInfo);
    }

    // Actions
    const actions = document.createElement("div");
    actions.className = "pl-actions";

    const dupBtn = document.createElement("button");
    dupBtn.className = "dv-btn";
    dupBtn.textContent = "Duplizieren";
    dupBtn.addEventListener("click", async () => {
      try {
        await ProjectService.duplicateProject(p.id);
        await this.loadProjects();
        this.renderList();
        ToastContainer.show("success", "Projekt dupliziert");
      } catch {
        ToastContainer.show("error", "Duplizieren fehlgeschlagen");
      }
    });
    actions.appendChild(dupBtn);

    const delBtn = document.createElement("button");
    delBtn.className = "dv-btn dv-note-delete";
    delBtn.textContent = "Loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm(`Projekt "${p.name}" wirklich loeschen?`)) return;
      try {
        await ProjectService.deleteProject(p.id);
        this.selectedProject = null;
        await this.loadProjects();
        this.renderList();
        const detailP = this.overlay?.querySelector<HTMLElement>('[data-id="pl-detail"]');
        if (detailP) detailP.textContent = "Projekt auswaehlen";
      } catch {
        ToastContainer.show("error", "Loeschen fehlgeschlagen");
      }
    });
    actions.appendChild(delBtn);

    pane.appendChild(actions);

    // Audit history
    const auditSection = document.createElement("div");
    auditSection.style.marginTop = "12px";
    const auditBtn = document.createElement("button");
    auditBtn.className = "dv-btn";
    auditBtn.style.fontSize = "0.85em";
    auditBtn.textContent = "Aenderungshistorie";
    auditBtn.addEventListener("click", async () => {
      auditBtn.style.display = "none";
      try {
        const entries = await ReportService.getAuditLog("project", p.id);
        if (entries.length === 0) {
          const hint = document.createElement("div");
          hint.style.fontSize = "0.85em";
          hint.style.opacity = "0.6";
          hint.textContent = "Keine Aenderungen protokolliert";
          auditSection.appendChild(hint);
          return;
        }
        const table = document.createElement("table");
        table.className = "pl-tc-table";
        table.style.fontSize = "0.85em";
        table.innerHTML = "<thead><tr><th>Feld</th><th>Alt</th><th>Neu</th><th>Datum</th></tr></thead>";
        const tbody = document.createElement("tbody");
        for (const e of entries) {
          const tr = document.createElement("tr");
          for (const cell of [e.fieldName, e.oldValue || "-", e.newValue || "-", e.changedAt.substring(0, 16)]) {
            const td = document.createElement("td");
            td.textContent = cell;
            tr.appendChild(td);
          }
          tbody.appendChild(tr);
        }
        table.appendChild(tbody);
        auditSection.appendChild(table);
      } catch { ToastContainer.show("error", "Historie konnte nicht geladen werden"); }
    });
    auditSection.appendChild(auditBtn);
    pane.appendChild(auditSection);
  }

  private createField(
    label: string,
    value: string,
    onSave: (val: string) => Promise<void>
  ): HTMLElement {
    const id = this.nextFieldId();
    const group = document.createElement("div");
    group.className = "pl-field";
    const lbl = document.createElement("label");
    lbl.className = "pp-setting-label";
    lbl.textContent = label;
    lbl.htmlFor = id;
    const input = document.createElement("input");
    input.id = id;
    input.className = "pp-setting-input";
    input.value = value;
    input.addEventListener("change", () => onSave(input.value));
    group.appendChild(lbl);
    group.appendChild(input);
    return group;
  }

  private fmtMinutes(minutes: number): string {
    if (minutes < 60) return `${Math.round(minutes)}min`;
    const h = Math.floor(minutes / 60);
    const m = Math.round(minutes % 60);
    return m > 0 ? `${h}h ${m}min` : `${h}h`;
  }

  private close(): void {
    if (this.releaseFocusTrap) {
      this.releaseFocusTrap();
      this.releaseFocusTrap = null;
    }
    if (this.keyHandler) {
      document.removeEventListener("keydown", this.keyHandler);
      this.keyHandler = null;
    }
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }
}
