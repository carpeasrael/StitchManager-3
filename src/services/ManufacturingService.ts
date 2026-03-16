import { invoke } from "@tauri-apps/api/core";
import type {
  Supplier,
  Material,
  MaterialInventory,
  Product,
  BillOfMaterial,
  TimeEntry,
  StepDefinition,
  ProductStep,
  WorkflowStep,
  LicenseRecord,
  QualityInspection,
  DefectRecord,
} from "../types/index";

// ── Suppliers ──────────────────────────────────────────────────────────────

export async function createSupplier(supplier: {
  name: string;
  contact?: string;
  website?: string;
  notes?: string;
}): Promise<Supplier> {
  return invoke("create_supplier", { supplier });
}

export async function getSuppliers(): Promise<Supplier[]> {
  return invoke("get_suppliers");
}

export async function getSupplier(supplierId: number): Promise<Supplier> {
  return invoke("get_supplier", { supplierId });
}

export async function updateSupplier(
  supplierId: number,
  update: { name?: string; contact?: string; website?: string; notes?: string }
): Promise<Supplier> {
  return invoke("update_supplier", { supplierId, update });
}

export async function deleteSupplier(supplierId: number): Promise<void> {
  return invoke("delete_supplier", { supplierId });
}

// ── Materials ──────────────────────────────────────────────────────────────

export async function createMaterial(material: {
  name: string;
  materialNumber?: string;
  materialType?: string;
  unit?: string;
  supplierId?: number;
  netPrice?: number;
  wasteFactor?: number;
  minStock?: number;
  reorderTimeDays?: number;
  notes?: string;
}): Promise<Material> {
  return invoke("create_material", { material });
}

export async function getMaterials(): Promise<Material[]> {
  return invoke("get_materials");
}

export async function getMaterial(materialId: number): Promise<Material> {
  return invoke("get_material", { materialId });
}

export async function updateMaterial(
  materialId: number,
  update: {
    name?: string;
    materialNumber?: string;
    materialType?: string;
    unit?: string;
    supplierId?: number;
    netPrice?: number;
    wasteFactor?: number;
    minStock?: number;
    reorderTimeDays?: number;
    notes?: string;
  }
): Promise<Material> {
  return invoke("update_material", { materialId, update });
}

export async function deleteMaterial(materialId: number): Promise<void> {
  return invoke("delete_material", { materialId });
}

// ── Inventory ──────────────────────────────────────────────────────────────

export async function getInventory(materialId: number): Promise<MaterialInventory> {
  return invoke("get_inventory", { materialId });
}

export async function updateInventory(
  materialId: number,
  totalStock?: number,
  reservedStock?: number,
  location?: string
): Promise<MaterialInventory> {
  return invoke("update_inventory", { materialId, totalStock, reservedStock, location });
}

export async function getLowStockMaterials(): Promise<Material[]> {
  return invoke("get_low_stock_materials");
}

// ── Products ───────────────────────────────────────────────────────────────

export async function createProduct(product: {
  name: string;
  productNumber?: string;
  category?: string;
  description?: string;
  productType?: string;
}): Promise<Product> {
  return invoke("create_product", { product });
}

export async function getProducts(): Promise<Product[]> {
  return invoke("get_products");
}

export async function getProduct(productId: number): Promise<Product> {
  return invoke("get_product", { productId });
}

export async function updateProduct(
  productId: number,
  update: {
    name?: string;
    productNumber?: string;
    category?: string;
    description?: string;
    productType?: string;
    status?: string;
  }
): Promise<Product> {
  return invoke("update_product", { productId, update });
}

export async function deleteProduct(productId: number): Promise<void> {
  return invoke("delete_product", { productId });
}

// ── Bill of Materials ──────────────────────────────────────────────────────

export async function addBomEntry(
  productId: number,
  materialId: number,
  quantity: number,
  unit?: string,
  notes?: string
): Promise<BillOfMaterial> {
  return invoke("add_bom_entry", { productId, materialId, quantity, unit, notes });
}

export async function getBomEntries(productId: number): Promise<BillOfMaterial[]> {
  return invoke("get_bom_entries", { productId });
}

export async function updateBomEntry(
  bomId: number,
  quantity?: number,
  unit?: string,
  notes?: string
): Promise<BillOfMaterial> {
  return invoke("update_bom_entry", { bomId, quantity, unit, notes });
}

export async function deleteBomEntry(bomId: number): Promise<void> {
  return invoke("delete_bom_entry", { bomId });
}

// ── Time Entries ───────────────────────────────────────────────────────────

export async function createTimeEntry(entry: {
  projectId: number;
  stepName: string;
  plannedMinutes?: number;
  actualMinutes?: number;
  worker?: string;
  machine?: string;
}): Promise<TimeEntry> {
  return invoke("create_time_entry", { entry });
}

export async function getTimeEntries(projectId: number): Promise<TimeEntry[]> {
  return invoke("get_time_entries", { projectId });
}

export async function updateTimeEntry(
  entryId: number,
  stepName?: string,
  plannedMinutes?: number,
  actualMinutes?: number,
  worker?: string,
  machine?: string
): Promise<TimeEntry> {
  return invoke("update_time_entry", { entryId, stepName, plannedMinutes, actualMinutes, worker, machine });
}

