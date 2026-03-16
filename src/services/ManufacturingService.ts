import { invoke } from "@tauri-apps/api/core";
import type {
  Supplier,
  Material,
  MaterialInventory,
  Product,
  BillOfMaterial,
  TimeEntry,
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
