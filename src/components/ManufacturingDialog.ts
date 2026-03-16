import * as MfgService from "../services/ManufacturingService";
import { ToastContainer } from "./Toast";
import * as ProjectService from "../services/ProjectService";
import * as ProcService from "../services/ProcurementService";
import * as ReportService from "../services/ReportService";
import type {
  Supplier,
  Material,
  MaterialInventory,
  Product,
  BillOfMaterial,
  Project,
  TimeEntry,
  StepDefinition,
  PurchaseOrder,
  OrderItem,
  LicenseRecord,
  QualityInspection,
  DefectRecord,
  ProjectReport,
} from "../types";

type TabKey = "materials" | "suppliers" | "products" | "inventory" | "timetracking" | "workflow" | "orders" | "licenses" | "quality" | "reports";

export class ManufacturingDialog {
  private static instance: ManufacturingDialog | null = null;

  private overlay: HTMLElement | null = null;
  private keyHandler: ((e: KeyboardEvent) => void) | null = null;

  private activeTab: TabKey = "materials";

  // Data caches
  private materials: Material[] = [];
  private suppliers: Supplier[] = [];
  private products: Product[] = [];
  private inventoryMap: Map<number, MaterialInventory> = new Map();
  private bomMap: Map<number, BillOfMaterial[]> = new Map();

  // Selection state
  private selectedMaterial: Material | null = null;
  private selectedSupplier: Supplier | null = null;
  private selectedProduct: Product | null = null;
  private fieldIdCounter = 0;

  // Time tracking state
  private allProjects: Project[] = [];
  private ttSelectedProjectId: number | null = null;
  private timeEntries: TimeEntry[] = [];
  private selectedTimeEntry: TimeEntry | null = null;

  // Workflow state
  private stepDefs: StepDefinition[] = [];
  private selectedStepDef: StepDefinition | null = null;

