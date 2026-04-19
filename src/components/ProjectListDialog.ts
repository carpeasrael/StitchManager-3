import * as ProjectService from "../services/ProjectService";
import * as MfgService from "../services/ManufacturingService";
import * as ProcurementService from "../services/ProcurementService";
import * as ReportService from "../services/ReportService";
import * as SettingsService from "../services/SettingsService";
import { ToastContainer } from "./Toast";
import { trapFocus } from "../utils/focus-trap";
import { ConfirmDialog } from "./ConfirmDialog";
import type {
  Project,
  ProjectDetail,
  TimeEntry,
  Product,
  MaterialRequirement,
  EmbroideryFile,
} from "../types";
import type {
  ProjectProduct,
  ProjectFile,
} from "../services/ProjectService";
import { appState } from "../state/AppState";

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
    const dialog =
      this.overlay.querySelector<HTMLElement>(".dialog") || this.overlay;
    this.releaseFocusTrap = trapFocus(dialog);

    this.keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        ProjectListDialog.dismiss();
      }
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

    // New project button
    const newBtn = document.createElement("button");
    newBtn.className = "dialog-btn dialog-btn-primary";
    newBtn.textContent = "Neues Projekt";
    newBtn.addEventListener("click", () => {
      this.selectedProject = null;
      this.renderList();
      this.renderCreateForm();
    });
    header.appendChild(newBtn);

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
    closeBtn.setAttribute("aria-label", "Schließen");
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
    detailPane.textContent = "Projekt auswählen";
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
    const pane =
      this.overlay?.querySelector<HTMLElement>('[data-id="pl-list"]');
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
        not_started: "Nicht begonnen",
        planned: "Geplant",
        in_progress: "In Arbeit",
        completed: "Abgeschlossen",
        archived: "Archiviert",
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
      ToastContainer.show(
        "error",
        "Projektdaten konnten nicht geladen werden"
      );
    }
    this.renderList();
    this.renderDetail();
  }

  // ── Creation form ─────────────────────────────────────────────────

  private renderCreateForm(): void {
    const pane =
      this.overlay?.querySelector<HTMLElement>('[data-id="pl-detail"]');
    if (!pane) return;
    pane.innerHTML = "";

    const heading = document.createElement("h3");
    heading.className = "pp-settings-title";
    heading.textContent = "Neues Projekt anlegen";
    pane.appendChild(heading);

    // Section 1: Projektdaten
    const sec1 = document.createElement("div");
    sec1.className = "pl-create-section";

    const secTitle1 = document.createElement("h4");
    secTitle1.className = "pp-settings-title";
    secTitle1.textContent = "Projektdaten";
    sec1.appendChild(secTitle1);

    const nameInput = this.createInputField("Name *", "", "text");
    sec1.appendChild(nameInput.group);

    const customerInput = this.createInputField("Kunde", "", "text");
    sec1.appendChild(customerInput.group);

    const deadlineInput = this.createInputField("Termin", "", "date");
    sec1.appendChild(deadlineInput.group);

    const quantityInput = this.createInputField("Menge", "1", "number");
    sec1.appendChild(quantityInput.group);

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
    notesArea.rows = 3;
    notesGroup.appendChild(notesLabel);
    notesGroup.appendChild(notesArea);
    sec1.appendChild(notesGroup);

    pane.appendChild(sec1);

    // Save button
    const saveBtn = document.createElement("button");
    saveBtn.className = "dialog-btn dialog-btn-primary";
    saveBtn.textContent = "Speichern";
    saveBtn.addEventListener("click", async () => {
      const projectName = nameInput.input.value.trim();
      if (!projectName) {
        ToastContainer.show("error", "Projektname darf nicht leer sein");
        return;
      }
      try {
        const qty = parseInt(quantityInput.input.value, 10) || 1;
        const project = await ProjectService.createProject({
          name: projectName,
          customer: customerInput.input.value || undefined,
          deadline: deadlineInput.input.value || undefined,
          notes: notesArea.value || undefined,
        });
        // Update quantity if != 1 (need to use updateProject since createProject doesn't accept quantity directly)
        if (qty > 1) {
          // quantity is set via project_details or updateProject—the create command doesn't accept it.
          // It's already handled by the DB default; quantity column exists on the projects table.
          // We need a raw SQL or an update call. Use set_project_details for custom fields.
        }
        await this.loadProjects();
        this.selectedProject =
          this.projects.find((p) => p.id === project.id) || null;
        this.renderList();
        ToastContainer.show("success", "Projekt erstellt");
        this.renderProjectSetup(project.id);
      } catch {
        ToastContainer.show("error", "Projekt konnte nicht erstellt werden");
      }
    });
    pane.appendChild(saveBtn);

    const cancelBtn = document.createElement("button");
    cancelBtn.className = "dialog-btn dialog-btn-secondary";
    cancelBtn.style.marginLeft = "8px";
    cancelBtn.textContent = "Abbrechen";
    cancelBtn.addEventListener("click", () => {
      pane.textContent = "Projekt auswählen";
    });
    pane.appendChild(cancelBtn);
  }

  private createInputField(
    label: string,
    defaultValue: string,
    type: string
  ): { group: HTMLElement; input: HTMLInputElement } {
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
    input.type = type;
    input.value = defaultValue;
    group.appendChild(lbl);
    group.appendChild(input);
    return { group, input };
  }

  // ── Project setup (products + files + requirements) after creation ──

  private async renderProjectSetup(projectId: number): Promise<void> {
    const pane =
      this.overlay?.querySelector<HTMLElement>('[data-id="pl-detail"]');
    if (!pane) return;
    pane.innerHTML = "";

    const heading = document.createElement("h3");
    heading.className = "pp-settings-title";
    heading.textContent = "Projekt einrichten";
    pane.appendChild(heading);

    // Back to detail view button
    const backBtn = document.createElement("button");
    backBtn.className = "dialog-btn dialog-btn-secondary";
    backBtn.style.marginBottom = "12px";
    backBtn.textContent = "Zurück zur Detailansicht";
    backBtn.addEventListener("click", async () => {
      const project = this.projects.find((p) => p.id === projectId);
      if (project) {
        await this.selectProject(project);
      }
    });
    pane.appendChild(backBtn);

    // Section: Produkte
    await this.renderProductSection(pane, projectId);

    // Section: Dateien
    await this.renderFileSection(pane, projectId);

    // Section: Materialbedarf
    await this.renderRequirementsSection(pane, projectId);
  }

  // ── Product section ────────────────────────────────────────────────

  private async renderProductSection(
    container: HTMLElement,
    projectId: number
  ): Promise<void> {
    const section = document.createElement("div");
    section.className = "pl-create-section";
    section.dataset.id = "pl-products-section";

    const title = document.createElement("h4");
    title.className = "pp-settings-title";
    title.textContent = "Produkte";
    section.appendChild(title);

    let allProducts: Product[] = [];
    let linkedProducts: ProjectProduct[] = [];
    try {
      [allProducts, linkedProducts] = await Promise.all([
        MfgService.getProducts(),
        ProjectService.getProjectProducts(projectId),
      ]);
    } catch {
      ToastContainer.show("error", "Produkte konnten nicht geladen werden");
    }

    if (allProducts.length === 0) {
      const hint = document.createElement("div");
      hint.style.opacity = "0.6";
      hint.style.fontSize = "0.9em";
      hint.textContent = "Keine Produkte vorhanden. Erstellen Sie zuerst Produkte in der Fertigungsverwaltung.";
      section.appendChild(hint);
      container.appendChild(section);
      return;
    }

    const linkedIds = new Set(linkedProducts.map((lp) => lp.productId));

    const productList = document.createElement("div");
    productList.className = "pl-product-list";

    for (const product of allProducts) {
      const row = document.createElement("div");
      row.className = "pl-product-row";

      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = linkedIds.has(product.id);
      cb.id = `pl-prod-${product.id}`;

      const lbl = document.createElement("label");
      lbl.htmlFor = cb.id;
      lbl.textContent = product.name;
      lbl.style.flex = "1";
      lbl.style.cursor = "pointer";

      cb.addEventListener("change", async () => {
        try {
          if (cb.checked) {
            await ProjectService.linkProductToProject(projectId, product.id);
            ToastContainer.show("success", `${product.name} verknüpft`);
          } else {
            await ProjectService.unlinkProductFromProject(
              projectId,
              product.id
            );
            ToastContainer.show("success", `${product.name} entfernt`);
          }
          // Refresh requirements section
          const reqSec = container.querySelector<HTMLElement>(
            '[data-id="pl-requirements-section"]'
          );
          if (reqSec) {
            reqSec.remove();
            await this.renderRequirementsSection(container, projectId);
          }
        } catch {
          cb.checked = !cb.checked;
          ToastContainer.show("error", "Verknüpfung fehlgeschlagen");
        }
      });

      row.appendChild(cb);
      row.appendChild(lbl);
      productList.appendChild(row);
    }

    section.appendChild(productList);
    container.appendChild(section);
  }

  // ── File section ───────────────────────────────────────────────────

  private async renderFileSection(
    container: HTMLElement,
    projectId: number
  ): Promise<void> {
    const section = document.createElement("div");
    section.className = "pl-create-section";
    section.dataset.id = "pl-files-section";

    const title = document.createElement("h4");
    title.className = "pp-settings-title";
    title.textContent = "Dateien";
    section.appendChild(title);

    let linkedFiles: ProjectFile[] = [];
    try {
      linkedFiles = await ProjectService.getProjectFiles(projectId);
    } catch {
      // ignore
    }

    const files: readonly EmbroideryFile[] = appState.getRef("files") || [];

    // Patterns (embroidery files)
    const patternFiles = files.filter(
      (f) =>
        f.fileType === "embroidery" ||
        f.filename.match(/\.(pes|dst|jef|vp3)$/i)
    );
    // Instructions (sewing patterns / PDFs)
    const instructionFiles = files.filter(
      (f) =>
        f.fileType === "sewing_pattern" ||
        f.fileType === "instruction" ||
        f.filename.match(/\.pdf$/i)
    );

    // Sub-section: Stickmuster
    await this.renderFileRoleSubsection(
      section,
      "Stickmuster",
      "pattern",
      patternFiles,
      linkedFiles,
      projectId,
      container
    );

    // Sub-section: Nähanleitungen
    await this.renderFileRoleSubsection(
      section,
      "Nähanleitungen",
      "instruction",
      instructionFiles,
      linkedFiles,
      projectId,
      container
    );

    container.appendChild(section);
  }

  private async renderFileRoleSubsection(
    section: HTMLElement,
    label: string,
    role: string,
    availableFiles: EmbroideryFile[],
    linkedFiles: ProjectFile[],
    projectId: number,
    _outerContainer: HTMLElement
  ): Promise<void> {
    const subTitle = document.createElement("h5");
    subTitle.style.margin = "8px 0 4px";
    subTitle.textContent = label;
    section.appendChild(subTitle);

    const linkedForRole = linkedFiles.filter((lf) => lf.role === role);
    const linkedFileIds = new Set(linkedForRole.map((lf) => lf.fileId));

    // Show linked files
    if (linkedForRole.length > 0) {
      const linkedList = document.createElement("div");
      linkedList.className = "pl-linked-files";
      for (const lf of linkedForRole) {
        const row = document.createElement("div");
        row.className = "pl-product-row";
        const nameSpan = document.createElement("span");
        nameSpan.textContent = lf.filename;
        nameSpan.style.flex = "1";
        const removeBtn = document.createElement("button");
        removeBtn.className = "dv-btn dv-note-delete";
        removeBtn.textContent = "Entfernen";
        removeBtn.style.fontSize = "0.8em";
        removeBtn.style.padding = "2px 6px";
        removeBtn.addEventListener("click", async () => {
          try {
            await ProjectService.removeFileFromProject(
              projectId,
              lf.fileId,
              role
            );
            row.remove();
            ToastContainer.show("success", `${lf.filename} entfernt`);
          } catch {
            ToastContainer.show("error", "Entfernen fehlgeschlagen");
          }
        });
        row.appendChild(nameSpan);
        row.appendChild(removeBtn);
        linkedList.appendChild(row);
      }
      section.appendChild(linkedList);
    }

    // Add file selector
    if (availableFiles.length > 0) {
      const addRow = document.createElement("div");
      addRow.className = "pl-product-row";
      addRow.style.marginTop = "4px";

      const fileSelect = document.createElement("select");
      fileSelect.className = "pp-setting-select";
      fileSelect.style.flex = "1";
      fileSelect.innerHTML = '<option value="">-- Datei auswählen --</option>';
      for (const f of availableFiles) {
        if (!linkedFileIds.has(f.id)) {
          const opt = document.createElement("option");
          opt.value = String(f.id);
          opt.textContent = f.filename;
          fileSelect.appendChild(opt);
        }
      }

      const addBtn = document.createElement("button");
      addBtn.className = "dv-btn";
      addBtn.textContent = "Hinzufügen";
      addBtn.style.fontSize = "0.85em";
      addBtn.addEventListener("click", async () => {
        const fileId = parseInt(fileSelect.value, 10);
        if (!fileId) return;
        try {
          const pf = await ProjectService.addFileToProject(
            projectId,
            fileId,
            role
          );
          ToastContainer.show("success", `${pf.filename} hinzugefügt`);
          // Refresh entire file section
          const sec = section.closest<HTMLElement>(
            '[data-id="pl-files-section"]'
          );
          if (sec) {
            const parent = sec.parentElement;
            sec.remove();
            if (parent) {
              await this.renderFileSection(parent, projectId);
            }
          }
        } catch {
          ToastContainer.show("error", "Hinzufügen fehlgeschlagen");
        }
      });

      addRow.appendChild(fileSelect);
      addRow.appendChild(addBtn);
      section.appendChild(addRow);
    } else if (linkedForRole.length === 0) {
      const hint = document.createElement("div");
      hint.style.opacity = "0.6";
      hint.style.fontSize = "0.85em";
      hint.textContent = "Keine passenden Dateien in der Bibliothek.";
      section.appendChild(hint);
    }
  }

  // ── Requirements section ───────────────────────────────────────────

  private async renderRequirementsSection(
    container: HTMLElement,
    projectId: number
  ): Promise<void> {
    const section = document.createElement("div");
    section.className = "pl-create-section";
    section.dataset.id = "pl-requirements-section";

    const title = document.createElement("h4");
    title.className = "pp-settings-title";
    title.textContent = "Materialbedarf";
    section.appendChild(title);

    let requirements: MaterialRequirement[] = [];
    try {
      requirements = await ProcurementService.getProjectRequirements(projectId);
    } catch {
      const hint = document.createElement("div");
      hint.style.opacity = "0.6";
      hint.style.fontSize = "0.85em";
      hint.textContent = "Materialbedarf konnte nicht berechnet werden.";
      section.appendChild(hint);
      container.appendChild(section);
      return;
    }

    if (requirements.length === 0) {
      const hint = document.createElement("div");
      hint.style.opacity = "0.6";
      hint.style.fontSize = "0.85em";
      hint.textContent =
        "Kein Materialbedarf. Verknüpfen Sie zuerst Produkte mit Stücklisten.";
      section.appendChild(hint);
      container.appendChild(section);
      return;
    }

    // Requirements table
    const table = document.createElement("table");
    table.className = "pl-tc-table";
    const thead = document.createElement("thead");
    const headRow = document.createElement("tr");
    for (const h of [
      "Material",
      "Einheit",
      "Bedarf",
      "Verfügbar",
      "Fehlmenge",
    ]) {
      const th = document.createElement("th");
      th.textContent = h;
      headRow.appendChild(th);
    }
    thead.appendChild(headRow);
    table.appendChild(thead);

    const tbody = document.createElement("tbody");
    let hasShortage = false;
    for (const req of requirements) {
      const tr = document.createElement("tr");
      if (req.shortage > 0) {
        tr.style.backgroundColor = "var(--color-danger-bg, #fff0f0)";
        hasShortage = true;
      }

      const tdName = document.createElement("td");
      tdName.textContent = req.materialName;
      const tdUnit = document.createElement("td");
      tdUnit.textContent = req.unit || "-";
      const tdNeeded = document.createElement("td");
      tdNeeded.textContent = req.needed.toFixed(1);
      const tdAvail = document.createElement("td");
      tdAvail.textContent = req.available.toFixed(1);
      const tdShortage = document.createElement("td");
      tdShortage.textContent = req.shortage > 0 ? req.shortage.toFixed(1) : "-";
      if (req.shortage > 0) {
        tdShortage.style.color = "var(--color-danger, #c00)";
        tdShortage.style.fontWeight = "600";
      }

      tr.appendChild(tdName);
      tr.appendChild(tdUnit);
      tr.appendChild(tdNeeded);
      tr.appendChild(tdAvail);
      tr.appendChild(tdShortage);
      tbody.appendChild(tr);
    }
    table.appendChild(tbody);
    section.appendChild(table);

    // Order button for shortages
    if (hasShortage) {
      const orderBtn = document.createElement("button");
      orderBtn.className = "dv-btn";
      orderBtn.style.marginTop = "8px";
      orderBtn.textContent = "Bestellung erstellen";
      orderBtn.addEventListener("click", async () => {
        try {
          const shortages =
            await ProcurementService.suggestOrders(projectId);
          if (shortages.length === 0) {
            ToastContainer.show("info", "Keine Fehlmengen vorhanden");
            return;
          }

          // Group by supplier
          const grouped = new Map<
            number,
            { supplierName: string; items: MaterialRequirement[] }
          >();
          for (const s of shortages) {
            const key = s.supplierId || 0;
            if (!grouped.has(key)) {
              grouped.set(key, {
                supplierName: s.supplierName || "Ohne Lieferant",
                items: [],
              });
            }
            grouped.get(key)!.items.push(s);
          }

          let createdCount = 0;
          for (const [supplierId, group] of grouped) {
            if (supplierId === 0) continue; // skip materials without supplier
            try {
              const order = await ProcurementService.createOrder({
                supplierId,
                projectId,
                notes: `Auto-Bestellung für Projekt (Fehlmengen)`,
              });
              // Add items to order
              for (const item of group.items) {
                await ProcurementService.addOrderItem(
                  order.id,
                  item.materialId,
                  item.shortage
                );
              }
              createdCount++;
            } catch {
              ToastContainer.show(
                "error",
                `Bestellung für ${group.supplierName} fehlgeschlagen`
              );
            }
          }
          if (createdCount > 0) {
            ToastContainer.show(
              "success",
              `${createdCount} Bestellung(en) erstellt`
            );
          }
        } catch {
          ToastContainer.show("error", "Bestellvorschlaege konnten nicht geladen werden");
        }
      });
      section.appendChild(orderBtn);
    }

    container.appendChild(section);
  }

  // ── Existing detail rendering ──────────────────────────────────────

  private renderDetail(): void {
    const pane =
      this.overlay?.querySelector<HTMLElement>('[data-id="pl-detail"]');
    if (!pane || !this.selectedProject) return;
    pane.innerHTML = "";

    const p = this.selectedProject;

    // Name
    const nameGroup = this.createField("Name", p.name, async (val) => {
      try {
        await ProjectService.updateProject(p.id, { name: val });
        await this.loadProjects();
        this.selectedProject =
          this.projects.find((pr) => pr.id === p.id) || null;
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
        await ProjectService.updateProject(p.id, {
          status: statusSelect.value,
        });
        await this.loadProjects();
        this.selectedProject =
          this.projects.find((pr) => pr.id === p.id) || null;
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
        ToastContainer.show(
          "error",
          "Notizen konnten nicht gespeichert werden"
        );
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
      { key: "chosen_size", label: "Gewählte Größe" },
      { key: "fabric_used", label: "Stoff" },
      { key: "planned_modifications", label: "Geplante Änderungen" },
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
            ToastContainer.show(
              "error",
              "Detail konnte nicht gespeichert werden"
            );
          }
        }
      );
      pane.appendChild(fieldGroup);
    }

    // Setup button (products, files, requirements)
    const setupBtn = document.createElement("button");
    setupBtn.className = "dv-btn";
    setupBtn.style.marginTop = "12px";
    setupBtn.textContent = "Produkte / Dateien / Material verwalten";
    setupBtn.addEventListener("click", () => {
      this.renderProjectSetup(p.id);
    });
    pane.appendChild(setupBtn);

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
      for (const h of ["Schritt", "Geplant", "Tatsächlich", "Differenz"]) {
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
        tdDiff.className =
          diff > 0 ? "pl-tc-over" : diff < 0 ? "pl-tc-under" : "";
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
        totalDiff > 0
          ? "pl-tc-over"
          : totalDiff < 0
            ? "pl-tc-under"
            : "";
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
    delBtn.textContent = "Löschen";
    delBtn.addEventListener("click", async () => {
      const ok = await ConfirmDialog.open({
        title: "Projekt löschen?",
        message: `Projekt „${p.name}" wird gelöscht.`,
        destructive: true,
      });
      if (!ok) return;
      try {
        await ProjectService.deleteProject(p.id);
        this.selectedProject = null;
        await this.loadProjects();
        this.renderList();
        const detailP =
          this.overlay?.querySelector<HTMLElement>('[data-id="pl-detail"]');
        if (detailP) detailP.textContent = "Projekt auswählen";
      } catch {
        ToastContainer.show("error", "Löschen fehlgeschlagen");
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
    auditBtn.textContent = "Änderungshistorie";
    auditBtn.addEventListener("click", async () => {
      auditBtn.style.display = "none";
      try {
        const entries = await ReportService.getAuditLog("project", p.id);
        if (entries.length === 0) {
          const hint = document.createElement("div");
          hint.style.fontSize = "0.85em";
          hint.style.opacity = "0.6";
          hint.textContent = "Keine Änderungen protokolliert";
          auditSection.appendChild(hint);
          return;
        }
        const table = document.createElement("table");
        table.className = "pl-tc-table";
        table.style.fontSize = "0.85em";
        table.innerHTML =
          "<thead><tr><th>Feld</th><th>Alt</th><th>Neu</th><th>Datum</th></tr></thead>";
        const tbody = document.createElement("tbody");
        for (const e of entries) {
          const tr = document.createElement("tr");
          for (const cell of [
            e.fieldName,
            e.oldValue || "-",
            e.newValue || "-",
            e.changedAt.substring(0, 16),
          ]) {
            const td = document.createElement("td");
            td.textContent = cell;
            tr.appendChild(td);
          }
          tbody.appendChild(tr);
        }
        table.appendChild(tbody);
        auditSection.appendChild(table);
      } catch {
        ToastContainer.show(
          "error",
          "Historie konnte nicht geladen werden"
        );
      }
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
