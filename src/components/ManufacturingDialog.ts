import * as MfgService from "../services/ManufacturingService";
import { ToastContainer } from "./Toast";
import { trapFocus } from "../utils/focus-trap";
import * as ProjectService from "../services/ProjectService";
import * as ReportService from "../services/ReportService";
import { appState } from "../state/AppState";
import type {
  Supplier,
  Material,
  Product,
  BillOfMaterial,
  Project,
  StepDefinition,
  LicenseRecord,
  QualityInspection,
  DefectRecord,
  ProjectReport,
  CostBreakdown,
  CostRate,
  EmbroideryFile,
} from "../types";

type TabKey = "materials" | "suppliers" | "products" | "workflow" | "licenses" | "quality" | "costrates" | "reports";

export class ManufacturingDialog {
  private static instance: ManufacturingDialog | null = null;

  private overlay: HTMLElement | null = null;
  private keyHandler: ((e: KeyboardEvent) => void) | null = null;
  private releaseFocusTrap: (() => void) | null = null;

  private activeTab: TabKey = "materials";

  // Data caches
  private materials: Material[] = [];
  private suppliers: Supplier[] = [];
  private products: Product[] = [];
  private bomMap: Map<number, BillOfMaterial[]> = new Map();

  // Selection state
  private selectedMaterial: Material | null = null;
  private selectedSupplier: Supplier | null = null;
  private selectedProduct: Product | null = null;
  private fieldIdCounter = 0;

  // Shared project state
  private allProjects: Project[] = [];

  // Workflow state
  private stepDefs: StepDefinition[] = [];
  private selectedStepDef: StepDefinition | null = null;

  // License state
  private licenses: LicenseRecord[] = [];
  private selectedLicense: LicenseRecord | null = null;

  // Quality state
  private qaProjectId: number | null = null;
  private inspections: QualityInspection[] = [];
  private selectedInspection: QualityInspection | null = null;
  private defects: DefectRecord[] = [];

  // Reports state
  private reportProjectId: number | null = null;
  private currentReport: ProjectReport | null = null;
  private costBreakdown: CostBreakdown | null = null;
  private costRates: CostRate[] = [];
  private reportMode: "project" | "product" = "project";
  private reportProductId: number | null = null;
  private reportQuantity: number = 1;

  static async open(): Promise<void> {
    if (ManufacturingDialog.instance) ManufacturingDialog.dismiss();
    const dialog = new ManufacturingDialog();
    ManufacturingDialog.instance = dialog;
    await dialog.init();
  }

  static dismiss(): void {
    if (ManufacturingDialog.instance) {
      ManufacturingDialog.instance.close();
      ManufacturingDialog.instance = null;
    }
  }