  // Orders state
  private orders: PurchaseOrder[] = [];
  private selectedOrder: PurchaseOrder | null = null;
  private orderItems: Map<number, OrderItem[]> = new Map();

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
    this.keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        ManufacturingDialog.dismiss();
      }
    };
    document.addEventListener("keydown", this.keyHandler);
  }

  private async loadAll(): Promise<void> {
    [this.materials, this.suppliers, this.products, this.allProjects, this.stepDefs, this.orders, this.licenses] =
      await Promise.all([
        MfgService.getMaterials(),
        MfgService.getSuppliers(),
        MfgService.getProducts(),
        ProjectService.getProjects(),
        MfgService.getStepDefs(),
        ProcService.getOrders(),
        MfgService.getLicenses(),
      ]);
    // Load inventory for all materials
    this.inventoryMap.clear();
    await Promise.all(
      this.materials.map(async (m) => {
        try {
          const inv = await MfgService.getInventory(m.id);
          this.inventoryMap.set(m.id, inv);
        } catch {
          // No inventory record yet
        }
      })
    );
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
      { key: "inventory", label: "Inventar" },
      { key: "timetracking", label: "Zeiterfassung" },
      { key: "workflow", label: "Workflow" },
      { key: "orders", label: "Bestellungen" },
      { key: "licenses", label: "Lizenzen" },
      { key: "quality", label: "Qualitaet" },
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
      case "inventory":
        this.renderInventoryDashboard(dashboard);
        this.renderInventoryTab(content);
        break;
      case "timetracking":
        this.renderTimeTrackingDashboard(dashboard);
        this.renderTimeTrackingTab(content);
        break;
      case "workflow":
        this.renderWorkflowDashboard(dashboard);
        this.renderWorkflowTab(content);
        break;
      case "orders":
        this.renderOrdersDashboard(dashboard);
        this.renderOrdersTab(content);
        break;
      case "licenses":
        this.renderLicensesDashboard(dashboard);
        this.renderLicensesTab(content);
        break;
      case "quality":
        this.renderQualityDashboard(dashboard);
        this.renderQualityTab(content);
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
    let lowStock = 0;
    for (const m of this.materials) {
      const inv = this.inventoryMap.get(m.id);
      if (inv && m.minStock && inv.totalStock - inv.reservedStock < m.minStock) {
        lowStock++;
      }
    }
    this.addBadge(container, `Gesamt: ${total}`, "");
    if (lowStock > 0) {
      this.addBadge(container, `Niedriger Bestand: ${lowStock}`, "mfg-badge-warn");
    }
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

      const inv = this.inventoryMap.get(m.id);
      const available = inv ? inv.totalStock - inv.reservedStock : 0;
      const isLow = m.minStock != null && m.minStock > 0 && available < m.minStock;
      const isWarn =
        m.minStock != null && m.minStock > 0 && available < m.minStock * 2 && !isLow;

      const dot = document.createElement("span");
      dot.className = "mfg-stock-dot" +
        (isLow ? " mfg-stock-low" : isWarn ? " mfg-stock-warn" : " mfg-stock-ok");
      item.appendChild(dot);

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

    // Inventory section
    const inv = this.inventoryMap.get(m.id);
    if (inv) {
      const invSection = document.createElement("div");
      invSection.className = "mfg-inv-section";
      const invTitle = document.createElement("h4");
      invTitle.className = "mfg-section-title";
      invTitle.textContent = "Bestand";
      invSection.appendChild(invTitle);

      const available = inv.totalStock - inv.reservedStock;
      this.addNumberField(invSection, "Gesamtbestand", inv.totalStock, (v) =>
        this.updateInv(m.id, v, undefined, undefined)
      );
      this.addNumberField(invSection, "Reserviert", inv.reservedStock, (v) =>
        this.updateInv(m.id, undefined, v, undefined)
      );
      const availEl = document.createElement("div");
      availEl.className = "mfg-field";
      const availLabel = document.createElement("label");
      availLabel.className = "mfg-label";
      availLabel.textContent = "Verfuegbar";
      const availVal = document.createElement("span");
      availVal.className = "mfg-readonly-value";
      availVal.textContent = String(available);
      availEl.appendChild(availLabel);
      availEl.appendChild(availVal);
      invSection.appendChild(availEl);

      this.addTextField(invSection, "Lagerort", inv.location || "", (v) =>
        this.updateInv(m.id, undefined, undefined, v)
      );
      container.appendChild(invSection);
    }

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

  private async updateInv(
    materialId: number,
    totalStock?: number,
    reservedStock?: number,
    location?: string
  ): Promise<void> {
    try {
      const inv = await MfgService.updateInventory(
        materialId,
        totalStock,
        reservedStock,
        location
      );
      this.inventoryMap.set(materialId, inv);
      this.renderActiveTab();
    } catch (e) {
      ToastContainer.show("error", "Bestand konnte nicht aktualisiert werden");
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
    if (entries.length > 0) {
      const table = document.createElement("table");
      table.className = "mfg-bom-table";
      table.innerHTML =
        "<thead><tr><th>Material</th><th>Menge</th><th>Einheit</th><th></th></tr></thead>";
      const tbody = document.createElement("tbody");
      for (const bom of entries) {
        const mat = this.materials.find((m) => m.id === bom.materialId);
        const tr = document.createElement("tr");
        const tdName = document.createElement("td");
        tdName.textContent = mat?.name || "?";
        const tdQty = document.createElement("td");
        tdQty.textContent = String(bom.quantity);
        const tdUnit = document.createElement("td");
        tdUnit.textContent = bom.unit || "";
        tr.appendChild(tdName);
        tr.appendChild(tdQty);
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
      empty.textContent = "Keine Materialien zugeordnet";
      bomSection.appendChild(empty);
    }

    // Add BOM entry form
    const addRow = document.createElement("div");
    addRow.className = "mfg-bom-add";
    const matSelect = document.createElement("select");
    matSelect.className = "mfg-input";
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
    addRow.appendChild(matSelect);

    const qtyInput = document.createElement("input");
    qtyInput.type = "number";
    qtyInput.className = "mfg-input mfg-input-sm";
    qtyInput.placeholder = "Menge";
    qtyInput.min = "0.01";
    qtyInput.step = "0.01";
    addRow.appendChild(qtyInput);

    const unitInput = document.createElement("input");
    unitInput.type = "text";
    unitInput.className = "mfg-input mfg-input-sm";
    unitInput.placeholder = "Einheit";
    addRow.appendChild(unitInput);

    const addBtn = document.createElement("button");
    addBtn.className = "dialog-btn dialog-btn-primary";
    addBtn.textContent = "+";
    addBtn.addEventListener("click", async () => {
      const matId = Number(matSelect.value);
      const qty = Number(qtyInput.value);
      if (!matId || !qty || qty <= 0) {
        ToastContainer.show("error", "Material und Menge angeben");
        return;
      }
      try {
        await MfgService.addBomEntry(
          p.id,
          matId,
          qty,
          unitInput.value || undefined
        );
        this.bomMap.set(p.id, await MfgService.getBomEntries(p.id));
        this.renderActiveTab();
      } catch (e) {
        ToastContainer.show("error", "BOM-Eintrag fehlgeschlagen");
      }
    });
    addRow.appendChild(addBtn);
    bomSection.appendChild(addRow);
    container.appendChild(bomSection);

    // Actions
    const actions = document.createElement("div");
    actions.className = "mfg-actions";
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

  // ── Inventory Tab ────────────────────────────────────────────────

  private renderInventoryDashboard(container: HTMLElement): void {
    let lowCount = 0;
    for (const m of this.materials) {
      const inv = this.inventoryMap.get(m.id);
      if (inv && m.minStock && inv.totalStock - inv.reservedStock < m.minStock) {
        lowCount++;
      }
    }
    this.addBadge(container, `Materialien: ${this.materials.length}`, "");
    if (lowCount > 0) {
      this.addBadge(container, `Unter Mindestbestand: ${lowCount}`, "mfg-badge-warn");
    }
  }

  private renderInventoryTab(container: HTMLElement): void {
    container.className = "mfg-content mfg-content-single";
    if (this.materials.length === 0) {
      container.textContent = "Keine Materialien vorhanden";
      return;
    }
    const table = document.createElement("table");
    table.className = "mfg-inv-table";
    table.innerHTML =
      "<thead><tr>" +
      "<th>Material</th><th>Typ</th><th>Gesamt</th><th>Reserviert</th>" +
      "<th>Verfuegbar</th><th>Mindest</th><th>Lagerort</th><th>Status</th>" +
      "</tr></thead>";

    const tbody = document.createElement("tbody");
    for (const m of this.materials) {
      const inv = this.inventoryMap.get(m.id);
      const total = inv?.totalStock ?? 0;
      const reserved = inv?.reservedStock ?? 0;
      const available = total - reserved;
      const minStock = m.minStock ?? 0;

      let statusClass = "mfg-inv-ok";
      let statusLabel = "OK";
      if (minStock > 0) {
        if (available < minStock) {
          statusClass = "mfg-inv-low";
          statusLabel = "Niedrig";
        } else if (available < minStock * 2) {
          statusClass = "mfg-inv-warn";
          statusLabel = "Warnung";
        }
      }

      const tr = document.createElement("tr");
      tr.className = statusClass;
      const cells = [
        m.name,
        this.materialTypeLabel(m.materialType || ""),
        String(total),
        String(reserved),
        null, // available — special handling
        minStock ? String(minStock) : "-",
        inv?.location || "-",
        null, // status — special handling
      ];
      for (let ci = 0; ci < cells.length; ci++) {
        const td = document.createElement("td");
        if (ci === 4) {
          const strong = document.createElement("strong");
          strong.textContent = String(available);
          td.appendChild(strong);
        } else if (ci === 7) {
          const span = document.createElement("span");
          span.className = "mfg-inv-status " + statusClass;
          span.textContent = statusLabel;
          td.appendChild(span);
        } else {
          td.textContent = cells[ci]!;
        }
        tr.appendChild(td);
      }
      tbody.appendChild(tr);
    }
    table.appendChild(tbody);
    container.appendChild(table);
  }

  // ── Time Tracking Tab ─────────────────────────────────────────────

  private renderTimeTrackingDashboard(container: HTMLElement): void {
    let totalPlanned = 0;
    let totalActual = 0;
    for (const e of this.timeEntries) {
      totalPlanned += e.plannedMinutes ?? 0;
      totalActual += e.actualMinutes ?? 0;
    }
    this.addBadge(container, `Eintraege: ${this.timeEntries.length}`, "");
    if (this.timeEntries.length > 0) {
      this.addBadge(
        container,
        `Geplant: ${this.fmtHours(totalPlanned)}`,
        ""
      );
      this.addBadge(
        container,
        `Tatsaechlich: ${this.fmtHours(totalActual)}`,
        totalActual > totalPlanned && totalPlanned > 0 ? "mfg-badge-warn" : ""
      );
    }
    this.addCreateBtn(container, "Zeiteintrag", () => this.createTimeEntry());
  }

  private renderTimeTrackingTab(container: HTMLElement): void {
    // Project selector at top
    const selectorRow = document.createElement("div");
    selectorRow.className = "mfg-tt-selector";
    const selectorLabel = document.createElement("label");
    selectorLabel.className = "mfg-label";
    selectorLabel.textContent = "Projekt:";
    selectorRow.appendChild(selectorLabel);

    const projectSelect = document.createElement("select");
    projectSelect.className = "mfg-input";
    const emptyOpt = document.createElement("option");
    emptyOpt.value = "";
    emptyOpt.textContent = "Projekt waehlen";
    projectSelect.appendChild(emptyOpt);
    for (const p of this.allProjects) {
      const opt = document.createElement("option");
      opt.value = String(p.id);
      opt.textContent = p.name;
      if (this.ttSelectedProjectId === p.id) opt.selected = true;
      projectSelect.appendChild(opt);
    }
    projectSelect.addEventListener("change", async () => {
      const id = projectSelect.value ? Number(projectSelect.value) : null;
      this.ttSelectedProjectId = id;
      this.selectedTimeEntry = null;
      if (id) {
        try {
          this.timeEntries = await MfgService.getTimeEntries(id);
        } catch {
          this.timeEntries = [];
          ToastContainer.show("error", "Zeiteintraege konnten nicht geladen werden");
        }
      } else {
        this.timeEntries = [];
      }
      this.renderActiveTab();
    });
    selectorRow.appendChild(projectSelect);
    container.insertBefore(selectorRow, container.firstChild);

    if (!this.ttSelectedProjectId) {
      const hint = document.createElement("div");
      hint.className = "mfg-tt-hint";
      hint.textContent = "Projekt auswaehlen, um Zeiteintraege anzuzeigen";
      container.appendChild(hint);
      return;
    }

    // List + detail layout
    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    this.renderTimeEntryList(listPane);
    container.appendChild(listPane);

    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";
    if (this.selectedTimeEntry) {
      this.renderTimeEntryDetail(detailPane, this.selectedTimeEntry);
    } else {
      detailPane.textContent = "Eintrag auswaehlen";
    }
    container.appendChild(detailPane);
  }

  private renderTimeEntryList(container: HTMLElement): void {
    container.innerHTML = "";
    if (this.timeEntries.length === 0) {
      container.textContent = "Keine Zeiteintraege";
      return;
    }
    for (const e of this.timeEntries) {
      const item = document.createElement("div");
      item.className = "mfg-item";
      if (this.selectedTimeEntry?.id === e.id) item.classList.add("selected");

      const info = document.createElement("div");
      info.className = "mfg-item-info";
      const nameEl = document.createElement("span");
      nameEl.className = "mfg-item-name";
      nameEl.textContent = e.stepName;
      info.appendChild(nameEl);

      const sub = document.createElement("span");
      sub.className = "mfg-item-sub";
      const planned = e.plannedMinutes ?? 0;
      const actual = e.actualMinutes ?? 0;
      sub.textContent = `${this.fmtHours(planned)} geplant / ${this.fmtHours(actual)} tatsaechlich`;
      info.appendChild(sub);

      if (e.worker) {
        const workerEl = document.createElement("span");
        workerEl.className = "mfg-item-sub";
        workerEl.textContent = e.worker;
        info.appendChild(workerEl);
      }

      // Progress bar
      if (planned > 0) {
        const bar = document.createElement("div");
        bar.className = "mfg-tt-bar";
        const pctRaw = Math.round((actual / planned) * 100);
        bar.title = `${pctRaw}% (${this.fmtHours(actual)} / ${this.fmtHours(planned)})`;
        const fill = document.createElement("div");
        fill.className = "mfg-tt-bar-fill";
        const pct = Math.min(pctRaw, 100);
        fill.style.width = pct + "%";
        if (actual > planned) fill.classList.add("mfg-tt-bar-over");
        bar.appendChild(fill);
        info.appendChild(bar);
      }

      item.appendChild(info);
      item.addEventListener("click", () => {
        this.selectedTimeEntry = e;
        this.renderActiveTab();
      });
      container.appendChild(item);
    }
  }

  private renderTimeEntryDetail(container: HTMLElement, e: TimeEntry): void {
    container.innerHTML = "";
    const form = document.createElement("div");
    form.className = "mfg-form";

    this.addTextField(form, "Arbeitsschritt", e.stepName, (v) =>
      this.updateTimeEntry(e.id, { stepName: v })
    );
    this.addNumberField(form, "Geplante Minuten", e.plannedMinutes, (v) =>
      this.updateTimeEntry(e.id, { plannedMinutes: v })
    );
    this.addNumberField(form, "Tatsaechliche Minuten", e.actualMinutes, (v) =>
      this.updateTimeEntry(e.id, { actualMinutes: v })
    );
    this.addTextField(form, "Mitarbeiter", e.worker || "", (v) =>
      this.updateTimeEntry(e.id, { worker: v })
    );
    this.addTextField(form, "Maschine", e.machine || "", (v) =>
      this.updateTimeEntry(e.id, { machine: v })
    );

    // Summary
    const planned = e.plannedMinutes ?? 0;
    const actual = e.actualMinutes ?? 0;
    const diff = actual - planned;
    if (planned > 0 || actual > 0) {
      const summary = document.createElement("div");
      summary.className = "mfg-tt-summary";
      const diffLabel = document.createElement("span");
      diffLabel.className =
        "mfg-tt-diff" + (diff > 0 ? " mfg-tt-diff-over" : " mfg-tt-diff-under");
      diffLabel.textContent =
        diff > 0
          ? `+${this.fmtHours(diff)} ueber Plan`
          : diff < 0
            ? `${this.fmtHours(Math.abs(diff))} unter Plan`
            : "Im Plan";
      summary.appendChild(diffLabel);
      form.appendChild(summary);
    }

    container.appendChild(form);

    // Actions
    const actions = document.createElement("div");
    actions.className = "mfg-actions";
    const delBtn = document.createElement("button");
    delBtn.className = "dialog-btn dialog-btn-danger";
    delBtn.textContent = "Eintrag loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm(`Zeiteintrag "${e.stepName}" wirklich loeschen?`)) return;
      try {
        await MfgService.deleteTimeEntry(e.id);
        this.selectedTimeEntry = null;
        if (this.ttSelectedProjectId) {
          this.timeEntries = await MfgService.getTimeEntries(
            this.ttSelectedProjectId
          );
        }
        this.renderActiveTab();
        ToastContainer.show("success", "Zeiteintrag geloescht");
      } catch {
        ToastContainer.show("error", "Loeschen fehlgeschlagen");
      }
    });
    actions.appendChild(delBtn);
    container.appendChild(actions);
  }

  private async createTimeEntry(): Promise<void> {
    if (!this.ttSelectedProjectId) {
      ToastContainer.show("error", "Bitte zuerst ein Projekt waehlen");
      return;
    }
    try {
      const entry = await MfgService.createTimeEntry({
        projectId: this.ttSelectedProjectId,
        stepName: "Neuer Arbeitsschritt",
      });
      this.timeEntries = await MfgService.getTimeEntries(
        this.ttSelectedProjectId
      );
      this.selectedTimeEntry =
        this.timeEntries.find((x) => x.id === entry.id) || null;
      this.renderActiveTab();
      ToastContainer.show("success", "Zeiteintrag erstellt");
    } catch {
      ToastContainer.show("error", "Erstellen fehlgeschlagen");
    }
  }

  private async updateTimeEntry(
    id: number,
    update: {
      stepName?: string;
      plannedMinutes?: number;
      actualMinutes?: number;
      worker?: string;
      machine?: string;
    }
  ): Promise<void> {
    try {
      const updated = await MfgService.updateTimeEntry(
        id,
        update.stepName,
        update.plannedMinutes,
        update.actualMinutes,
        update.worker,
        update.machine
      );
      const idx = this.timeEntries.findIndex((x) => x.id === id);
      if (idx >= 0) this.timeEntries[idx] = updated;
      this.selectedTimeEntry = updated;
      this.renderActiveTab();
    } catch {
      ToastContainer.show("error", "Speichern fehlgeschlagen");
    }
  }

  private fmtHours(minutes: number): string {
    if (minutes < 60) return `${Math.round(minutes)}min`;
    const h = Math.floor(minutes / 60);
    const m = Math.round(minutes % 60);
    return m > 0 ? `${h}h ${m}min` : `${h}h`;
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

  // ── Orders Tab ───────────────────────────────────────────────────

  private renderOrdersDashboard(container: HTMLElement): void {
    const open = this.orders.filter((o) => o.status !== "delivered" && o.status !== "cancelled").length;
    this.addBadge(container, `Gesamt: ${this.orders.length}`, "");
    if (open > 0) this.addBadge(container, `Offen: ${open}`, "mfg-badge-warn");
    this.addCreateBtn(container, "Bestellung", () => this.createOrder());
  }

  private renderOrdersTab(container: HTMLElement): void {
    const listPane = document.createElement("div");
    listPane.className = "mfg-list-pane";
    this.renderOrderList(listPane);
    container.appendChild(listPane);

    const detailPane = document.createElement("div");
    detailPane.className = "mfg-detail-pane";
    if (this.selectedOrder) {
      this.renderOrderDetail(detailPane, this.selectedOrder);
    } else {
      detailPane.textContent = "Bestellung auswaehlen";
    }
    container.appendChild(detailPane);
  }

  private renderOrderList(container: HTMLElement): void {
    container.innerHTML = "";
    if (this.orders.length === 0) { container.textContent = "Keine Bestellungen"; return; }
    const statusLabels: Record<string, string> = {
      draft: "Entwurf", ordered: "Bestellt", partially_delivered: "Teilgeliefert",
      delivered: "Geliefert", cancelled: "Storniert",
    };
    for (const o of this.orders) {
      const item = document.createElement("div");
      item.className = "mfg-item";
      if (this.selectedOrder?.id === o.id) item.classList.add("selected");
      const info = document.createElement("div");
      info.className = "mfg-item-info";
      const nameEl = document.createElement("span");
      nameEl.className = "mfg-item-name";
      nameEl.textContent = o.orderNumber || `#${o.id}`;
      info.appendChild(nameEl);
      const sub = document.createElement("span");
      sub.className = "mfg-item-sub";
      const supplier = this.suppliers.find((s) => s.id === o.supplierId);
      sub.textContent = `${supplier?.name || "?"} - ${statusLabels[o.status] || o.status}`;
      info.appendChild(sub);
      item.appendChild(info);
      item.addEventListener("click", async () => {
        try {
          this.selectedOrder = o;
          if (!this.orderItems.has(o.id)) {
            this.orderItems.set(o.id, await ProcService.getOrderItems(o.id));
          }
          this.renderActiveTab();
        } catch { ToastContainer.show("error", "Positionen konnten nicht geladen werden"); }
      });
      container.appendChild(item);
    }
  }

  private renderOrderDetail(container: HTMLElement, o: PurchaseOrder): void {
    container.innerHTML = "";
    const form = document.createElement("div");
    form.className = "mfg-form";

    this.addTextField(form, "Bestellnummer", o.orderNumber || "", async (v) => {
      try {
        const updated = await ProcService.updateOrder(o.id, { orderNumber: v });
        const idx = this.orders.findIndex((x) => x.id === o.id);
        if (idx >= 0) this.orders[idx] = updated;
        this.selectedOrder = updated;
        this.renderActiveTab();
      } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    this.addSelectField(form, "Status", o.status, [
      { value: "draft", label: "Entwurf" },
      { value: "ordered", label: "Bestellt" },
      { value: "partially_delivered", label: "Teilgeliefert" },
      { value: "delivered", label: "Geliefert" },
      { value: "cancelled", label: "Storniert" },
    ], async (v) => {
      try {
        const updated = await ProcService.updateOrder(o.id, { status: v });
        const idx = this.orders.findIndex((x) => x.id === o.id);
        if (idx >= 0) this.orders[idx] = updated;
        this.selectedOrder = updated;
        this.renderActiveTab();
      } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    this.addTextField(form, "Bestelldatum", o.orderDate || "", async (v) => {
      try { await ProcService.updateOrder(o.id, { orderDate: v }); } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    this.addTextField(form, "Erwartete Lieferung", o.expectedDelivery || "", async (v) => {
      try { await ProcService.updateOrder(o.id, { expectedDelivery: v }); } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    this.addTextArea(form, "Notizen", o.notes || "", async (v) => {
      try { await ProcService.updateOrder(o.id, { notes: v }); } catch { ToastContainer.show("error", "Speichern fehlgeschlagen"); }
    });
    container.appendChild(form);

    // Order items
    const itemsSection = document.createElement("div");
    itemsSection.className = "mfg-bom-section";
    const itemsTitle = document.createElement("h4");
    itemsTitle.className = "mfg-section-title";
    itemsTitle.textContent = "Positionen";
    itemsSection.appendChild(itemsTitle);

    const items = this.orderItems.get(o.id) || [];
    if (items.length > 0) {
      const table = document.createElement("table");
      table.className = "mfg-bom-table";
      const oThead = document.createElement("thead");
      const oHeadRow = document.createElement("tr");
      for (const h of ["Material", "Bestellt", "Geliefert", "Preis", ""]) {
        const th = document.createElement("th"); th.textContent = h; oHeadRow.appendChild(th);
      }
      oThead.appendChild(oHeadRow);
      table.appendChild(oThead);
      const tbody = document.createElement("tbody");
      for (const oi of items) {
        const mat = this.materials.find((m) => m.id === oi.materialId);
        const tr = document.createElement("tr");
        const tdMat = document.createElement("td"); tdMat.textContent = mat?.name || "?";
        const tdOrd = document.createElement("td"); tdOrd.textContent = String(oi.quantityOrdered);
        const tdDel = document.createElement("td"); tdDel.textContent = String(oi.quantityDelivered);
        const tdPrice = document.createElement("td"); tdPrice.textContent = oi.unitPrice != null ? oi.unitPrice.toFixed(2) : "-";
        const tdAction = document.createElement("td");
        const rmBtn = document.createElement("button");
        rmBtn.className = "mfg-bom-remove";
        rmBtn.textContent = "\u2716";
        rmBtn.addEventListener("click", async () => {
          try {
            await ProcService.deleteOrderItem(oi.id);
            this.orderItems.set(o.id, await ProcService.getOrderItems(o.id));
            this.renderActiveTab();
          } catch { ToastContainer.show("error", "Position konnte nicht entfernt werden"); }
        });
        tdAction.appendChild(rmBtn);
        tr.appendChild(tdMat); tr.appendChild(tdOrd); tr.appendChild(tdDel); tr.appendChild(tdPrice); tr.appendChild(tdAction);
        tbody.appendChild(tr);
      }
      table.appendChild(tbody);
      itemsSection.appendChild(table);
    }

    // Add item form
    const addRow = document.createElement("div");
    addRow.className = "mfg-bom-add";
    const matSelect = document.createElement("select");
    matSelect.className = "mfg-input";
    const defOpt = document.createElement("option"); defOpt.value = ""; defOpt.textContent = "Material";
    matSelect.appendChild(defOpt);
    for (const m of this.materials) {
      const opt = document.createElement("option"); opt.value = String(m.id); opt.textContent = m.name;
      matSelect.appendChild(opt);
    }
    addRow.appendChild(matSelect);
    const qtyInput = document.createElement("input"); qtyInput.type = "number"; qtyInput.className = "mfg-input mfg-input-sm"; qtyInput.placeholder = "Menge";
    addRow.appendChild(qtyInput);
    const priceInput = document.createElement("input"); priceInput.type = "number"; priceInput.className = "mfg-input mfg-input-sm"; priceInput.placeholder = "Preis";
    addRow.appendChild(priceInput);
    const addBtn = document.createElement("button");
    addBtn.className = "dialog-btn dialog-btn-primary"; addBtn.textContent = "+";
    addBtn.addEventListener("click", async () => {
      const matId = Number(matSelect.value);
      const qty = Number(qtyInput.value);
      if (!matId || !qty || qty <= 0) { ToastContainer.show("error", "Material und Menge angeben"); return; }
      try {
        await ProcService.addOrderItem(o.id, matId, qty, priceInput.value ? Number(priceInput.value) : undefined);
        this.orderItems.set(o.id, await ProcService.getOrderItems(o.id));
        this.renderActiveTab();
      } catch { ToastContainer.show("error", "Position hinzufuegen fehlgeschlagen"); }
    });
    addRow.appendChild(addBtn);
    itemsSection.appendChild(addRow);
    container.appendChild(itemsSection);

    // Actions
    const actions = document.createElement("div");
    actions.className = "mfg-actions";
    const delBtn = document.createElement("button");
    delBtn.className = "dialog-btn dialog-btn-danger";
    delBtn.textContent = "Bestellung loeschen";
    delBtn.addEventListener("click", async () => {
      if (!confirm("Bestellung wirklich loeschen?")) return;
      try {
        await ProcService.deleteOrder(o.id);
        this.selectedOrder = null;
        this.orders = await ProcService.getOrders();
        this.renderActiveTab();
        ToastContainer.show("success", "Bestellung geloescht");
      } catch { ToastContainer.show("error", "Loeschen fehlgeschlagen"); }
    });
    actions.appendChild(delBtn);
    container.appendChild(actions);
  }

  private async createOrder(): Promise<void> {
    if (this.suppliers.length === 0) {
      ToastContainer.show("error", "Bitte zuerst einen Lieferanten anlegen");
      return;
    }
    try {
      const o = await ProcService.createOrder({ supplierId: this.suppliers[0].id });
      this.orders = await ProcService.getOrders();
      this.selectedOrder = this.orders.find((x) => x.id === o.id) || null;
      this.renderActiveTab();
      ToastContainer.show("success", "Bestellung erstellt");
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

  // ── Reports Tab ─────────────────────────────────────────────────

  private renderReportsDashboard(container: HTMLElement): void {
    if (this.currentReport) {
      this.addBadge(container, `Kosten: ${this.currentReport.totalCost.toFixed(2)} EUR`, "");
      const pct = this.currentReport.workflowTotal > 0
        ? Math.round((this.currentReport.workflowCompleted / this.currentReport.workflowTotal) * 100) : 0;
      this.addBadge(container, `Fortschritt: ${pct}%`, "");
    }
  }

  private renderReportsTab(container: HTMLElement): void {
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
      if (this.reportProjectId === p.id) opt.selected = true;
      projectSelect.appendChild(opt);
    }
    projectSelect.addEventListener("change", async () => {
      const id = projectSelect.value ? Number(projectSelect.value) : null;
      this.reportProjectId = id;
      if (id) {
        try { this.currentReport = await ReportService.getProjectReport(id); } catch { this.currentReport = null; ToastContainer.show("error", "Bericht konnte nicht geladen werden"); }
      } else { this.currentReport = null; }
      this.renderActiveTab();
    });
    selectorRow.appendChild(projectSelect);
    container.insertBefore(selectorRow, container.firstChild);

    if (!this.currentReport) {
      const hint = document.createElement("div");
      hint.className = "mfg-tt-hint";
      hint.textContent = this.reportProjectId ? "Bericht wird geladen..." : "Projekt auswaehlen";
      container.appendChild(hint);
      return;
    }

    const r = this.currentReport;
    container.className = "mfg-content mfg-content-single";

    // Report cards
    const cards = document.createElement("div");
    cards.className = "mfg-report-cards";

    // Time card
    const timeCard = this.createReportCard("Zeit", [
      { label: "Geplant", value: this.fmtHours(r.totalPlannedMinutes) },
      { label: "Tatsaechlich", value: this.fmtHours(r.totalActualMinutes) },
      { label: "Differenz", value: this.fmtHours(Math.abs(r.totalActualMinutes - r.totalPlannedMinutes)), cls: r.totalActualMinutes > r.totalPlannedMinutes ? "pl-tc-over" : "pl-tc-under" },
    ]);
    cards.appendChild(timeCard);

    // Cost card
    const costCard = this.createReportCard("Kosten", [
      { label: "Material", value: `${r.materialCost.toFixed(2)} EUR` },
      { label: "Arbeit", value: `${r.laborCost.toFixed(2)} EUR` },
      { label: "Gesamt", value: `${r.totalCost.toFixed(2)} EUR` },
    ]);
    cards.appendChild(costCard);

    // Quality card
    const passRate = r.inspectionCount > 0 ? Math.round((r.passCount / r.inspectionCount) * 100) : 0;
    const qualityCard = this.createReportCard("Qualitaet", [
      { label: "Pruefungen", value: String(r.inspectionCount) },
      { label: "Bestanden", value: `${r.passCount} (${passRate}%)` },
      { label: "Fehlgeschlagen", value: String(r.failCount), cls: r.failCount > 0 ? "pl-tc-over" : "" },
      { label: "Offene Fehler", value: String(r.openDefects), cls: r.openDefects > 0 ? "pl-tc-over" : "" },
    ]);
    cards.appendChild(qualityCard);

    // Workflow card
    const wfPct = r.workflowTotal > 0 ? Math.round((r.workflowCompleted / r.workflowTotal) * 100) : 0;
    const wfCard = this.createReportCard("Workflow", [
      { label: "Schritte gesamt", value: String(r.workflowTotal) },
      { label: "Abgeschlossen", value: String(r.workflowCompleted) },
      { label: "Fortschritt", value: `${wfPct}%` },
    ]);
    cards.appendChild(wfCard);

    container.appendChild(cards);

    // CSV export button
    const exportRow = document.createElement("div");
    exportRow.className = "mfg-actions";
    const exportBtn = document.createElement("button");
    exportBtn.className = "dialog-btn dialog-btn-primary";
    exportBtn.textContent = "CSV Export";
    exportBtn.addEventListener("click", async () => {
      if (!this.reportProjectId) return;
      try {
        const csv = await ReportService.exportProjectCsv(this.reportProjectId);
        const blob = new Blob([csv], { type: "text/csv" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = `projekt_${this.reportProjectId}_bericht.csv`;
        a.click();
        URL.revokeObjectURL(url);
        ToastContainer.show("success", "CSV exportiert");
      } catch { ToastContainer.show("error", "Export fehlgeschlagen"); }
    });
    exportRow.appendChild(exportBtn);
    container.appendChild(exportRow);
  }

  private createReportCard(title: string, rows: { label: string; value: string; cls?: string }[]): HTMLElement {
    const card = document.createElement("div");
    card.className = "mfg-report-card";
    const h = document.createElement("h4");
    h.className = "mfg-report-card-title";
    h.textContent = title;
    card.appendChild(h);
    for (const row of rows) {
      const r = document.createElement("div");
      r.className = "mfg-report-row";
      const lbl = document.createElement("span");
      lbl.className = "mfg-report-label";
      lbl.textContent = row.label;
      const val = document.createElement("span");
      val.className = "mfg-report-value" + (row.cls ? ` ${row.cls}` : "");
      val.textContent = row.value;
      r.appendChild(lbl);
      r.appendChild(val);
      card.appendChild(r);
    }
    return card;
  }

  // ── Helpers ──────────────────────────────────────────────────────

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