export async function deleteTimeEntry(entryId: number): Promise<void> {
  return invoke("delete_time_entry", { entryId });
}

// ── Step Definitions ───────────────────────────────────────────────

export async function createStepDef(step: {
  name: string;
  description?: string;
  defaultDurationMinutes?: number;
  sortOrder?: number;
}): Promise<StepDefinition> {
  return invoke("create_step_def", { step });
}

export async function getStepDefs(): Promise<StepDefinition[]> {
  return invoke("get_step_defs");
}

export async function updateStepDef(
  stepId: number,
  name?: string,
  description?: string,
  defaultDurationMinutes?: number,
  sortOrder?: number
): Promise<StepDefinition> {
  return invoke("update_step_def", { stepId, name, description, defaultDurationMinutes, sortOrder });
}

export async function deleteStepDef(stepId: number): Promise<void> {
  return invoke("delete_step_def", { stepId });
}

// ── Product Steps ──────────────────────────────────────────────────

export async function setProductSteps(productId: number, stepDefIds: number[]): Promise<ProductStep[]> {
  return invoke("set_product_steps", { productId, stepDefIds });
}

export async function getProductSteps(productId: number): Promise<ProductStep[]> {
  return invoke("get_product_steps", { productId });
}

// ── Workflow Steps ─────────────────────────────────────────────────

export async function createWorkflowStepsFromProduct(projectId: number, productId: number): Promise<WorkflowStep[]> {
  return invoke("create_workflow_steps_from_product", { projectId, productId });
}

export async function getWorkflowSteps(projectId: number): Promise<WorkflowStep[]> {
  return invoke("get_workflow_steps", { projectId });
}

export async function updateWorkflowStep(
  stepId: number,
  status?: string,
  responsible?: string,
  notes?: string
): Promise<WorkflowStep> {
  return invoke("update_workflow_step", { stepId, status, responsible, notes });
}

export async function deleteWorkflowStep(stepId: number): Promise<void> {
  return invoke("delete_workflow_step", { stepId });
}

// ── License Management ─────────────────────────────────────────────

export async function createLicense(license: {
  name: string;
  licenseType?: string;
  validFrom?: string;
  validUntil?: string;
  maxUses?: number;
  commercialAllowed?: boolean;
  source?: string;
  notes?: string;
}): Promise<LicenseRecord> {
  return invoke("create_license", { license });
}

export async function getLicenses(): Promise<LicenseRecord[]> {
  return invoke("get_licenses");
}

export async function getLicense(licenseId: number): Promise<LicenseRecord> {
  return invoke("get_license", { licenseId });
}

export async function updateLicense(
  licenseId: number,
  name?: string,
  licenseType?: string,
  validFrom?: string,
  validUntil?: string,
  maxUses?: number,
  commercialAllowed?: boolean,
  source?: string,
  notes?: string
): Promise<LicenseRecord> {
  return invoke("update_license", { licenseId, name, licenseType, validFrom, validUntil, maxUses, commercialAllowed, source, notes });
}

export async function deleteLicense(licenseId: number): Promise<void> {
  return invoke("delete_license", { licenseId });
}

export async function linkLicenseToFile(licenseId: number, fileId: number): Promise<void> {
  return invoke("link_license_to_file", { licenseId, fileId });
}

export async function unlinkLicenseFromFile(licenseId: number, fileId: number): Promise<void> {
  return invoke("unlink_license_from_file", { licenseId, fileId });
}

export async function getFileLicenses(fileId: number): Promise<LicenseRecord[]> {
  return invoke("get_file_licenses", { fileId });
}

export async function getExpiringLicenses(daysAhead?: number): Promise<LicenseRecord[]> {
  return invoke("get_expiring_licenses", { daysAhead });
}

// ── Quality Inspections ────────────────────────────────────────────

export async function createInspection(
  projectId: number,
  workflowStepId?: number,
  inspector?: string,
  result?: string,
  notes?: string
): Promise<QualityInspection> {
  return invoke("create_inspection", { projectId, workflowStepId, inspector, result, notes });
}

export async function getInspections(projectId: number): Promise<QualityInspection[]> {
  return invoke("get_inspections", { projectId });
}

export async function updateInspection(
  inspectionId: number,
  result?: string,
  inspector?: string,
  notes?: string
): Promise<QualityInspection> {
  return invoke("update_inspection", { inspectionId, result, inspector, notes });
}

export async function deleteInspection(inspectionId: number): Promise<void> {
  return invoke("delete_inspection", { inspectionId });
}

export async function createDefect(
  inspectionId: number,
  description: string,
  severity?: string,
  notes?: string
): Promise<DefectRecord> {
  return invoke("create_defect", { inspectionId, description, severity, notes });
}

export async function getDefects(inspectionId: number): Promise<DefectRecord[]> {
  return invoke("get_defects", { inspectionId });
}

export async function updateDefect(
  defectId: number,
  description?: string,
  severity?: string,
  status?: string,
  notes?: string
): Promise<DefectRecord> {
  return invoke("update_defect", { defectId, description, severity, status, notes });
}

export async function deleteDefect(defectId: number): Promise<void> {
  return invoke("delete_defect", { defectId });
}
