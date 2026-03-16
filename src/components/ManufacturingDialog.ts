import * as MfgService from "../services/ManufacturingService";
import { ToastContainer } from "./Toast";
import * as ProjectService from "../services/ProjectService";
import type {
  Supplier,
  Material,
  MaterialInventory,
  Product,
  BillOfMaterial,
  Project,
  TimeEntry,
} from "../types";

type TabKey = "materials" | "suppliers" | "products" | "inventory" | "timetracking";

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
    [this.materials, this.suppliers, this.products, this.allProjects] =
      await Promise.all([
        MfgService.getMaterials(),
        MfgService.getSuppliers(),
        MfgService.getProducts(),
        ProjectService.getProjects(),
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
