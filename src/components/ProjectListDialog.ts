import * as ProjectService from "../services/ProjectService";
import { ToastContainer } from "./Toast";
import type { Project, ProjectDetail } from "../types";

export class ProjectListDialog {
  private static instance: ProjectListDialog | null = null;

  private overlay: HTMLElement | null = null;
  private projects: Project[] = [];
  private selectedProject: Project | null = null;
  private details: ProjectDetail[] = [];
  private statusFilter = "";
  private keyHandler: ((e: KeyboardEvent) => void) | null = null;

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
    this.overlay = this.buildUI();
    document.body.appendChild(this.overlay);

    this.keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") ProjectListDialog.dismiss();
    };
    document.addEventListener("keydown", this.keyHandler);
  }

  private async loadProjects(): Promise<void> {
    this.projects = await ProjectService.getProjects(
      this.statusFilter || undefined
    );
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
      await this.loadProjects();
      this.renderList();
    });
    header.appendChild(filter);

    const closeBtn = document.createElement("button");
    closeBtn.className = "dv-close-btn";
    closeBtn.textContent = "\u00D7";
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
    this.details = await ProjectService.getProjectDetails(project.id);
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
      await ProjectService.updateProject(p.id, { name: val });
      await this.loadProjects();
      this.selectedProject = this.projects.find((pr) => pr.id === p.id) || null;
      this.renderList();
    });
    pane.appendChild(nameGroup);

    // Status
    const statusGroup = document.createElement("div");
    statusGroup.className = "pl-field";
    const statusLabel = document.createElement("label");
    statusLabel.className = "pp-setting-label";
    statusLabel.textContent = "Status";
    const statusSelect = document.createElement("select");
    statusSelect.className = "pp-setting-select";
    statusSelect.innerHTML =
      '<option value="not_started">Nicht begonnen</option>' +
      '<option value="planned">Geplant</option>' +
      '<option value="in_progress">In Arbeit</option>' +
      '<option value="completed">Abgeschlossen</option>' +
      '<option value="archived">Archiviert</option>';
    statusSelect.value = p.status;
    statusSelect.addEventListener("change", async () => {
      await ProjectService.updateProject(p.id, { status: statusSelect.value });
      await this.loadProjects();
      this.selectedProject = this.projects.find((pr) => pr.id === p.id) || null;
      this.renderList();
    });
    statusGroup.appendChild(statusLabel);
    statusGroup.appendChild(statusSelect);
    pane.appendChild(statusGroup);

    // Notes
    const notesGroup = document.createElement("div");
    notesGroup.className = "pl-field";
    const notesLabel = document.createElement("label");
    notesLabel.className = "pp-setting-label";
    notesLabel.textContent = "Notizen";
    const notesArea = document.createElement("textarea");
    notesArea.className = "dv-note-text";
    notesArea.value = p.notes || "";
    notesArea.rows = 4;
    const notesSave = document.createElement("button");
    notesSave.className = "dv-btn";
    notesSave.textContent = "Speichern";
    notesSave.addEventListener("click", async () => {
      await ProjectService.updateProject(p.id, { notes: notesArea.value });
      ToastContainer.show("success", "Notizen gespeichert");
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
          await ProjectService.setProjectDetails(p.id, [
            { key: field.key, value: val || null },
          ]);
        }
      );
      pane.appendChild(fieldGroup);
    }

    // Actions
    const actions = document.createElement("div");
    actions.className = "pl-actions";

    const dupBtn = document.createElement("button");
    dupBtn.className = "dv-btn";
    dupBtn.textContent = "Duplizieren";
    dupBtn.addEventListener("click", async () => {
      await ProjectService.duplicateProject(p.id);
      await this.loadProjects();
      this.renderList();
      ToastContainer.show("success", "Projekt dupliziert");
    });
    actions.appendChild(dupBtn);

    const delBtn = document.createElement("button");
    delBtn.className = "dv-btn dv-note-delete";
    delBtn.textContent = "Loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm(`Projekt "${p.name}" wirklich loeschen?`)) return;
      await ProjectService.deleteProject(p.id);
      this.selectedProject = null;
      await this.loadProjects();
      this.renderList();
      const detailP = this.overlay?.querySelector<HTMLElement>('[data-id="pl-detail"]');
      if (detailP) detailP.textContent = "Projekt auswaehlen";
    });
    actions.appendChild(delBtn);

    pane.appendChild(actions);
  }

  private createField(
    label: string,
    value: string,
    onSave: (val: string) => Promise<void>
  ): HTMLElement {
    const group = document.createElement("div");
    group.className = "pl-field";
    const lbl = document.createElement("label");
    lbl.className = "pp-setting-label";
    lbl.textContent = label;
    const input = document.createElement("input");
    input.className = "pp-setting-input";
    input.value = value;
    input.addEventListener("change", () => onSave(input.value));
    group.appendChild(lbl);
    group.appendChild(input);
    return group;
  }

  private close(): void {
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