  private async init(): Promise<void> {
    try {
      await this.loadAll();
    } catch (e) {
      ToastContainer.show("error", "Fertigungsdaten konnten nicht geladen werden");
      return;
    }
    this.overlay = this.buildUI();
    document.body.appendChild(this.overlay);
    const dialog = this.overlay.querySelector<HTMLElement>(".dialog") || this.overlay;
    this.releaseFocusTrap = trapFocus(dialog);
    this.keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        ManufacturingDialog.dismiss();
      }
    };
    document.addEventListener("keydown", this.keyHandler);
  }

  private async loadAll(): Promise<void> {
    [this.materials, this.suppliers, this.products, this.allProjects, this.stepDefs, this.licenses, this.costRates] =
      await Promise.all([
        MfgService.getMaterials(),
        MfgService.getSuppliers(),
        MfgService.getProducts(),
        ProjectService.getProjects(),
        MfgService.getStepDefs(),
        MfgService.getLicenses(),
        ReportService.listCostRates(),
      ]);
  }

  // ── UI Build ─────────────────────────────────────────────────────

  private buildUI(): HTMLElement {
    const overlay = document.createElement("div");
    overlay.className = "mfg-overlay";

    const dialog = document.createElement("div");
    dialog.className = "mfg-dialog";
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", "Fertigung");

    // Header
    const header = document.createElement("div");
    header.className = "mfg-header";
    const title = document.createElement("h2");
    title.className = "mfg-title";
    title.textContent = "Fertigung";
    header.appendChild(title);

    const closeBtn = document.createElement("button");
    closeBtn.className = "dv-close-btn";
    closeBtn.textContent = "\u00D7";
    closeBtn.setAttribute("aria-label", "Schliessen");
    closeBtn.addEventListener("click", () => ManufacturingDialog.dismiss());
    header.appendChild(closeBtn);
    dialog.appendChild(header);

    // Tab bar
    const tabBar = document.createElement("div");
    tabBar.className = "mfg-tab-bar";
    tabBar.setAttribute("role", "tablist");
    const tabs: { key: TabKey; label: string }[] = [
      { key: "materials", label: "Materialien" },
      { key: "suppliers", label: "Lieferanten" },
      { key: "products", label: "Produkte" },
      { key: "workflow", label: "Workflow" },
      { key: "licenses", label: "Lizenzen" },
      { key: "quality", label: "Qualitaet" },
      { key: "costrates", label: "Kostensaetze" },
      { key: "reports", label: "Berichte" },
    ];
    for (const t of tabs) {
      const btn = document.createElement("button");
      btn.className = "mfg-tab" + (t.key === this.activeTab ? " active" : "");
      btn.textContent = t.label;
      btn.dataset.tab = t.key;
      btn.setAttribute("role", "tab");
      btn.setAttribute("aria-selected", t.key === this.activeTab ? "true" : "false");
      btn.addEventListener("click", () => {
        this.activeTab = t.key;
        tabBar.querySelectorAll(".mfg-tab").forEach((b) => {
          b.classList.remove("active");
          b.setAttribute("aria-selected", "false");
        });
        btn.classList.add("active");
        btn.setAttribute("aria-selected", "true");
        this.renderActiveTab();
      });
      tabBar.appendChild(btn);
    }
    dialog.appendChild(tabBar);

    // Dashboard
    const dashboard = document.createElement("div");
    dashboard.className = "mfg-dashboard";
    dashboard.dataset.id = "mfg-dashboard";
    dialog.appendChild(dashboard);

    // Content area
    const content = document.createElement("div");
    content.className = "mfg-content";
    content.dataset.id = "mfg-content";
    dialog.appendChild(content);

    overlay.appendChild(dialog);
    this.renderActiveTab();
    return overlay;
  }

  private renderActiveTab(): void {
    const dashboard = this.overlay?.querySelector<HTMLElement>(
      '[data-id="mfg-dashboard"]'
    );
    const content = this.overlay?.querySelector<HTMLElement>(
      '[data-id="mfg-content"]'
    );
    if (!dashboard || !content) return;
    dashboard.innerHTML = "";
    content.innerHTML = "";

    switch (this.activeTab) {
      case "materials":
        this.renderMaterialsDashboard(dashboard);
        this.renderMaterialsTab(content);
        break;
      case "suppliers":
        this.renderSuppliersDashboard(dashboard);
        this.renderSuppliersTab(content);
        break;
      case "products":
        this.renderProductsDashboard(dashboard);
        this.renderProductsTab(content);
        break;
      case "workflow":
        this.renderWorkflowDashboard(dashboard);
        this.renderWorkflowTab(content);
        break;
      case "licenses":
        this.renderLicensesDashboard(dashboard);
        this.renderLicensesTab(content);
        break;
      case "quality":
        this.renderQualityDashboard(dashboard);
        this.renderQualityTab(content);
        break;
      case "costrates":
        this.renderCostRatesDashboard(dashboard);
        this.renderCostRatesTab(content);
        break;
      case "reports":
        this.renderReportsDashboard(dashboard);
        this.renderReportsTab(content);
        break;
    }
  }

  // ── Materials Tab ────────────────────────────────────────────────

  private renderMaterialsDashboard(container: HTMLElement): void {
    const total = this.materials.length;
    this.addBadge(container, `Gesamt: ${total}`, "");
    this.addCreateBtn(container, "Material", () => this.createMaterial());
  }

  private renderMaterialsTab(container: HTMLElement): void {
    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    this.renderMaterialList(listPane);
    container.appendChild(listPane);

    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";
    detailPane.dataset.id = "mfg-detail";
    if (this.selectedMaterial) {
      this.renderMaterialDetail(detailPane, this.selectedMaterial);
    } else {
      detailPane.textContent = "Material auswaehlen";
    }
    container.appendChild(detailPane);
  }

  private renderMaterialList(container: HTMLElement): void {
    container.innerHTML = "";
    if (this.materials.length === 0) {
      container.textContent = "Keine Materialien";
      return;
    }
    for (const m of this.materials) {
      const item = document.createElement("div");
      item.className = "mfg-item";
      if (this.selectedMaterial?.id === m.id) item.classList.add("selected");

      const info = document.createElement("div");
      info.className = "mfg-item-info";
      const nameEl = document.createElement("span");
      nameEl.className = "mfg-item-name";
      nameEl.textContent = m.name;
      info.appendChild(nameEl);
      if (m.materialType) {
        const typeEl = document.createElement("span");
        typeEl.className = "mfg-item-sub";
        typeEl.textContent = this.materialTypeLabel(m.materialType);
        info.appendChild(typeEl);
      }
      item.appendChild(info);

      item.addEventListener("click", () => {
        this.selectedMaterial = m;
        this.renderActiveTab();
      });
      container.appendChild(item);
    }
  }

  private renderMaterialDetail(container: HTMLElement, m: Material): void {
    container.innerHTML = "";

    const form = document.createElement("div");
    form.className = "mfg-form";

    this.addTextField(form, "Name", m.name, (v) =>
      this.updateMaterial(m.id, { name: v })
    );
    this.addTextField(form, "Materialnummer", m.materialNumber || "", (v) =>
      this.updateMaterial(m.id, { materialNumber: v })
    );
    this.addSelectField(
      form,
      "Typ",
      m.materialType || "",
      [
        { value: "", label: "-- Waehlen --" },
        { value: "fabric", label: "Stoff" },
        { value: "thread", label: "Garn" },
        { value: "embroidery_thread", label: "Stickgarn" },
        { value: "vlies", label: "Vlies" },
        { value: "zipper", label: "Reissverschluss" },
        { value: "button", label: "Knopf" },
        { value: "label", label: "Etikett" },
        { value: "other", label: "Sonstiges" },
      ],
      (v) => this.updateMaterial(m.id, { materialType: v })
    );
    this.addSelectField(
      form,
      "Einheit",
      m.unit || "Stk",
      [
        { value: "Stk", label: "Stueck (Stk)" },
        { value: "m", label: "Meter (m)" },
        { value: "m2", label: "Quadratmeter (m\u00B2)" },
        { value: "kg", label: "Kilogramm (kg)" },
      ],
      (v) => this.updateMaterial(m.id, { unit: v })
    );
    this.addSelectField(
      form,
      "Lieferant",
      String(m.supplierId ?? ""),
      [
        { value: "", label: "-- Kein Lieferant --" },
        ...this.suppliers.map((s) => ({ value: String(s.id), label: s.name })),
      ],
      (v) =>
        this.updateMaterial(m.id, { supplierId: v ? Number(v) : undefined })
    );
    this.addNumberField(form, "Nettopreis", m.netPrice, (v) =>
      this.updateMaterial(m.id, { netPrice: v })
    );
    this.addNumberField(form, "Verschnittfaktor (0-1)", m.wasteFactor, (v) =>
      this.updateMaterial(m.id, { wasteFactor: v })
    );
    this.addNumberField(form, "Mindestbestand", m.minStock, (v) =>
      this.updateMaterial(m.id, { minStock: v })
    );
    this.addNumberField(form, "Nachbestellzeit (Tage)", m.reorderTimeDays, (v) =>
      this.updateMaterial(m.id, { reorderTimeDays: v != null ? Math.round(v) : undefined })
    );
    this.addTextArea(form, "Notizen", m.notes || "", (v) =>
      this.updateMaterial(m.id, { notes: v })
    );

    container.appendChild(form);

    // Actions
    const actions = document.createElement("div");
    actions.className = "mfg-actions";
    const delBtn = document.createElement("button");
    delBtn.className = "dialog-btn dialog-btn-danger";
    delBtn.textContent = "Material loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm(`Material "${m.name}" wirklich loeschen?`)) return;
      try {
        await MfgService.deleteMaterial(m.id);
        this.selectedMaterial = null;
        await this.loadAll();
        this.renderActiveTab();
        ToastContainer.show("success", "Material geloescht");
      } catch (e) {
        ToastContainer.show("error", "Loeschen fehlgeschlagen");
      }
    });
    actions.appendChild(delBtn);
    container.appendChild(actions);

    this.renderAuditHistory(container, "material", m.id);
  }

  private async createMaterial(): Promise<void> {
    try {
      const m = await MfgService.createMaterial({ name: "Neues Material" });
      await this.loadAll();
      this.selectedMaterial = this.materials.find((x) => x.id === m.id) || null;
      this.renderActiveTab();
      ToastContainer.show("success", "Material erstellt");
    } catch (e) {
      ToastContainer.show("error", "Erstellen fehlgeschlagen");
    }
  }

  private async updateMaterial(
    id: number,
    update: Parameters<typeof MfgService.updateMaterial>[1]
  ): Promise<void> {
    try {
      const updated = await MfgService.updateMaterial(id, update);
      const idx = this.materials.findIndex((x) => x.id === id);
      if (idx >= 0) this.materials[idx] = updated;
      this.selectedMaterial = updated;
      this.renderActiveTab();
    } catch (e) {
      ToastContainer.show("error", "Speichern fehlgeschlagen");
    }
  }



  // ── Suppliers Tab ────────────────────────────────────────────────

  private renderSuppliersDashboard(container: HTMLElement): void {
    this.addBadge(container, `Gesamt: ${this.suppliers.length}`, "");
    this.addCreateBtn(container, "Lieferant", () => this.createSupplier());
  }

  private renderSuppliersTab(container: HTMLElement): void {
    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    this.renderSupplierList(listPane);
    container.appendChild(listPane);

    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";
    if (this.selectedSupplier) {
      this.renderSupplierDetail(detailPane, this.selectedSupplier);
    } else {
      detailPane.textContent = "Lieferant auswaehlen";
    }
    container.appendChild(detailPane);
  }

  private renderSupplierList(container: HTMLElement): void {
    container.innerHTML = "";
    if (this.suppliers.length === 0) {
      container.textContent = "Keine Lieferanten";
      return;
    }
    for (const s of this.suppliers) {
      const item = document.createElement("div");
      item.className = "mfg-item";
      if (this.selectedSupplier?.id === s.id) item.classList.add("selected");
      const info = document.createElement("div");
      info.className = "mfg-item-info";
      const nameEl = document.createElement("span");
      nameEl.className = "mfg-item-name";
      nameEl.textContent = s.name;
      info.appendChild(nameEl);
      if (s.contact) {
        const sub = document.createElement("span");
        sub.className = "mfg-item-sub";
        sub.textContent = s.contact;
        info.appendChild(sub);
      }
      item.appendChild(info);
      item.addEventListener("click", () => {
        this.selectedSupplier = s;
        this.renderActiveTab();
      });
      container.appendChild(item);
    }
  }

  private renderSupplierDetail(container: HTMLElement, s: Supplier): void {
    container.innerHTML = "";
    const form = document.createElement("div");
    form.className = "mfg-form";

    this.addTextField(form, "Name", s.name, (v) =>
      this.updateSupplier(s.id, { name: v })
    );
    this.addTextField(form, "Kontakt", s.contact || "", (v) =>
      this.updateSupplier(s.id, { contact: v })
    );
    this.addTextField(form, "Website", s.website || "", (v) =>
      this.updateSupplier(s.id, { website: v })
    );
    this.addTextArea(form, "Notizen", s.notes || "", (v) =>
      this.updateSupplier(s.id, { notes: v })
    );
    container.appendChild(form);

    const actions = document.createElement("div");
    actions.className = "mfg-actions";
    const delBtn = document.createElement("button");
    delBtn.className = "dialog-btn dialog-btn-danger";
    delBtn.textContent = "Lieferant loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm(`Lieferant "${s.name}" wirklich loeschen?`)) return;
      try {
        await MfgService.deleteSupplier(s.id);
        this.selectedSupplier = null;
        await this.loadAll();
        this.renderActiveTab();
        ToastContainer.show("success", "Lieferant geloescht");
      } catch (e) {
        ToastContainer.show("error", "Loeschen fehlgeschlagen");
      }
    });
    actions.appendChild(delBtn);
    container.appendChild(actions);
  }

  private async createSupplier(): Promise<void> {
    try {
      const s = await MfgService.createSupplier({ name: "Neuer Lieferant" });
      await this.loadAll();
      this.selectedSupplier =
        this.suppliers.find((x) => x.id === s.id) || null;
      this.renderActiveTab();
      ToastContainer.show("success", "Lieferant erstellt");
    } catch (e) {
      ToastContainer.show("error", "Erstellen fehlgeschlagen");
    }
  }

  private async updateSupplier(
    id: number,
    update: Parameters<typeof MfgService.updateSupplier>[1]
  ): Promise<void> {
    try {
      const updated = await MfgService.updateSupplier(id, update);
      const idx = this.suppliers.findIndex((x) => x.id === id);
      if (idx >= 0) this.suppliers[idx] = updated;
      this.selectedSupplier = updated;
      this.renderActiveTab();
    } catch (e) {
      ToastContainer.show("error", "Speichern fehlgeschlagen");
    }
  }

  // ── Products Tab ─────────────────────────────────────────────────

  private renderProductsDashboard(container: HTMLElement): void {
    const active = this.products.filter((p) => p.status !== "inactive").length;
    this.addBadge(container, `Gesamt: ${this.products.length}`, "");
    this.addBadge(container, `Aktiv: ${active}`, "");
    this.addCreateBtn(container, "Produkt", () => this.createProduct());
  }

  private renderProductsTab(container: HTMLElement): void {
    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    this.renderProductList(listPane);
    container.appendChild(listPane);

    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";
    if (this.selectedProduct) {
      this.renderProductDetail(detailPane, this.selectedProduct);
    } else {
      detailPane.textContent = "Produkt auswaehlen";
    }
    container.appendChild(detailPane);
  }

  private renderProductList(container: HTMLElement): void {
    container.innerHTML = "";
    if (this.products.length === 0) {
      container.textContent = "Keine Produkte";
      return;
    }
    for (const p of this.products) {
      const item = document.createElement("div");
      item.className = "mfg-item";
      if (this.selectedProduct?.id === p.id) item.classList.add("selected");
      const info = document.createElement("div");
      info.className = "mfg-item-info";
      const nameEl = document.createElement("span");
      nameEl.className = "mfg-item-name";
      nameEl.textContent = p.name;
      info.appendChild(nameEl);
      if (p.productType) {
        const sub = document.createElement("span");
        sub.className = "mfg-item-sub";
        sub.textContent = this.productTypeLabel(p.productType);
        info.appendChild(sub);
      }
      item.appendChild(info);
      item.addEventListener("click", async () => {
        try {
          this.selectedProduct = p;
          if (!this.bomMap.has(p.id)) {
            this.bomMap.set(p.id, await MfgService.getBomEntries(p.id));
          }
          this.renderActiveTab();
        } catch (e) {
          ToastContainer.show("error", "Stueckliste konnte nicht geladen werden");
        }
      });
      container.appendChild(item);
    }
  }

  private renderProductDetail(container: HTMLElement, p: Product): void {
    container.innerHTML = "";
    const form = document.createElement("div");
    form.className = "mfg-form";

    this.addTextField(form, "Name", p.name, (v) =>
      this.updateProduct(p.id, { name: v })
    );
    this.addTextField(form, "Produktnummer", p.productNumber || "", (v) =>
      this.updateProduct(p.id, { productNumber: v })
    );
    this.addTextField(form, "Kategorie", p.category || "", (v) =>
      this.updateProduct(p.id, { category: v })
    );
    this.addSelectField(
      form,
      "Produkttyp",
      p.productType || "",
      [
        { value: "", label: "-- Waehlen --" },
        { value: "naehprodukt", label: "Naehprodukt" },
        { value: "stickprodukt", label: "Stickprodukt" },
        { value: "kombiprodukt", label: "Kombiprodukt" },
      ],
      (v) => this.updateProduct(p.id, { productType: v })
    );
    this.addSelectField(
      form,
      "Status",
      p.status || "active",
      [
        { value: "active", label: "Aktiv" },
        { value: "inactive", label: "Inaktiv" },
      ],
      (v) => this.updateProduct(p.id, { status: v })
    );
    this.addTextArea(form, "Beschreibung", p.description || "", (v) =>
      this.updateProduct(p.id, { description: v })
    );
    container.appendChild(form);

    // BOM section
    const bomSection = document.createElement("div");
    bomSection.className = "mfg-bom-section";
    const bomTitle = document.createElement("h4");
    bomTitle.className = "mfg-section-title";
    bomTitle.textContent = "Stueckliste (BOM)";
    bomSection.appendChild(bomTitle);

    const entries = this.bomMap.get(p.id) || [];
    const files = appState.get("files") as EmbroideryFile[] || [];
    if (entries.length > 0) {
      const table = document.createElement("table");
      table.className = "mfg-bom-table";
      const thead = document.createElement("thead");
      const headTr = document.createElement("tr");
      for (const h of ["Typ", "Bezeichnung", "Menge/Zeit", "Einheit", ""]) {
        const th = document.createElement("th");
        th.textContent = h;
        headTr.appendChild(th);
      }
      thead.appendChild(headTr);
      table.appendChild(thead);
      const tbody = document.createElement("tbody");
      for (const bom of entries) {
        const tr = document.createElement("tr");
        const tdType = document.createElement("td");
        tdType.textContent = this.bomTypeLabel(bom.entryType);
        tr.appendChild(tdType);

        const tdName = document.createElement("td");
        if (bom.entryType === "material") {
          const mat = this.materials.find((m) => m.id === bom.materialId);
          tdName.textContent = mat?.name || "?";
        } else if (bom.entryType === "work_step" || bom.entryType === "machine_time") {
          const sd = this.stepDefs.find((s) => s.id === bom.stepDefinitionId);
          tdName.textContent = bom.label || sd?.name || "?";
        } else if (bom.entryType === "pattern" || bom.entryType === "cutting_template") {
          const f = files.find((fi) => fi.id === bom.fileId);
          let name = f?.filename || f?.name || "?";
          if (bom.entryType === "pattern" && f?.stitchCount) {
            name += ` (${f.stitchCount} Stiche)`;
          }
          tdName.textContent = name;
        }
        tr.appendChild(tdName);

        const tdQty = document.createElement("td");
        if (bom.entryType === "material") {
          tdQty.textContent = String(bom.quantity);
        } else if (bom.entryType === "work_step" || bom.entryType === "machine_time") {
          tdQty.textContent = bom.durationMinutes != null ? `${bom.durationMinutes} min` : "-";
        } else {
          tdQty.textContent = "-";
        }
        tr.appendChild(tdQty);

        const tdUnit = document.createElement("td");
        if (bom.entryType === "material") {
          tdUnit.textContent = bom.unit || "";
        } else {
          tdUnit.textContent = "";
        }
        tr.appendChild(tdUnit);

        const tdAction = document.createElement("td");
        const rmBtn = document.createElement("button");
        rmBtn.className = "mfg-bom-remove";
        rmBtn.textContent = "\u2716";
        rmBtn.title = "Entfernen";
        rmBtn.addEventListener("click", async () => {
          try {
            await MfgService.deleteBomEntry(bom.id);
            this.bomMap.set(p.id, await MfgService.getBomEntries(p.id));
            this.renderActiveTab();
          } catch (e) {
            ToastContainer.show("error", "BOM-Eintrag konnte nicht entfernt werden");
          }
        });
        tdAction.appendChild(rmBtn);
        tr.appendChild(tdAction);
        tbody.appendChild(tr);
      }
      table.appendChild(tbody);
      bomSection.appendChild(table);
    } else {
      const empty = document.createElement("div");
      empty.className = "mfg-item-sub";
      empty.textContent = "Keine Eintraege in der Stueckliste";
      bomSection.appendChild(empty);
    }

    // Add BOM entry form with type selector
    const addRow = document.createElement("div");
    addRow.className = "mfg-bom-add";
    addRow.style.flexWrap = "wrap";
    addRow.style.gap = "4px";

    const typeSelect = document.createElement("select");
    typeSelect.className = "mfg-input";
    for (const [val, lbl] of [
      ["material", "Material"],
      ["work_step", "Arbeitsschritt"],
      ["machine_time", "Maschinenzeit"],
      ["pattern", "Stickmuster"],
      ["cutting_template", "Schnittvorlage"],
    ] as [string, string][]) {
      const opt = document.createElement("option");
      opt.value = val;
      opt.textContent = lbl;
      typeSelect.appendChild(opt);
    }
    addRow.appendChild(typeSelect);

    // Dynamic fields container
    const dynFields = document.createElement("div");
    dynFields.className = "mfg-bom-add";
    dynFields.style.flexWrap = "wrap";
    dynFields.style.gap = "4px";

    const renderDynFields = (et: string) => {
      dynFields.innerHTML = "";
      if (et === "material") {
        const matSelect = document.createElement("select");
        matSelect.className = "mfg-input";
        matSelect.dataset.field = "materialId";
        const defaultOpt = document.createElement("option");
        defaultOpt.value = "";
        defaultOpt.textContent = "Material waehlen";
        matSelect.appendChild(defaultOpt);
        for (const mat of this.materials) {
          const opt = document.createElement("option");
          opt.value = String(mat.id);
          opt.textContent = mat.name;
          matSelect.appendChild(opt);
        }
        dynFields.appendChild(matSelect);
        const qtyIn = document.createElement("input");
        qtyIn.type = "number";
        qtyIn.className = "mfg-input mfg-input-sm";
        qtyIn.placeholder = "Menge";
        qtyIn.min = "0.01";
        qtyIn.step = "0.01";
        qtyIn.dataset.field = "quantity";
        dynFields.appendChild(qtyIn);
        const unitIn = document.createElement("input");
        unitIn.type = "text";
        unitIn.className = "mfg-input mfg-input-sm";
        unitIn.placeholder = "Einheit";
        unitIn.dataset.field = "unit";
        dynFields.appendChild(unitIn);
      } else if (et === "work_step" || et === "machine_time") {
        const dlId = `bom-step-dl-${this.fieldIdCounter++}`;
        const labelIn = document.createElement("input");
        labelIn.className = "mfg-input";
        labelIn.placeholder = et === "work_step" ? "Arbeitsschritt" : "Maschine";
        labelIn.setAttribute("list", dlId);
        labelIn.dataset.field = "label";
        dynFields.appendChild(labelIn);
        const dl = document.createElement("datalist");
        dl.id = dlId;
        for (const sd of this.stepDefs) {
          const opt = document.createElement("option");
          opt.value = sd.name;
          opt.dataset.sdId = String(sd.id);
          dl.appendChild(opt);
        }
        dynFields.appendChild(dl);
        const durIn = document.createElement("input");
        durIn.type = "number";
        durIn.className = "mfg-input mfg-input-sm";
        durIn.placeholder = "Dauer (min)";
        durIn.min = "0.1";
        durIn.step = "0.1";
        durIn.dataset.field = "durationMinutes";
        dynFields.appendChild(durIn);
      } else if (et === "pattern" || et === "cutting_template") {
        const dlId = `bom-file-dl-${this.fieldIdCounter++}`;
        const fileIn = document.createElement("input");
        fileIn.className = "mfg-input";
        fileIn.placeholder = "Datei suchen...";
        fileIn.setAttribute("list", dlId);
        fileIn.dataset.field = "fileSearch";
        dynFields.appendChild(fileIn);
        const dl = document.createElement("datalist");
        dl.id = dlId;
        const filteredFiles = et === "pattern"
          ? files.filter((f) => ["pes", "dst", "jef", "vp3"].includes(f.filepath.split(".").pop()?.toLowerCase() || ""))
          : files;
        for (const f of filteredFiles) {
          const opt = document.createElement("option");
          const displayText = f.name || f.filename;
          opt.value = displayText;
          opt.dataset.fileId = String(f.id);
          if (et === "pattern" && f.stitchCount) {
            opt.textContent = `${displayText} (${f.stitchCount} Stiche)`;
          }
          dl.appendChild(opt);
        }
        dynFields.appendChild(dl);
      }
    };

    renderDynFields("material");
    typeSelect.addEventListener("change", () => renderDynFields(typeSelect.value));
    addRow.appendChild(dynFields);

    const addBtn = document.createElement("button");
    addBtn.className = "dialog-btn dialog-btn-primary";
    addBtn.textContent = "+";
    addBtn.addEventListener("click", async () => {
      const et = typeSelect.value;
      try {
        if (et === "material") {
          const matId = Number((dynFields.querySelector('[data-field="materialId"]') as HTMLSelectElement)?.value);
          const qty = Number((dynFields.querySelector('[data-field="quantity"]') as HTMLInputElement)?.value);
          const unit = (dynFields.querySelector('[data-field="unit"]') as HTMLInputElement)?.value;
          if (!matId || !qty || qty <= 0) {
            ToastContainer.show("error", "Material und Menge angeben");
            return;
          }
          await MfgService.addBomEntry(p.id, { entryType: "material", materialId: matId, quantity: qty, unit: unit || undefined });
        } else if (et === "work_step" || et === "machine_time") {
          const labelVal = (dynFields.querySelector('[data-field="label"]') as HTMLInputElement)?.value;
          const dur = Number((dynFields.querySelector('[data-field="durationMinutes"]') as HTMLInputElement)?.value);
          if (!dur || dur <= 0) {
            ToastContainer.show("error", "Dauer muss angegeben werden");
            return;
          }
          // Check if label matches a step definition for step_definition_id
          let stepDefId: number | undefined;
          const dlOpts = dynFields.querySelectorAll("datalist option");
          dlOpts.forEach((optEl) => {
            const opt = optEl as HTMLOptionElement;
            if (opt.value === labelVal && opt.dataset.sdId) {
              stepDefId = Number(opt.dataset.sdId);
            }
          });
          await MfgService.addBomEntry(p.id, { entryType: et, stepDefinitionId: stepDefId, durationMinutes: dur, label: labelVal || undefined });
        } else if (et === "pattern" || et === "cutting_template") {
          const fileSearch = (dynFields.querySelector('[data-field="fileSearch"]') as HTMLInputElement)?.value;
          let fileId: number | undefined;
          const dlOpts = dynFields.querySelectorAll("datalist option");
          dlOpts.forEach((optEl) => {
            const opt = optEl as HTMLOptionElement;
            if (opt.value === fileSearch && opt.dataset.fileId) {
              fileId = Number(opt.dataset.fileId);
            }
          });
          if (!fileId) {
            ToastContainer.show("error", "Datei muss ausgewaehlt werden");
            return;
          }
          await MfgService.addBomEntry(p.id, { entryType: et, fileId });
        }
        this.bomMap.set(p.id, await MfgService.getBomEntries(p.id));
        this.renderActiveTab();
      } catch (e) {
        ToastContainer.show("error", "BOM-Eintrag fehlgeschlagen");
      }
    });
    addRow.appendChild(addBtn);
    bomSection.appendChild(addRow);
    container.appendChild(bomSection);

    // Variants section
    const varSection = document.createElement("div");
    varSection.className = "mfg-bom-section";
    const varTitle = document.createElement("h4");
    varTitle.className = "mfg-section-title";
    varTitle.textContent = "Varianten";
    varSection.appendChild(varTitle);

    // Load and render variants
    MfgService.getProductVariants(p.id).then(variants => {
      if (variants.length > 0) {
        const vtable = document.createElement("table");
        vtable.className = "mfg-bom-table";
        const vthead = document.createElement("thead");
        const vheadTr = document.createElement("tr");
        for (const h of ["SKU", "Name", "Beschreibung", "Groesse", "Farbe", "Zusatzk.", ""]) {
          const vth = document.createElement("th");
          vth.textContent = h;
          vheadTr.appendChild(vth);
        }
        vthead.appendChild(vheadTr);
        vtable.appendChild(vthead);
        const vtbody = document.createElement("tbody");
        for (const v of variants) {
          const vtr = document.createElement("tr");
          for (const cell of [v.sku || "-", v.variantName || "-", v.description || "-", v.size || "-", v.color || "-", v.additionalCost ? `${v.additionalCost.toFixed(2)} EUR` : "-"]) {
            const vtd = document.createElement("td");
            vtd.textContent = cell;
            vtr.appendChild(vtd);
          }
          const vtdAction = document.createElement("td");
          const vrmBtn = document.createElement("button");
          vrmBtn.className = "mfg-bom-remove";
          vrmBtn.textContent = "\u2716";
          vrmBtn.title = "Variante loeschen";
          vrmBtn.addEventListener("click", async () => {
            try {
              await MfgService.deleteVariant(v.id);
              this.renderActiveTab();
            } catch { ToastContainer.show("error", "Loeschen fehlgeschlagen"); }
          });
          vtdAction.appendChild(vrmBtn);
          vtr.appendChild(vtdAction);
          vtbody.appendChild(vtr);
        }
        vtable.appendChild(vtbody);
        varSection.insertBefore(vtable, varSection.lastElementChild);
      }
    }).catch(() => { ToastContainer.show("error", "Varianten konnten nicht geladen werden"); });

    // Add variant form
    const varAddRow = document.createElement("div");
    varAddRow.className = "mfg-bom-add";
    varAddRow.style.flexWrap = "wrap";

    const skuIn = document.createElement("input");
    skuIn.className = "mfg-input mfg-input-sm";
    skuIn.placeholder = "SKU";
    varAddRow.appendChild(skuIn);

    const vnameIn = document.createElement("input");
    vnameIn.className = "mfg-input mfg-input-sm";
    vnameIn.placeholder = "Name";
    varAddRow.appendChild(vnameIn);

    const descIn = document.createElement("input");
    descIn.className = "mfg-input mfg-input-sm";
    descIn.placeholder = "Beschreibung";
    varAddRow.appendChild(descIn);

    const sizeIn = document.createElement("input");
    sizeIn.className = "mfg-input mfg-input-sm";
    sizeIn.placeholder = "Groesse";
    sizeIn.style.width = "70px";
    varAddRow.appendChild(sizeIn);

    const colorIn = document.createElement("input");
    colorIn.className = "mfg-input mfg-input-sm";
    colorIn.placeholder = "Farbe";
    colorIn.style.width = "70px";
    varAddRow.appendChild(colorIn);

    const costIn = document.createElement("input");
    costIn.className = "mfg-input mfg-input-sm";
    costIn.type = "number";
    costIn.step = "0.01";
    costIn.placeholder = "Zusatzkosten";
    costIn.style.width = "90px";
    varAddRow.appendChild(costIn);

    const varAddBtn = document.createElement("button");
    varAddBtn.className = "dialog-btn dialog-btn-primary";
    varAddBtn.textContent = "+";
    varAddBtn.addEventListener("click", async () => {
      if (!skuIn.value && !vnameIn.value && !sizeIn.value && !colorIn.value) {
        ToastContainer.show("error", "Mindestens SKU, Name, Groesse oder Farbe angeben");
        return;
      }
      try {
        await MfgService.createVariant(p.id, {
          sku: skuIn.value || undefined,
          variantName: vnameIn.value || undefined,
          description: descIn.value || undefined,
          size: sizeIn.value || undefined,
          color: colorIn.value || undefined,
          additionalCost: costIn.value ? parseFloat(costIn.value) : undefined,
        });
        this.renderActiveTab();
        ToastContainer.show("success", "Variante erstellt");
      } catch { ToastContainer.show("error", "Erstellen fehlgeschlagen"); }
    });
    varAddRow.appendChild(varAddBtn);
    varSection.appendChild(varAddRow);
    container.appendChild(varSection);

    // Actions
    const actions = document.createElement("div");
    actions.className = "mfg-actions";
    const bomExportBtn = document.createElement("button");
    bomExportBtn.className = "dialog-btn";
    bomExportBtn.textContent = "BOM Export";
    bomExportBtn.addEventListener("click", async () => {
      try {
        const csv = await ReportService.exportBomCsv(p.id);
        this.downloadCsv(csv, `bom_${p.productNumber || p.id}.csv`);
        ToastContainer.show("success", "BOM exportiert");
      } catch { ToastContainer.show("error", "Export fehlgeschlagen"); }
    });
    actions.appendChild(bomExportBtn);
    const delBtn = document.createElement("button");
    delBtn.className = "dialog-btn dialog-btn-danger";
    delBtn.textContent = "Produkt loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm(`Produkt "${p.name}" wirklich loeschen?`)) return;
      try {
        await MfgService.deleteProduct(p.id);
        this.selectedProduct = null;
        this.bomMap.delete(p.id);
        await this.loadAll();
        this.renderActiveTab();
        ToastContainer.show("success", "Produkt geloescht");
      } catch (e) {
        ToastContainer.show("error", "Loeschen fehlgeschlagen");
      }
    });
    actions.appendChild(delBtn);
    container.appendChild(actions);
  }

  private async createProduct(): Promise<void> {
    try {
      const p = await MfgService.createProduct({ name: "Neues Produkt" });
      await this.loadAll();
      this.selectedProduct = this.products.find((x) => x.id === p.id) || null;
      this.renderActiveTab();
      ToastContainer.show("success", "Produkt erstellt");
    } catch (e) {
      ToastContainer.show("error", "Erstellen fehlgeschlagen");
    }
  }

  private async updateProduct(
    id: number,
    update: Parameters<typeof MfgService.updateProduct>[1]
  ): Promise<void> {
    try {
      const updated = await MfgService.updateProduct(id, update);
      const idx = this.products.findIndex((x) => x.id === id);
      if (idx >= 0) this.products[idx] = updated;
      this.selectedProduct = updated;
      this.renderActiveTab();
    } catch (e) {
      ToastContainer.show("error", "Speichern fehlgeschlagen");
    }
  }

  // ── Workflow Tab ──────────────────────────────────────────────────

  private renderWorkflowDashboard(container: HTMLElement): void {
    this.addBadge(container, `Schrittvorlagen: ${this.stepDefs.length}`, "");
    this.addCreateBtn(container, "Schritt", () => this.createStepDef());
  }

  private renderWorkflowTab(container: HTMLElement): void {
    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    this.renderStepDefList(listPane);
    container.appendChild(listPane);

    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";
    if (this.selectedStepDef) {
      this.renderStepDefDetail(detailPane, this.selectedStepDef);
    } else {
      detailPane.textContent = "Schrittvorlage auswaehlen";
    }
    container.appendChild(detailPane);
  }

  private renderStepDefList(container: HTMLElement): void {
    container.innerHTML = "";
    if (this.stepDefs.length === 0) { container.textContent = "Keine Schrittvorlagen"; return; }
    for (const sd of this.stepDefs) {
      const item = document.createElement("div");
      item.className = "mfg-item";
      if (this.selectedStepDef?.id === sd.id) item.classList.add("selected");
      const info = document.createElement("div");
      info.className = "mfg-item-info";
      const nameEl = document.createElement("span");
      nameEl.className = "mfg-item-name";
      nameEl.textContent = sd.name;
      info.appendChild(nameEl);
      if (sd.defaultDurationMinutes) {
        const sub = document.createElement("span");
        sub.className = "mfg-item-sub";
        sub.textContent = `${sd.defaultDurationMinutes} min`;
        info.appendChild(sub);
      }
      item.appendChild(info);
      item.addEventListener("click", () => { this.selectedStepDef = sd; this.renderActiveTab(); });
      container.appendChild(item);
    }
  }

  private renderStepDefDetail(container: HTMLElement, sd: StepDefinition): void {
    container.innerHTML = "";
    const form = document.createElement("div");
    form.className = "mfg-form";
    this.addTextField(form, "Name", sd.name, async (v) => {
      try {
        const updated = await MfgService.updateStepDef(sd.id, v);
        const idx = this.stepDefs.findIndex((x) => x.id === sd.id);
        if (idx >= 0) this.stepDefs[idx] = updated;
        this.selectedStepDef = updated;
        this.renderActiveTab();
      } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    this.addTextArea(form, "Beschreibung", sd.description || "", async (v) => {
      try {
        const updated = await MfgService.updateStepDef(sd.id, undefined, v);
        const idx = this.stepDefs.findIndex((x) => x.id === sd.id);
        if (idx >= 0) this.stepDefs[idx] = updated;
        this.selectedStepDef = updated;
        this.renderActiveTab();
      } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    this.addNumberField(form, "Standarddauer (min)", sd.defaultDurationMinutes, async (v) => {
      try {
        const updated = await MfgService.updateStepDef(sd.id, undefined, undefined, v);
        const idx = this.stepDefs.findIndex((x) => x.id === sd.id);
        if (idx >= 0) this.stepDefs[idx] = updated;
        this.selectedStepDef = updated;
        this.renderActiveTab();
      } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    container.appendChild(form);

    const actions = document.createElement("div");
    actions.className = "mfg-actions";
    const delBtn = document.createElement("button");
    delBtn.className = "dialog-btn dialog-btn-danger";
    delBtn.textContent = "Vorlage loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm(`Schrittvorlage "${sd.name}" wirklich loeschen?`)) return;
      try {
        await MfgService.deleteStepDef(sd.id);
        this.selectedStepDef = null;
        this.stepDefs = await MfgService.getStepDefs();
        this.renderActiveTab();
        ToastContainer.show("success", "Vorlage geloescht");
      } catch { ToastContainer.show("error", "Loeschen fehlgeschlagen"); }
    });
    actions.appendChild(delBtn);
    container.appendChild(actions);
  }

  private async createStepDef(): Promise<void> {
    try {
      const sd = await MfgService.createStepDef({ name: "Neuer Schritt" });
      this.stepDefs = await MfgService.getStepDefs();
      this.selectedStepDef = this.stepDefs.find((x) => x.id === sd.id) || null;
      this.renderActiveTab();
      ToastContainer.show("success", "Schrittvorlage erstellt");
    } catch { ToastContainer.show("error", "Erstellen fehlgeschlagen"); }
  }

  // ── Licenses Tab ─────────────────────────────────────────────────

  private renderLicensesDashboard(container: HTMLElement): void {
    this.addBadge(container, `Gesamt: ${this.licenses.length}`, "");
    const now = new Date().toISOString();
    const expiring = this.licenses.filter((l) => l.validUntil && l.validUntil >= now.slice(0, 10) && l.validUntil <= new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10)).length;
    const expired = this.licenses.filter((l) => l.validUntil && l.validUntil < now.slice(0, 10)).length;
    if (expiring > 0) this.addBadge(container, `Bald ablaufend: ${expiring}`, "mfg-badge-warn");
    if (expired > 0) this.addBadge(container, `Abgelaufen: ${expired}`, "mfg-badge-warn");
    this.addCreateBtn(container, "Lizenz", () => this.createLicenseRecord());
  }

  private renderLicensesTab(container: HTMLElement): void {
    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    this.renderLicenseList(listPane);
    container.appendChild(listPane);

    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";
    if (this.selectedLicense) {
      this.renderLicenseDetail(detailPane, this.selectedLicense);
    } else {
      detailPane.textContent = "Lizenz auswaehlen";
    }
    container.appendChild(detailPane);
  }

  private renderLicenseList(container: HTMLElement): void {
    container.innerHTML = "";
    if (this.licenses.length === 0) { container.textContent = "Keine Lizenzen"; return; }
    const now = new Date().toISOString().slice(0, 10);
    for (const l of this.licenses) {
      const item = document.createElement("div");
      item.className = "mfg-item";
      if (this.selectedLicense?.id === l.id) item.classList.add("selected");

      let statusCls = "mfg-stock-ok";
      if (l.validUntil) {
        if (l.validUntil < now) statusCls = "mfg-stock-low";
        else if (l.validUntil <= new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10)) statusCls = "mfg-stock-warn";
      }
      const dot = document.createElement("span");
      dot.className = "mfg-stock-dot " + statusCls;
      item.appendChild(dot);

      const info = document.createElement("div");
      info.className = "mfg-item-info";
      const nameEl = document.createElement("span");
      nameEl.className = "mfg-item-name";
      nameEl.textContent = l.name;
      info.appendChild(nameEl);
      const sub = document.createElement("span");
      sub.className = "mfg-item-sub";
      sub.textContent = l.licenseType || "personal";
      info.appendChild(sub);
      item.appendChild(info);

      item.addEventListener("click", () => { this.selectedLicense = l; this.renderActiveTab(); });
      container.appendChild(item);
    }
  }

  private renderLicenseDetail(container: HTMLElement, l: LicenseRecord): void {
    container.innerHTML = "";
    const form = document.createElement("div");
    form.className = "mfg-form";

    const updateLic = async (
      name?: string, licenseType?: string, validFrom?: string, validUntil?: string,
      maxUses?: number, commercialAllowed?: boolean, source?: string, notes?: string
    ) => {
      try {
        const updated = await MfgService.updateLicense(l.id, name, licenseType, validFrom, validUntil, maxUses, commercialAllowed, source, notes);
        const idx = this.licenses.findIndex((x) => x.id === l.id);
        if (idx >= 0) this.licenses[idx] = updated;
        this.selectedLicense = updated;
        this.renderActiveTab();
      } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    };

    this.addTextField(form, "Name", l.name, (v) => updateLic(v));
    this.addSelectField(form, "Lizenztyp", l.licenseType || "personal", [
      { value: "personal", label: "Persoenlich" },
      { value: "commercial", label: "Kommerziell" },
      { value: "educational", label: "Bildung" },
      { value: "open", label: "Offen/Frei" },
    ], (v) => updateLic(undefined, v));
    this.addTextField(form, "Gueltig ab", l.validFrom || "", (v) => updateLic(undefined, undefined, v));
    this.addTextField(form, "Gueltig bis", l.validUntil || "", (v) => updateLic(undefined, undefined, undefined, v));
    this.addNumberField(form, "Max. Verwendungen", l.maxUses, (v) => updateLic(undefined, undefined, undefined, undefined, v != null ? Math.round(v) : undefined));
    this.addSelectField(form, "Kommerziell erlaubt", l.commercialAllowed ? "1" : "0", [
      { value: "0", label: "Nein" },
      { value: "1", label: "Ja" },
    ], (v) => updateLic(undefined, undefined, undefined, undefined, undefined, v === "1"));
    this.addTextField(form, "Quelle", l.source || "", (v) => updateLic(undefined, undefined, undefined, undefined, undefined, undefined, v));
    this.addTextArea(form, "Notizen", l.notes || "", (v) => updateLic(undefined, undefined, undefined, undefined, undefined, undefined, undefined, v));

    // Usage info
    const usageEl = document.createElement("div");
    usageEl.className = "mfg-field";
    const usageLbl = document.createElement("label");
    usageLbl.className = "mfg-label";
    usageLbl.textContent = "Verwendungen";
    const usageVal = document.createElement("span");
    usageVal.className = "mfg-readonly-value";
    usageVal.textContent = l.maxUses != null ? `${l.currentUses} / ${l.maxUses}` : `${l.currentUses}`;
    usageEl.appendChild(usageLbl);
    usageEl.appendChild(usageVal);
    form.appendChild(usageEl);

    container.appendChild(form);

    const actions = document.createElement("div");
    actions.className = "mfg-actions";
    const delBtn = document.createElement("button");
    delBtn.className = "dialog-btn dialog-btn-danger";
    delBtn.textContent = "Lizenz loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm(`Lizenz "${l.name}" wirklich loeschen?`)) return;
      try {
        await MfgService.deleteLicense(l.id);
        this.selectedLicense = null;
        this.licenses = await MfgService.getLicenses();
        this.renderActiveTab();
        ToastContainer.show("success", "Lizenz geloescht");
      } catch { ToastContainer.show("error", "Loeschen fehlgeschlagen"); }
    });
    actions.appendChild(delBtn);
    container.appendChild(actions);

    this.renderAuditHistory(container, "license", l.id);
  }

  private async createLicenseRecord(): Promise<void> {
    try {
      const l = await MfgService.createLicense({ name: "Neue Lizenz" });
      this.licenses = await MfgService.getLicenses();
      this.selectedLicense = this.licenses.find((x) => x.id === l.id) || null;
      this.renderActiveTab();
      ToastContainer.show("success", "Lizenz erstellt");
    } catch { ToastContainer.show("error", "Erstellen fehlgeschlagen"); }
  }

  // ── Quality Tab ──────────────────────────────────────────────────

  private renderQualityDashboard(container: HTMLElement): void {
    this.addBadge(container, `Pruefungen: ${this.inspections.length}`, "");
    const failed = this.inspections.filter((i) => i.result === "failed").length;
    if (failed > 0) this.addBadge(container, `Fehlgeschlagen: ${failed}`, "mfg-badge-warn");
    this.addCreateBtn(container, "Pruefung", () => this.createInspection());
  }

  private renderQualityTab(container: HTMLElement): void {
    // Project selector
    const selectorRow = document.createElement("div");
    selectorRow.className = "mfg-tt-selector";
    const selectorLabel = document.createElement("label");
    selectorLabel.className = "mfg-label";
    selectorLabel.textContent = "Projekt:";
    selectorRow.appendChild(selectorLabel);
    const projectSelect = document.createElement("select");
    projectSelect.className = "mfg-input";
    const emptyOpt = document.createElement("option"); emptyOpt.value = ""; emptyOpt.textContent = "Projekt waehlen";
    projectSelect.appendChild(emptyOpt);
    for (const p of this.allProjects) {
      const opt = document.createElement("option"); opt.value = String(p.id); opt.textContent = p.name;
      if (this.qaProjectId === p.id) opt.selected = true;
      projectSelect.appendChild(opt);
    }
    projectSelect.addEventListener("change", async () => {
      const id = projectSelect.value ? Number(projectSelect.value) : null;
      this.qaProjectId = id;
      this.selectedInspection = null;
      this.defects = [];
      if (id) {
        try { this.inspections = await MfgService.getInspections(id); } catch { this.inspections = []; }
      } else { this.inspections = []; }
      this.renderActiveTab();
    });
    selectorRow.appendChild(projectSelect);
    container.insertBefore(selectorRow, container.firstChild);

    if (!this.qaProjectId) {
      const hint = document.createElement("div");
      hint.className = "mfg-tt-hint";
      hint.textContent = "Projekt auswaehlen";
      container.appendChild(hint);
      return;
    }

    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    this.renderInspectionList(listPane);
    container.appendChild(listPane);

    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";
    if (this.selectedInspection) {
      this.renderInspectionDetail(detailPane, this.selectedInspection);
    } else {
      detailPane.textContent = "Pruefung auswaehlen";
    }
    container.appendChild(detailPane);
  }

  private renderInspectionList(container: HTMLElement): void {
    container.innerHTML = "";
    if (this.inspections.length === 0) { container.textContent = "Keine Pruefungen"; return; }
    const resultLabels: Record<string, string> = { pending: "Ausstehend", passed: "Bestanden", failed: "Fehlgeschlagen", rework: "Nacharbeit" };
    for (const insp of this.inspections) {
      const item = document.createElement("div");
      item.className = "mfg-item";
      if (this.selectedInspection?.id === insp.id) item.classList.add("selected");
      const dot = document.createElement("span");
      dot.className = "mfg-stock-dot " + (insp.result === "passed" ? "mfg-stock-ok" : insp.result === "failed" ? "mfg-stock-low" : "mfg-stock-warn");
      item.appendChild(dot);
      const info = document.createElement("div");
      info.className = "mfg-item-info";
      const nameEl = document.createElement("span");
      nameEl.className = "mfg-item-name";
      nameEl.textContent = resultLabels[insp.result] || insp.result;
      info.appendChild(nameEl);
      const sub = document.createElement("span");
      sub.className = "mfg-item-sub";
      sub.textContent = (insp.inspector || "Kein Pruefer") + " - " + insp.inspectionDate.slice(0, 10);
      info.appendChild(sub);
      item.appendChild(info);
      item.addEventListener("click", async () => {
        try {
          this.selectedInspection = insp;
          this.defects = await MfgService.getDefects(insp.id);
          this.renderActiveTab();
        } catch { ToastContainer.show("error", "Fehler konnten nicht geladen werden"); }
      });
      container.appendChild(item);
    }
  }

  private renderInspectionDetail(container: HTMLElement, insp: QualityInspection): void {
    container.innerHTML = "";
    const form = document.createElement("div");
    form.className = "mfg-form";

    this.addSelectField(form, "Ergebnis", insp.result, [
      { value: "pending", label: "Ausstehend" }, { value: "passed", label: "Bestanden" },
      { value: "failed", label: "Fehlgeschlagen" }, { value: "rework", label: "Nacharbeit" },
    ], async (v) => {
      try {
        const updated = await MfgService.updateInspection(insp.id, v);
        const idx = this.inspections.findIndex((x) => x.id === insp.id);
        if (idx >= 0) this.inspections[idx] = updated;
        this.selectedInspection = updated;
        this.renderActiveTab();
      } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    this.addTextField(form, "Pruefer", insp.inspector || "", async (v) => {
      try { await MfgService.updateInspection(insp.id, undefined, v); } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    this.addTextArea(form, "Notizen", insp.notes || "", async (v) => {
      try { await MfgService.updateInspection(insp.id, undefined, undefined, v); } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    container.appendChild(form);

    // Defects section
    const defSection = document.createElement("div");
    defSection.className = "mfg-bom-section";
    const defTitle = document.createElement("h4");
    defTitle.className = "mfg-section-title";
    defTitle.textContent = "Fehler / Maengel";
    defSection.appendChild(defTitle);

    for (const d of this.defects) {
      const row = document.createElement("div");
      row.className = "mfg-item";
      const info = document.createElement("div");
      info.className = "mfg-item-info";
      const descEl = document.createElement("span");
      descEl.className = "mfg-item-name";
      descEl.textContent = d.description;
      info.appendChild(descEl);
      const sub = document.createElement("span");
      sub.className = "mfg-item-sub";
      sub.textContent = `${d.severity || "minor"} - ${d.status || "open"}`;
      info.appendChild(sub);
      row.appendChild(info);
      const rmBtn = document.createElement("button");
      rmBtn.className = "mfg-bom-remove";
      rmBtn.textContent = "\u2716";
      rmBtn.addEventListener("click", async () => {
        try {
          await MfgService.deleteDefect(d.id);
          this.defects = await MfgService.getDefects(insp.id);
          this.renderActiveTab();
        } catch { ToastContainer.show("error", "Loeschen fehlgeschlagen"); }
      });
      row.appendChild(rmBtn);
      defSection.appendChild(row);
    }

    // Add defect form
    const addRow = document.createElement("div");
    addRow.className = "mfg-bom-add";
    const descInput = document.createElement("input");
    descInput.type = "text"; descInput.className = "mfg-input"; descInput.placeholder = "Fehlerbeschreibung";
    addRow.appendChild(descInput);
    const sevSelect = document.createElement("select");
    sevSelect.className = "mfg-input mfg-input-sm";
    for (const s of [{ v: "minor", l: "Gering" }, { v: "major", l: "Gross" }, { v: "critical", l: "Kritisch" }]) {
      const opt = document.createElement("option"); opt.value = s.v; opt.textContent = s.l; sevSelect.appendChild(opt);
    }
    addRow.appendChild(sevSelect);
    const addBtn = document.createElement("button");
    addBtn.className = "dialog-btn dialog-btn-primary"; addBtn.textContent = "+";
    addBtn.addEventListener("click", async () => {
      if (!descInput.value.trim()) { ToastContainer.show("error", "Beschreibung angeben"); return; }
      try {
        await MfgService.createDefect(insp.id, descInput.value.trim(), sevSelect.value);
        this.defects = await MfgService.getDefects(insp.id);
        descInput.value = "";
        this.renderActiveTab();
      } catch { ToastContainer.show("error", "Fehler erstellen fehlgeschlagen"); }
    });
    addRow.appendChild(addBtn);
    defSection.appendChild(addRow);
    container.appendChild(defSection);

    // Actions
    const actions = document.createElement("div");
    actions.className = "mfg-actions";
    const delBtn = document.createElement("button");
    delBtn.className = "dialog-btn dialog-btn-danger";
    delBtn.textContent = "Pruefung loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm("Pruefung wirklich loeschen?")) return;
      try {
        await MfgService.deleteInspection(insp.id);
        this.selectedInspection = null;
        this.defects = [];
        if (this.qaProjectId) this.inspections = await MfgService.getInspections(this.qaProjectId);
        this.renderActiveTab();
        ToastContainer.show("success", "Pruefung geloescht");
      } catch { ToastContainer.show("error", "Loeschen fehlgeschlagen"); }
    });
    actions.appendChild(delBtn);
    container.appendChild(actions);
  }

  private async createInspection(): Promise<void> {
    if (!this.qaProjectId) { ToastContainer.show("error", "Bitte zuerst ein Projekt waehlen"); return; }
    try {
      const insp = await MfgService.createInspection(this.qaProjectId);
      this.inspections = await MfgService.getInspections(this.qaProjectId);
      this.selectedInspection = this.inspections.find((x) => x.id === insp.id) || null;
      this.renderActiveTab();
      ToastContainer.show("success", "Pruefung erstellt");
    } catch { ToastContainer.show("error", "Erstellen fehlgeschlagen"); }
  }

  // ── Cost Rates Tab ──────────────────────────────────────────────

  private renderCostRatesDashboard(container: HTMLElement): void {
    const total = this.costRates.length;
    this.addBadge(container, `Kostensaetze: ${total}`, "");
    const laborRates = this.costRates.filter(r => r.rateType === "labor");
    if (laborRates.length > 0) {
      this.addBadge(container, `Arbeit: ${laborRates[0].rateValue} EUR/h`, "");
    }
  }

  private renderCostRatesTab(container: HTMLElement): void {
    // Two-column layout: rates left, product calculator right
    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    listPane.style.width = "50%";
    listPane.style.minWidth = "300px";

    // ── Left: Rate Management ──
    const rateSection = document.createElement("div");
    rateSection.className = "mfg-form";

    const rateTitle = document.createElement("h3");
    rateTitle.style.marginBottom = "var(--spacing-2)";
    rateTitle.textContent = "Kostensaetze verwalten";
    rateSection.appendChild(rateTitle);

    const renderInlineRates = () => {
      // Clear everything after title
      while (rateSection.children.length > 1) {
        rateSection.removeChild(rateSection.lastChild!);
      }

      const groups: Record<string, CostRate[]> = { stitch: [], labor: [], machine: [], overhead: [], profit: [] };
      for (const r of this.costRates) {
        if (groups[r.rateType]) groups[r.rateType].push(r);
      }

      const labels: Record<string, string> = {
        stitch: "Stickkosten (EUR/1000 Stiche)",
        labor: "Arbeit (EUR/h)",
        machine: "Maschine (EUR/h)",
        overhead: "Gemeinkosten (%)",
        profit: "Gewinn (%)",
      };

      for (const [type, rates] of Object.entries(groups)) {
        const section = document.createElement("div");
        section.className = "mfg-form-section";
        const sTitle = document.createElement("h4");
        sTitle.className = "mfg-form-section-title";
        sTitle.textContent = labels[type] || type;
        section.appendChild(sTitle);

        for (const rate of rates) {
          const row = document.createElement("div");
          row.className = "mfg-list-row";
          row.style.display = "flex";
          row.style.gap = "8px";
          row.style.alignItems = "center";
          row.style.marginBottom = "4px";

          const nameSpan = document.createElement("span");
          nameSpan.style.flex = "1";
          nameSpan.textContent = rate.name;
          row.appendChild(nameSpan);

          const valSpan = document.createElement("span");
          valSpan.style.minWidth = "80px";
          valSpan.style.textAlign = "right";
          valSpan.textContent = `${rate.rateValue}${rate.unit ? " " + rate.unit : ""}`;
          row.appendChild(valSpan);

          if (rate.setupCost > 0) {
            const setupSpan = document.createElement("span");
            setupSpan.style.minWidth = "100px";
            setupSpan.style.textAlign = "right";
            setupSpan.style.fontSize = "0.85em";
            setupSpan.style.opacity = "0.7";
            setupSpan.textContent = `Ruestk. ${rate.setupCost.toFixed(2)}`;
            row.appendChild(setupSpan);
          }

          const delBtn = document.createElement("button");
          delBtn.className = "dialog-btn dialog-btn-danger";
          delBtn.style.padding = "2px 8px";
          delBtn.style.fontSize = "0.85em";
          delBtn.textContent = "X";
          delBtn.addEventListener("click", async () => {
            try {
              await ReportService.deleteCostRate(rate.id);
              this.costRates = this.costRates.filter(r => r.id !== rate.id);
              renderInlineRates();
              this.renderCostRatesDashboard(
                this.overlay?.querySelector<HTMLElement>('[data-id="mfg-dashboard"]')!
              );
            } catch { ToastContainer.show("error", "Loeschen fehlgeschlagen"); }
          });
          row.appendChild(delBtn);
          section.appendChild(row);
        }

        // Add new rate form
        const addRow = document.createElement("div");
        addRow.style.display = "flex";
        addRow.style.gap = "8px";
        addRow.style.marginTop = "4px";

        const nameInput = document.createElement("input");
        nameInput.className = "mfg-input";
        nameInput.placeholder = "Name";
        nameInput.style.flex = "1";
        addRow.appendChild(nameInput);

        const valInput = document.createElement("input");
        valInput.className = "mfg-input";
        valInput.type = "number";
        valInput.step = "0.01";
        valInput.placeholder = "Wert";
        valInput.style.width = "80px";
        addRow.appendChild(valInput);

        if (type === "machine") {
          const setupInput = document.createElement("input");
          setupInput.className = "mfg-input";
          setupInput.type = "number";
          setupInput.step = "0.01";
          setupInput.placeholder = "Ruestk.";
          setupInput.style.width = "80px";
          addRow.appendChild(setupInput);

          const addBtn = document.createElement("button");
          addBtn.className = "dialog-btn dialog-btn-primary";
          addBtn.style.padding = "2px 12px";
          addBtn.textContent = "+";
          addBtn.addEventListener("click", async () => {
            const n = nameInput.value.trim();
            const v = parseFloat(valInput.value);
            const s = parseFloat(setupInput.value) || 0;
            if (!n || isNaN(v)) return;
            try {
              const created = await ReportService.createCostRate(type, n, v, "EUR/h", s);
              this.costRates.push(created);
              renderInlineRates();
              this.renderCostRatesDashboard(
                this.overlay?.querySelector<HTMLElement>('[data-id="mfg-dashboard"]')!
              );
            } catch { ToastContainer.show("error", "Erstellen fehlgeschlagen"); }
          });
          addRow.appendChild(addBtn);
        } else {
          const addBtn = document.createElement("button");
          addBtn.className = "dialog-btn dialog-btn-primary";
          addBtn.style.padding = "2px 12px";
          addBtn.textContent = "+";
          addBtn.addEventListener("click", async () => {
            const n = nameInput.value.trim();
            const v = parseFloat(valInput.value);
            if (!n || isNaN(v)) return;
            try {
              const unit = type === "stitch" ? "EUR/1000 Stiche" : (type === "overhead" || type === "profit") ? "%" : "EUR/h";
              const created = await ReportService.createCostRate(type, n, v, unit);
              this.costRates.push(created);
              renderInlineRates();
              this.renderCostRatesDashboard(
                this.overlay?.querySelector<HTMLElement>('[data-id="mfg-dashboard"]')!
              );
            } catch { ToastContainer.show("error", "Erstellen fehlgeschlagen"); }
          });
          addRow.appendChild(addBtn);
        }

        section.appendChild(addRow);
        rateSection.appendChild(section);
      }
    };

    renderInlineRates();
    listPane.appendChild(rateSection);
    container.appendChild(listPane);

    // ── Right: Product Cost Calculator ──
    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";

    const calcSection = document.createElement("div");
    calcSection.className = "mfg-form";

    const calcTitle = document.createElement("h3");
    calcTitle.style.marginBottom = "var(--spacing-2)";
    calcTitle.textContent = "Produktkalkulation";
    calcSection.appendChild(calcTitle);

    // Product selector
    const prodRow = document.createElement("div");
    prodRow.className = "mfg-field";
    const prodLabel = document.createElement("label");
    prodLabel.className = "mfg-label";
    prodLabel.textContent = "Produkt:";
    prodRow.appendChild(prodLabel);

    const prodSelect = document.createElement("select");
    prodSelect.className = "mfg-input";
    const emptyProdOpt = document.createElement("option");
    emptyProdOpt.value = "";
    emptyProdOpt.textContent = "Produkt waehlen";
    prodSelect.appendChild(emptyProdOpt);
    for (const p of this.products) {
      const opt = document.createElement("option");
      opt.value = String(p.id);
      opt.textContent = p.name;
      prodSelect.appendChild(opt);
    }
    prodRow.appendChild(prodSelect);
    calcSection.appendChild(prodRow);

    if (this.products.length === 0) {
      const hint = document.createElement("div");
      hint.className = "mfg-tt-hint";
      hint.textContent = "Bitte zuerst ein Produkt im Produkte-Tab anlegen.";
      calcSection.appendChild(hint);
    }

    // Quantity input
    const qtyRow = document.createElement("div");
    qtyRow.className = "mfg-field";
    const qtyLabel = document.createElement("label");
    qtyLabel.className = "mfg-label";
    qtyLabel.textContent = "Menge (Stueck):";
    const qtyInput = document.createElement("input");
    qtyInput.className = "mfg-input";
    qtyInput.type = "number";
    qtyInput.step = "1";
    qtyInput.min = "1";
    qtyInput.value = "1";
    qtyRow.appendChild(qtyLabel);
    qtyRow.appendChild(qtyInput);
    calcSection.appendChild(qtyRow);

    // Result card
    const resultCard = document.createElement("div");
    resultCard.className = "mfg-report-card mfg-kalkulation-card";
    resultCard.style.marginTop = "var(--spacing-2)";
    calcSection.appendChild(resultCard);

    const recalculate = async () => {
      const selectedProductId = prodSelect.value ? Number(prodSelect.value) : null;
      if (!selectedProductId) {
        resultCard.innerHTML = "";
        return;
      }
      try {
        const qty = Math.max(1, parseInt(qtyInput.value) || 1);
        const cb = await ReportService.calculateProductCost(selectedProductId, qty);
        resultCard.innerHTML = "";
        const card = this.createKalkulationCard(cb);
        // Move children from card into resultCard
        while (card.firstChild) resultCard.appendChild(card.firstChild);
      } catch {
        resultCard.innerHTML = "<div class=\"mfg-tt-hint\">Berechnung fehlgeschlagen</div>";
      }
    };

    prodSelect.addEventListener("change", recalculate);
    qtyInput.addEventListener("input", recalculate);
    detailPane.appendChild(calcSection);
    container.appendChild(detailPane);
  }

  // ── Reports Tab ─────────────────────────────────────────────────

  private renderReportsDashboard(container: HTMLElement): void {
    if (this.costBreakdown) {
      this.addBadge(container, `Selbstkosten: ${this.costBreakdown.selbstkosten.toFixed(2)} EUR`, "");
      this.addBadge(container, `Verkaufspreis: ${this.costBreakdown.nettoVerkaufspreis.toFixed(2)} EUR`, "");
    }
  }

  private renderReportsTab(container: HTMLElement): void {
    // Two-column layout: selectors left, results right
    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    listPane.style.width = "300px";

    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";

    // ── Left: Mode selector ──
    const modeRow = document.createElement("div");
    modeRow.style.display = "flex";
    modeRow.style.flexDirection = "column";
    modeRow.style.gap = "var(--spacing-1)";
    modeRow.style.marginBottom = "var(--spacing-2)";

    const modeLabel = document.createElement("label");
    modeLabel.className = "mfg-label";
    modeLabel.textContent = "Modus:";
    modeRow.appendChild(modeLabel);

    for (const m of [{ key: "project" as const, label: "Projekt" }, { key: "product" as const, label: "Produkt" }]) {
      const radioLabel = document.createElement("label");
      radioLabel.style.display = "flex";
      radioLabel.style.alignItems = "center";
      radioLabel.style.gap = "4px";
      const radio = document.createElement("input");
      radio.type = "radio";
      radio.name = "report-mode";
      radio.value = m.key;
      radio.checked = this.reportMode === m.key;
      radio.addEventListener("change", () => {
        this.reportMode = m.key;
        this.costBreakdown = null;
        this.currentReport = null;
        this.renderActiveTab();
      });
      radioLabel.appendChild(radio);
      radioLabel.appendChild(document.createTextNode(m.label));
      modeRow.appendChild(radioLabel);
    }
    listPane.appendChild(modeRow);

    // ── Left: Selector row (project or product) ──
    const selectorRow = document.createElement("div");
    selectorRow.style.marginBottom = "var(--spacing-2)";

    if (this.reportMode === "project") {
      const selectorLabel = document.createElement("label");
      selectorLabel.className = "mfg-label";
      selectorLabel.textContent = "Projekt:";
      selectorRow.appendChild(selectorLabel);
      const projectSelect = document.createElement("select");
      projectSelect.className = "mfg-input";
      const emptyOpt = document.createElement("option"); emptyOpt.value = ""; emptyOpt.textContent = "Projekt waehlen";
      projectSelect.appendChild(emptyOpt);
      for (const p of this.allProjects) {
        const opt = document.createElement("option"); opt.value = String(p.id); opt.textContent = p.name;
        if (this.reportProjectId === p.id) opt.selected = true;
        projectSelect.appendChild(opt);
      }
      projectSelect.addEventListener("change", async () => {
        const id = projectSelect.value ? Number(projectSelect.value) : null;
        this.reportProjectId = id;
        if (id) {
          try {
            this.currentReport = await ReportService.getProjectReport(id);
            this.costBreakdown = this.currentReport.costBreakdown;
          } catch {
            this.currentReport = null;
            this.costBreakdown = null;
            ToastContainer.show("error", "Bericht konnte nicht geladen werden");
          }
        } else { this.currentReport = null; this.costBreakdown = null; }
        this.renderActiveTab();
      });
      selectorRow.appendChild(projectSelect);
    } else {
      const prodLabel = document.createElement("label");
      prodLabel.className = "mfg-label";
      prodLabel.textContent = "Produkt:";
      selectorRow.appendChild(prodLabel);
      const prodSelect = document.createElement("select");
      prodSelect.className = "mfg-input";
      const emptyOpt = document.createElement("option"); emptyOpt.value = ""; emptyOpt.textContent = "Produkt waehlen";
      prodSelect.appendChild(emptyOpt);
      for (const p of this.products) {
        const opt = document.createElement("option"); opt.value = String(p.id); opt.textContent = p.name;
        if (this.reportProductId === p.id) opt.selected = true;
        prodSelect.appendChild(opt);
      }

      const qtyLabel = document.createElement("label");
      qtyLabel.className = "mfg-label";
      qtyLabel.style.marginTop = "var(--spacing-1)";
      qtyLabel.textContent = "Menge:";
      const qtyInput = document.createElement("input");
      qtyInput.className = "mfg-input";
      qtyInput.type = "number";
      qtyInput.min = "1";
      qtyInput.step = "1";
      qtyInput.value = String(this.reportQuantity);
      qtyInput.style.width = "80px";

      const loadProduct = async () => {
        const id = prodSelect.value ? Number(prodSelect.value) : null;
        this.reportProductId = id;
        this.reportQuantity = Math.max(1, parseInt(qtyInput.value) || 1);
        if (id) {
          try {
            this.costBreakdown = await ReportService.calculateProductCost(id, this.reportQuantity);
          } catch {
            this.costBreakdown = null;
            ToastContainer.show("error", "Berechnung fehlgeschlagen");
          }
        } else { this.costBreakdown = null; }
        this.renderActiveTab();
      };

      prodSelect.addEventListener("change", loadProduct);
      qtyInput.addEventListener("change", loadProduct);
      selectorRow.appendChild(prodSelect);
      selectorRow.appendChild(qtyLabel);
      selectorRow.appendChild(qtyInput);
    }
    listPane.appendChild(selectorRow);

    // ── Left: Action buttons ──
    this.renderReportsActions(listPane);

    container.appendChild(listPane);

    // ── Right: Section 1 — Netto-Kosten und Preis ──
    const cb = this.costBreakdown;
    if (!cb) {
      const hint = document.createElement("div");
      hint.className = "mfg-tt-hint";
      hint.textContent = this.reportMode === "project"
        ? (this.reportProjectId ? "Bericht wird geladen..." : "Projekt auswaehlen")
        : (this.reportProductId ? "Berechnung wird geladen..." : "Produkt auswaehlen");
      detailPane.appendChild(hint);
      container.appendChild(detailPane);
      return;
    }

    const section1Title = document.createElement("h3");
    section1Title.style.marginBottom = "var(--spacing-2)";
    section1Title.textContent = "Netto-Kosten und Preis";
    detailPane.appendChild(section1Title);

    const kalkCard = this.createKalkulationCard(cb);
    detailPane.appendChild(kalkCard);

    // ── Right: Section 2 — Verkauf ──
    const section2Title = document.createElement("h3");
    section2Title.style.margin = "var(--spacing-3) 0 var(--spacing-2)";
    section2Title.textContent = "Verkauf";
    detailPane.appendChild(section2Title);

    const verkaufCard = document.createElement("div");
    verkaufCard.className = "mfg-report-card mfg-kalkulation-card";

    // Profit margin input
    const marginRow = document.createElement("div");
    marginRow.style.display = "flex";
    marginRow.style.gap = "var(--spacing-2)";
    marginRow.style.alignItems = "center";
    marginRow.style.marginBottom = "var(--spacing-2)";
    const marginLabel = document.createElement("label");
    marginLabel.className = "mfg-label";
    marginLabel.textContent = "Gewinnspanne (%):";
    const marginInput = document.createElement("input");
    marginInput.className = "mfg-input";
    marginInput.type = "number";
    marginInput.step = "0.1";
    marginInput.min = "0";
    marginInput.value = String(cb.profitMarginPct);
    marginInput.style.width = "80px";
    marginRow.appendChild(marginLabel);
    marginRow.appendChild(marginInput);
    verkaufCard.appendChild(marginRow);

    // Display selling price
    const verkaufDisplay = document.createElement("div");
    const renderVerkauf = (pct: number) => {
      const profitAmt = cb.selbstkosten * (pct / 100);
      const nettoVP = cb.selbstkosten + profitAmt;
      verkaufDisplay.innerHTML = "";
      const lines: { label: string; value: string; cls?: string; separator?: boolean }[] = [
        { label: "Selbstkosten netto", value: `${cb.selbstkosten.toFixed(2)} EUR` },
        { label: `Gewinnzuschlag (${pct.toFixed(1)}%)`, value: `${profitAmt.toFixed(2)} EUR` },
        { label: "Netto-Verkaufspreis", value: `${nettoVP.toFixed(2)} EUR`, cls: "mfg-kalk-total", separator: true },
      ];
      if (cb.quantity > 1) {
        lines.push({ label: `Verkaufspreis/Stueck (${cb.quantity} St.)`, value: `${(nettoVP / cb.quantity).toFixed(2)} EUR` });
      }
      for (const line of lines) {
        if (line.separator) {
          const sep = document.createElement("hr");
          sep.className = "mfg-kalk-separator";
          verkaufDisplay.appendChild(sep);
        }
        const row = document.createElement("div");
        row.className = "mfg-report-row" + (line.cls ? ` ${line.cls}` : "");
        const lbl = document.createElement("span");
        lbl.className = "mfg-report-label";
        lbl.textContent = line.label;
        const val = document.createElement("span");
        val.className = "mfg-report-value";
        val.textContent = line.value;
        row.appendChild(lbl);
        row.appendChild(val);
        verkaufDisplay.appendChild(row);
      }
    };
    renderVerkauf(cb.profitMarginPct);
    marginInput.addEventListener("input", () => {
      const pct = parseFloat(marginInput.value) || 0;
      renderVerkauf(pct);
    });
    verkaufCard.appendChild(verkaufDisplay);
    detailPane.appendChild(verkaufCard);
    container.appendChild(detailPane);
  }

  private renderReportsActions(listPane: HTMLElement): void {
    const exportRow = document.createElement("div");
    exportRow.className = "mfg-actions";
    exportRow.style.marginTop = "var(--spacing-3)";
    exportRow.style.flexWrap = "wrap";

    if (this.reportMode === "project" && this.reportProjectId) {
      // CSV export
      const exportBtn = document.createElement("button");
      exportBtn.className = "dialog-btn dialog-btn-primary";
      exportBtn.textContent = "CSV Export";
      exportBtn.addEventListener("click", async () => {
        if (!this.reportProjectId) return;
        try {
          const csv = await ReportService.exportProjectCsv(this.reportProjectId);
          this.downloadCsv(csv, `projekt_${this.reportProjectId}_bericht.csv`);
          ToastContainer.show("success", "CSV exportiert");
        } catch { ToastContainer.show("error", "Export fehlgeschlagen"); }
      });
      exportRow.appendChild(exportBtn);

      // Save cost snapshot
      const saveBtn = document.createElement("button");
      saveBtn.className = "dialog-btn";
      saveBtn.textContent = "Kalkulation speichern";
      saveBtn.addEventListener("click", async () => {
        if (!this.reportProjectId) return;
        try {
          await ReportService.saveCostBreakdown(this.reportProjectId);
          ToastContainer.show("success", "Kalkulation gespeichert");
        } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
      });
      exportRow.appendChild(saveBtn);

      // Full project export
      const fullExportBtn = document.createElement("button");
      fullExportBtn.className = "dialog-btn";
      fullExportBtn.textContent = "Vollstaendiger Export";
      fullExportBtn.addEventListener("click", async () => {
        if (!this.reportProjectId) return;
        try {
          const csv = await ReportService.exportProjectFullCsv(this.reportProjectId);
          this.downloadCsv(csv, `projekt_${this.reportProjectId}_vollstaendig.csv`);
          ToastContainer.show("success", "Projekt exportiert");
        } catch { ToastContainer.show("error", "Export fehlgeschlagen"); }
      });
      exportRow.appendChild(fullExportBtn);

      // Material usage export
      const usageExportBtn = document.createElement("button");
      usageExportBtn.className = "dialog-btn";
      usageExportBtn.textContent = "Materialverbrauch Export";
      usageExportBtn.addEventListener("click", async () => {
        if (!this.reportProjectId) return;
        try {
          const csv = await ReportService.exportMaterialUsageCsv(this.reportProjectId);
          this.downloadCsv(csv, `materialverbrauch_${this.reportProjectId}.csv`);
          ToastContainer.show("success", "Materialverbrauch exportiert");
        } catch { ToastContainer.show("error", "Export fehlgeschlagen"); }
      });
      exportRow.appendChild(usageExportBtn);
    }

    // Cost rates config button (always visible)
    const ratesBtn = document.createElement("button");
    ratesBtn.className = "dialog-btn";
    ratesBtn.textContent = "Kostensaetze";
    ratesBtn.addEventListener("click", () => {
      this.activeTab = "costrates";
      const tabBar = this.overlay?.querySelector(".mfg-tab-bar");
      if (tabBar) {
        tabBar.querySelectorAll(".mfg-tab").forEach((b) => {
          b.classList.remove("active");
          b.setAttribute("aria-selected", "false");
          if ((b as HTMLElement).dataset.tab === "costrates") {
            b.classList.add("active");
            b.setAttribute("aria-selected", "true");
          }
        });
      }
      this.renderActiveTab();
    });
    exportRow.appendChild(ratesBtn);
    listPane.appendChild(exportRow);
  }

  private createKalkulationCard(cb: CostBreakdown): HTMLElement {
    const card = document.createElement("div");
    card.className = "mfg-report-card mfg-kalkulation-card";

    const h = document.createElement("h4");
    h.className = "mfg-report-card-title";
    h.textContent = "Kalkulation";
    card.appendChild(h);

    const lines: { label: string; value: string; cls?: string; separator?: boolean }[] = [
      { label: "Materialkosten netto", value: `${cb.materialCost.toFixed(2)} EUR` },
      { label: "Lizenzkosten netto", value: `${cb.licenseCost.toFixed(2)} EUR` },
      { label: "Stickkosten netto", value: `${cb.stitchCost.toFixed(2)} EUR` },
      { label: "Arbeitskosten netto", value: `${cb.laborCost.toFixed(2)} EUR` },
      { label: "Maschinenkosten netto", value: `${cb.machineCost.toFixed(2)} EUR` },
      { label: "Herstellkosten", value: `${cb.herstellkosten.toFixed(2)} EUR`, separator: true },
      { label: `Gemeinkosten (${cb.overheadPct.toFixed(1)}%)`, value: `${cb.overheadCost.toFixed(2)} EUR` },
      { label: "Selbstkosten netto", value: `${cb.selbstkosten.toFixed(2)} EUR`, cls: "mfg-kalk-subtotal", separator: true },
      { label: `Gewinnzuschlag (${cb.profitMarginPct.toFixed(1)}%)`, value: `${cb.profitAmount.toFixed(2)} EUR` },
      { label: "Netto-Verkaufspreis", value: `${cb.nettoVerkaufspreis.toFixed(2)} EUR`, cls: "mfg-kalk-total", separator: true },
    ];

    if (cb.quantity > 1) {
      lines.push({ label: `Selbstkosten/Stueck (${cb.quantity} St.)`, value: `${cb.selbstkostenPerPiece.toFixed(2)} EUR` });
      lines.push({ label: `Verkaufspreis/Stueck`, value: `${cb.verkaufspreisPerPiece.toFixed(2)} EUR` });
    }

    for (const line of lines) {
      if (line.separator) {
        const sep = document.createElement("hr");
        sep.className = "mfg-kalk-separator";
        card.appendChild(sep);
      }
      const row = document.createElement("div");
      row.className = "mfg-report-row" + (line.cls ? ` ${line.cls}` : "");
      const lbl = document.createElement("span");
      lbl.className = "mfg-report-label";
      lbl.textContent = line.label;
      const val = document.createElement("span");
      val.className = "mfg-report-value";
      val.textContent = line.value;
      row.appendChild(lbl);
      row.appendChild(val);
      card.appendChild(row);
    }

    return card;
  }

  // ── Helpers ──────────────────────────────────────────────────────

  private downloadCsv(csv: string, filename: string): void {
    const blob = new Blob([csv], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  private renderAuditHistory(container: HTMLElement, entityType: string, entityId: number): void {
    const section = document.createElement("div");
    section.className = "mfg-bom-section";
    const toggle = document.createElement("button");
    toggle.className = "dialog-btn";
    toggle.style.fontSize = "var(--font-size-caption)";
    toggle.textContent = "Aenderungshistorie";
    toggle.addEventListener("click", async () => {
      toggle.style.display = "none";
      try {
        const entries = await ReportService.getAuditLog(entityType, entityId);
        if (entries.length === 0) {
          const hint = document.createElement("div");
          hint.className = "mfg-item-sub";
          hint.textContent = "Keine Aenderungen protokolliert";
          section.appendChild(hint);
          return;
        }
        const table = document.createElement("table");
        table.className = "mfg-bom-table";
        table.style.fontSize = "var(--font-size-caption)";
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
        section.appendChild(table);
      } catch { ToastContainer.show("error", "Historie konnte nicht geladen werden"); }
    });
    section.appendChild(toggle);
    container.appendChild(section);
  }

  private addBadge(container: HTMLElement, text: string, cls: string): void {
    const badge = document.createElement("span");
    badge.className = "mfg-badge " + cls;
    badge.textContent = text;
    container.appendChild(badge);
  }

  private addCreateBtn(
    container: HTMLElement,
    label: string,
    onClick: () => void
  ): void {
    const btn = document.createElement("button");
    btn.className = "dialog-btn dialog-btn-primary mfg-create-btn";
    btn.textContent = `+ ${label}`;
    btn.addEventListener("click", onClick);
    container.appendChild(btn);
  }

  private nextFieldId(): string {
    return `mfg-f-${++this.fieldIdCounter}`;
  }

  private addTextField(
    form: HTMLElement,
    label: string,
    value: string,
    onSave: (v: string) => Promise<void>
  ): void {
    const id = this.nextFieldId();
    const group = document.createElement("div");
    group.className = "mfg-field";
    const lbl = document.createElement("label");
    lbl.className = "mfg-label";
    lbl.textContent = label;
    lbl.htmlFor = id;
    const input = document.createElement("input");
    input.type = "text";
    input.id = id;
    input.className = "mfg-input";
    input.value = value;
    input.addEventListener("change", () => onSave(input.value));
    group.appendChild(lbl);
    group.appendChild(input);
    form.appendChild(group);
  }

  private addNumberField(
    form: HTMLElement,
    label: string,
    value: number | null | undefined,
    onSave: (v: number | undefined) => Promise<void>
  ): void {
    const id = this.nextFieldId();
    const group = document.createElement("div");
    group.className = "mfg-field";
    const lbl = document.createElement("label");
    lbl.className = "mfg-label";
    lbl.textContent = label;
    lbl.htmlFor = id;
    const input = document.createElement("input");
    input.type = "number";
    input.id = id;
    input.className = "mfg-input";
    input.step = "any";
    input.value = value != null ? String(value) : "";
    input.addEventListener("change", () => {
      const v = input.value ? Number(input.value) : undefined;
      onSave(v);
    });
    group.appendChild(lbl);
    group.appendChild(input);
    form.appendChild(group);
  }

  private addSelectField(
    form: HTMLElement,
    label: string,
    value: string,
    options: { value: string; label: string }[],
    onSave: (v: string) => Promise<void>
  ): void {
    const id = this.nextFieldId();
    const group = document.createElement("div");
    group.className = "mfg-field";
    const lbl = document.createElement("label");
    lbl.className = "mfg-label";
    lbl.textContent = label;
    lbl.htmlFor = id;
    const select = document.createElement("select");
    select.id = id;
    select.className = "mfg-input";
    for (const opt of options) {
      const o = document.createElement("option");
      o.value = opt.value;
      o.textContent = opt.label;
      if (opt.value === value) o.selected = true;
      select.appendChild(o);
    }
    select.addEventListener("change", () => onSave(select.value));
    group.appendChild(lbl);
    group.appendChild(select);
    form.appendChild(group);
  }

  private addTextArea(
    form: HTMLElement,
    label: string,
    value: string,
    onSave: (v: string) => Promise<void>
  ): void {
    const id = this.nextFieldId();
    const group = document.createElement("div");
    group.className = "mfg-field";
    const lbl = document.createElement("label");
    lbl.className = "mfg-label";
    lbl.textContent = label;
    lbl.htmlFor = id;
    const textarea = document.createElement("textarea");
    textarea.id = id;
    textarea.className = "mfg-input";
    textarea.rows = 3;
    textarea.value = value;
    textarea.addEventListener("change", () => onSave(textarea.value));
    group.appendChild(lbl);
    group.appendChild(textarea);
    form.appendChild(group);
  }

  private materialTypeLabel(type: string): string {
    const map: Record<string, string> = {
      fabric: "Stoff",
      thread: "Garn",
      embroidery_thread: "Stickgarn",
      vlies: "Vlies",
      zipper: "Reissverschluss",
      button: "Knopf",
      label: "Etikett",
      other: "Sonstiges",
    };
    return map[type] || type;
  }

  private productTypeLabel(type: string): string {
    const map: Record<string, string> = {
      naehprodukt: "Naehprodukt",
      stickprodukt: "Stickprodukt",
      kombiprodukt: "Kombiprodukt",
    };
    return map[type] || type;
  }

  private bomTypeLabel(type: string): string {
    const map: Record<string, string> = {
      material: "Material",
      work_step: "Arbeitsschritt",
      machine_time: "Maschinenzeit",
      pattern: "Stickmuster",
      cutting_template: "Schnittvorlage",
    };
    return map[type] || type;
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
