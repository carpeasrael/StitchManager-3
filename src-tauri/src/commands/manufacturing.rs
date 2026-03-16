use serde::Deserialize;
use tauri::State;

use crate::db::models::{BillOfMaterial, Material, MaterialInventory, Product, Supplier, TimeEntry};
use crate::error::{lock_db, AppError};
use crate::DbState;

// ── Suppliers ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierCreate {
    pub name: String,
    pub contact: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierUpdate {
    pub name: Option<String>,
    pub contact: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn create_supplier(
    db: State<'_, DbState>,
    supplier: SupplierCreate,
) -> Result<Supplier, AppError> {
    let name = supplier.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("Lieferantenname darf nicht leer sein".into()));
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO suppliers (name, contact, website, notes) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![name, supplier.contact, supplier.website, supplier.notes],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, contact, website, notes, created_at, updated_at FROM suppliers WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        row_to_supplier,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_suppliers(db: State<'_, DbState>) -> Result<Vec<Supplier>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, contact, website, notes, created_at, updated_at FROM suppliers WHERE deleted_at IS NULL ORDER BY name"
    )?;
    let suppliers = stmt
        .query_map([], row_to_supplier)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(suppliers)
}

#[tauri::command]
pub fn get_supplier(db: State<'_, DbState>, supplier_id: i64) -> Result<Supplier, AppError> {
    let conn = lock_db(&db)?;
    conn.query_row(
        "SELECT id, name, contact, website, notes, created_at, updated_at FROM suppliers WHERE id = ?1 AND deleted_at IS NULL",
        [supplier_id],
        row_to_supplier,
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Lieferant {supplier_id} nicht gefunden")),
        _ => AppError::Database(e),
    })
}

#[tauri::command]
pub fn update_supplier(
    db: State<'_, DbState>,
    supplier_id: i64,
    update: SupplierUpdate,
) -> Result<Supplier, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(name) = &update.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("Lieferantenname darf nicht leer sein".into()));
        }
        params.push(Box::new(trimmed.to_string()));
        sets.push(format!("name = ?{}", params.len()));
    }
    if let Some(contact) = &update.contact {
        params.push(Box::new(contact.clone()));
        sets.push(format!("contact = ?{}", params.len()));
    }
    if let Some(website) = &update.website {
        params.push(Box::new(website.clone()));
        sets.push(format!("website = ?{}", params.len()));
    }
    if let Some(notes) = &update.notes {
        params.push(Box::new(notes.clone()));
        sets.push(format!("notes = ?{}", params.len()));
    }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, name, contact, website, notes, created_at, updated_at FROM suppliers WHERE id = ?1 AND deleted_at IS NULL",
            [supplier_id],
            row_to_supplier,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Lieferant {supplier_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    sets.push("updated_at = datetime('now')".to_string());
    params.push(Box::new(supplier_id));
    let sql = format!(
        "UPDATE suppliers SET {} WHERE id = ?{} AND deleted_at IS NULL",
        sets.join(", "),
        params.len()
    );
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Lieferant {supplier_id} nicht gefunden")));
    }
    conn.query_row(
        "SELECT id, name, contact, website, notes, created_at, updated_at FROM suppliers WHERE id = ?1 AND deleted_at IS NULL",
        [supplier_id],
        row_to_supplier,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_supplier(db: State<'_, DbState>, supplier_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE suppliers SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [supplier_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Lieferant {supplier_id} nicht gefunden")));
    }
    Ok(())
}

fn row_to_supplier(row: &rusqlite::Row) -> rusqlite::Result<Supplier> {
    Ok(Supplier {
        id: row.get(0)?,
        name: row.get(1)?,
        contact: row.get(2)?,
        website: row.get(3)?,
        notes: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

// ── Materials ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialCreate {
    pub material_number: Option<String>,
    pub name: String,
    pub material_type: Option<String>,
    pub unit: Option<String>,
    pub supplier_id: Option<i64>,
    pub net_price: Option<f64>,
    pub waste_factor: Option<f64>,
    pub min_stock: Option<f64>,
    pub reorder_time_days: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialUpdate {
    pub material_number: Option<String>,
    pub name: Option<String>,
    pub material_type: Option<String>,
    pub unit: Option<String>,
    pub supplier_id: Option<i64>,
    pub net_price: Option<f64>,
    pub waste_factor: Option<f64>,
    pub min_stock: Option<f64>,
    pub reorder_time_days: Option<i32>,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn create_material(
    db: State<'_, DbState>,
    material: MaterialCreate,
) -> Result<Material, AppError> {
    let name = material.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("Materialname darf nicht leer sein".into()));
    }
    if let Some(price) = material.net_price {
        if price < 0.0 { return Err(AppError::Validation("Preis darf nicht negativ sein".into())); }
    }
    if let Some(wf) = material.waste_factor {
        if !(0.0..=1.0).contains(&wf) { return Err(AppError::Validation("Verschnittfaktor muss zwischen 0.0 und 1.0 liegen".into())); }
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO materials (material_number, name, material_type, unit, supplier_id, net_price, waste_factor, min_stock, reorder_time_days, notes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            material.material_number, name, material.material_type,
            material.unit.as_deref().unwrap_or("Stk"),
            material.supplier_id, material.net_price,
            material.waste_factor.unwrap_or(0.0),
            material.min_stock.unwrap_or(0.0),
            material.reorder_time_days, material.notes
        ],
    )?;
    let id = conn.last_insert_rowid();

    // Create initial inventory record
    conn.execute(
        "INSERT INTO material_inventory (material_id, total_stock, reserved_stock) VALUES (?1, 0, 0)",
        [id],
    )?;

    conn.query_row(
        "SELECT id, material_number, name, material_type, unit, supplier_id, net_price, waste_factor, min_stock, reorder_time_days, notes, created_at, updated_at \
         FROM materials WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        row_to_material,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_materials(db: State<'_, DbState>) -> Result<Vec<Material>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, material_number, name, material_type, unit, supplier_id, net_price, waste_factor, min_stock, reorder_time_days, notes, created_at, updated_at \
         FROM materials WHERE deleted_at IS NULL ORDER BY name"
    )?;
    let materials = stmt
        .query_map([], row_to_material)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(materials)
}

#[tauri::command]
pub fn get_material(db: State<'_, DbState>, material_id: i64) -> Result<Material, AppError> {
    let conn = lock_db(&db)?;
    conn.query_row(
        "SELECT id, material_number, name, material_type, unit, supplier_id, net_price, waste_factor, min_stock, reorder_time_days, notes, created_at, updated_at \
         FROM materials WHERE id = ?1 AND deleted_at IS NULL",
        [material_id],
        row_to_material,
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Material {material_id} nicht gefunden")),
        _ => AppError::Database(e),
    })
}

#[tauri::command]
pub fn update_material(
    db: State<'_, DbState>,
    material_id: i64,
    update: MaterialUpdate,
) -> Result<Material, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(name) = &update.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("Materialname darf nicht leer sein".into()));
        }
        params.push(Box::new(trimmed.to_string()));
        sets.push(format!("name = ?{}", params.len()));
    }
    if let Some(v) = &update.material_number { params.push(Box::new(v.clone())); sets.push(format!("material_number = ?{}", params.len())); }
    if let Some(v) = &update.material_type { params.push(Box::new(v.clone())); sets.push(format!("material_type = ?{}", params.len())); }
    if let Some(v) = &update.unit { params.push(Box::new(v.clone())); sets.push(format!("unit = ?{}", params.len())); }
    if let Some(v) = update.supplier_id { params.push(Box::new(v)); sets.push(format!("supplier_id = ?{}", params.len())); }
    if let Some(v) = update.net_price {
        if v < 0.0 { return Err(AppError::Validation("Preis darf nicht negativ sein".into())); }
        params.push(Box::new(v)); sets.push(format!("net_price = ?{}", params.len()));
    }
    if let Some(v) = update.waste_factor {
        if !(0.0..=1.0).contains(&v) { return Err(AppError::Validation("Verschnittfaktor muss zwischen 0.0 und 1.0 liegen".into())); }
        params.push(Box::new(v)); sets.push(format!("waste_factor = ?{}", params.len()));
    }
    if let Some(v) = update.min_stock { params.push(Box::new(v)); sets.push(format!("min_stock = ?{}", params.len())); }
    if let Some(v) = update.reorder_time_days { params.push(Box::new(v)); sets.push(format!("reorder_time_days = ?{}", params.len())); }
    if let Some(v) = &update.notes { params.push(Box::new(v.clone())); sets.push(format!("notes = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, material_number, name, material_type, unit, supplier_id, net_price, waste_factor, min_stock, reorder_time_days, notes, created_at, updated_at \
             FROM materials WHERE id = ?1 AND deleted_at IS NULL",
            [material_id],
            row_to_material,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Material {material_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    sets.push("updated_at = datetime('now')".to_string());
    params.push(Box::new(material_id));
    let sql = format!(
        "UPDATE materials SET {} WHERE id = ?{} AND deleted_at IS NULL",
        sets.join(", "),
        params.len()
    );
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Material {material_id} nicht gefunden")));
    }
    conn.query_row(
        "SELECT id, material_number, name, material_type, unit, supplier_id, net_price, waste_factor, min_stock, reorder_time_days, notes, created_at, updated_at \
         FROM materials WHERE id = ?1 AND deleted_at IS NULL",
        [material_id],
        row_to_material,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_material(db: State<'_, DbState>, material_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE materials SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [material_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Material {material_id} nicht gefunden")));
    }
    Ok(())
}

fn row_to_material(row: &rusqlite::Row) -> rusqlite::Result<Material> {
    Ok(Material {
        id: row.get(0)?,
        material_number: row.get(1)?,
        name: row.get(2)?,
        material_type: row.get(3)?,
        unit: row.get(4)?,
        supplier_id: row.get(5)?,
        net_price: row.get(6)?,
        waste_factor: row.get(7)?,
        min_stock: row.get(8)?,
        reorder_time_days: row.get(9)?,
        notes: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

// ── Inventory ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_inventory(db: State<'_, DbState>, material_id: i64) -> Result<MaterialInventory, AppError> {
    let conn = lock_db(&db)?;
    conn.query_row(
        "SELECT id, material_id, total_stock, reserved_stock, location, updated_at \
         FROM material_inventory WHERE material_id = ?1",
        [material_id],
        row_to_inventory,
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Inventar fuer Material {material_id} nicht gefunden")),
        _ => AppError::Database(e),
    })
}

#[tauri::command]
pub fn update_inventory(
    db: State<'_, DbState>,
    material_id: i64,
    total_stock: Option<f64>,
    reserved_stock: Option<f64>,
    location: Option<String>,
) -> Result<MaterialInventory, AppError> {
    if let Some(v) = total_stock {
        if v < 0.0 { return Err(AppError::Validation("Gesamtbestand darf nicht negativ sein".into())); }
    }
    if let Some(v) = reserved_stock {
        if v < 0.0 { return Err(AppError::Validation("Reservierter Bestand darf nicht negativ sein".into())); }
    }

    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = vec!["updated_at = datetime('now')".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(v) = total_stock { params.push(Box::new(v)); sets.push(format!("total_stock = ?{}", params.len())); }
    if let Some(v) = reserved_stock { params.push(Box::new(v)); sets.push(format!("reserved_stock = ?{}", params.len())); }
    if let Some(v) = &location { params.push(Box::new(v.clone())); sets.push(format!("location = ?{}", params.len())); }

    params.push(Box::new(material_id));
    let sql = format!(
        "UPDATE material_inventory SET {} WHERE material_id = ?{}",
        sets.join(", "),
        params.len()
    );
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Inventar fuer Material {material_id} nicht gefunden")));
    }
    conn.query_row(
        "SELECT id, material_id, total_stock, reserved_stock, location, updated_at FROM material_inventory WHERE material_id = ?1",
        [material_id],
        row_to_inventory,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_low_stock_materials(db: State<'_, DbState>) -> Result<Vec<Material>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT m.id, m.material_number, m.name, m.material_type, m.unit, m.supplier_id, m.net_price, m.waste_factor, m.min_stock, m.reorder_time_days, m.notes, m.created_at, m.updated_at \
         FROM materials m \
         JOIN material_inventory i ON i.material_id = m.id \
         WHERE m.deleted_at IS NULL AND m.min_stock > 0 AND (i.total_stock - i.reserved_stock) < m.min_stock \
         ORDER BY m.name"
    )?;
    let materials = stmt
        .query_map([], row_to_material)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(materials)
}

fn row_to_inventory(row: &rusqlite::Row) -> rusqlite::Result<MaterialInventory> {
    Ok(MaterialInventory {
        id: row.get(0)?,
        material_id: row.get(1)?,
        total_stock: row.get(2)?,
        reserved_stock: row.get(3)?,
        location: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

// ── Products ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductCreate {
    pub product_number: Option<String>,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub product_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductUpdate {
    pub product_number: Option<String>,
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub product_type: Option<String>,
    pub status: Option<String>,
}

#[tauri::command]
pub fn create_product(
    db: State<'_, DbState>,
    product: ProductCreate,
) -> Result<Product, AppError> {
    let name = product.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("Produktname darf nicht leer sein".into()));
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO products (product_number, name, category, description, product_type) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![product.product_number, name, product.category, product.description, product.product_type],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, product_number, name, category, description, product_type, status, created_at, updated_at \
         FROM products WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        row_to_product,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_products(db: State<'_, DbState>) -> Result<Vec<Product>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, product_number, name, category, description, product_type, status, created_at, updated_at \
         FROM products WHERE deleted_at IS NULL ORDER BY name"
    )?;
    let products = stmt
        .query_map([], row_to_product)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(products)
}

#[tauri::command]
pub fn get_product(db: State<'_, DbState>, product_id: i64) -> Result<Product, AppError> {
    let conn = lock_db(&db)?;
    conn.query_row(
        "SELECT id, product_number, name, category, description, product_type, status, created_at, updated_at \
         FROM products WHERE id = ?1 AND deleted_at IS NULL",
        [product_id],
        row_to_product,
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Produkt {product_id} nicht gefunden")),
        _ => AppError::Database(e),
    })
}

#[tauri::command]
pub fn update_product(
    db: State<'_, DbState>,
    product_id: i64,
    update: ProductUpdate,
) -> Result<Product, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(name) = &update.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("Produktname darf nicht leer sein".into()));
        }
        params.push(Box::new(trimmed.to_string()));
        sets.push(format!("name = ?{}", params.len()));
    }
    if let Some(v) = &update.product_number { params.push(Box::new(v.clone())); sets.push(format!("product_number = ?{}", params.len())); }
    if let Some(v) = &update.category { params.push(Box::new(v.clone())); sets.push(format!("category = ?{}", params.len())); }
    if let Some(v) = &update.description { params.push(Box::new(v.clone())); sets.push(format!("description = ?{}", params.len())); }
    if let Some(v) = &update.product_type { params.push(Box::new(v.clone())); sets.push(format!("product_type = ?{}", params.len())); }
    if let Some(v) = &update.status { params.push(Box::new(v.clone())); sets.push(format!("status = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, product_number, name, category, description, product_type, status, created_at, updated_at \
             FROM products WHERE id = ?1 AND deleted_at IS NULL",
            [product_id],
            row_to_product,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Produkt {product_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    sets.push("updated_at = datetime('now')".to_string());
    params.push(Box::new(product_id));
    let sql = format!(
        "UPDATE products SET {} WHERE id = ?{} AND deleted_at IS NULL",
        sets.join(", "),
        params.len()
    );
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Produkt {product_id} nicht gefunden")));
    }
    conn.query_row(
        "SELECT id, product_number, name, category, description, product_type, status, created_at, updated_at \
         FROM products WHERE id = ?1 AND deleted_at IS NULL",
        [product_id],
        row_to_product,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_product(db: State<'_, DbState>, product_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE products SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [product_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Produkt {product_id} nicht gefunden")));
    }
    Ok(())
}

fn row_to_product(row: &rusqlite::Row) -> rusqlite::Result<Product> {
    Ok(Product {
        id: row.get(0)?,
        product_number: row.get(1)?,
        name: row.get(2)?,
        category: row.get(3)?,
        description: row.get(4)?,
        product_type: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

// ── Bill of Materials ──────────────────────────────────────────────────────

#[tauri::command]
pub fn add_bom_entry(
    db: State<'_, DbState>,
    product_id: i64,
    material_id: i64,
    quantity: f64,
    unit: Option<String>,
    notes: Option<String>,
) -> Result<BillOfMaterial, AppError> {
    if quantity <= 0.0 {
        return Err(AppError::Validation("Menge muss groesser als 0 sein".into()));
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO bill_of_materials (product_id, material_id, quantity, unit, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![product_id, material_id, quantity, unit, notes],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, product_id, material_id, quantity, unit, notes FROM bill_of_materials WHERE id = ?1",
        [id],
        row_to_bom,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_bom_entries(db: State<'_, DbState>, product_id: i64) -> Result<Vec<BillOfMaterial>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, product_id, material_id, quantity, unit, notes FROM bill_of_materials WHERE product_id = ?1"
    )?;
    let entries = stmt
        .query_map([product_id], row_to_bom)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

#[tauri::command]
pub fn update_bom_entry(
    db: State<'_, DbState>,
    bom_id: i64,
    quantity: Option<f64>,
    unit: Option<String>,
    notes: Option<String>,
) -> Result<BillOfMaterial, AppError> {
    if let Some(q) = quantity {
        if q <= 0.0 {
            return Err(AppError::Validation("Menge muss groesser als 0 sein".into()));
        }
    }
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(v) = quantity { params.push(Box::new(v)); sets.push(format!("quantity = ?{}", params.len())); }
    if let Some(v) = &unit { params.push(Box::new(v.clone())); sets.push(format!("unit = ?{}", params.len())); }
    if let Some(v) = &notes { params.push(Box::new(v.clone())); sets.push(format!("notes = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, product_id, material_id, quantity, unit, notes FROM bill_of_materials WHERE id = ?1",
            [bom_id],
            row_to_bom,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("BOM-Eintrag {bom_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    params.push(Box::new(bom_id));
    let sql = format!(
        "UPDATE bill_of_materials SET {} WHERE id = ?{}",
        sets.join(", "),
        params.len()
    );
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("BOM-Eintrag {bom_id} nicht gefunden")));
    }
    conn.query_row(
        "SELECT id, product_id, material_id, quantity, unit, notes FROM bill_of_materials WHERE id = ?1",
        [bom_id],
        row_to_bom,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_bom_entry(db: State<'_, DbState>, bom_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM bill_of_materials WHERE id = ?1", [bom_id])?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("BOM-Eintrag {bom_id} nicht gefunden")));
    }
    Ok(())
}

fn row_to_bom(row: &rusqlite::Row) -> rusqlite::Result<BillOfMaterial> {
    Ok(BillOfMaterial {
        id: row.get(0)?,
        product_id: row.get(1)?,
        material_id: row.get(2)?,
        quantity: row.get(3)?,
        unit: row.get(4)?,
        notes: row.get(5)?,
    })
}

// ── Time Entries ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntryCreate {
    pub project_id: i64,
    pub step_name: String,
    pub planned_minutes: Option<f64>,
    pub actual_minutes: Option<f64>,
    pub worker: Option<String>,
    pub machine: Option<String>,
}

#[tauri::command]
pub fn create_time_entry(
    db: State<'_, DbState>,
    entry: TimeEntryCreate,
) -> Result<TimeEntry, AppError> {
    let step = entry.step_name.trim().to_string();
    if step.is_empty() {
        return Err(AppError::Validation("Arbeitsschritt darf nicht leer sein".into()));
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO time_entries (project_id, step_name, planned_minutes, actual_minutes, worker, machine) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![entry.project_id, step, entry.planned_minutes, entry.actual_minutes, entry.worker, entry.machine],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, recorded_at \
         FROM time_entries WHERE id = ?1",
        [id],
        row_to_time_entry,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_time_entries(db: State<'_, DbState>, project_id: i64) -> Result<Vec<TimeEntry>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, recorded_at \
         FROM time_entries WHERE project_id = ?1 ORDER BY recorded_at DESC"
    )?;
    let entries = stmt
        .query_map([project_id], row_to_time_entry)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

#[tauri::command]
pub fn update_time_entry(
    db: State<'_, DbState>,
    entry_id: i64,
    step_name: Option<String>,
    planned_minutes: Option<f64>,
    actual_minutes: Option<f64>,
    worker: Option<String>,
    machine: Option<String>,
) -> Result<TimeEntry, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(s) = &step_name {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation("Arbeitsschritt darf nicht leer sein".into()));
        }
        params.push(Box::new(trimmed.to_string()));
        sets.push(format!("step_name = ?{}", params.len()));
    }
    if let Some(v) = planned_minutes { params.push(Box::new(v)); sets.push(format!("planned_minutes = ?{}", params.len())); }
    if let Some(v) = actual_minutes { params.push(Box::new(v)); sets.push(format!("actual_minutes = ?{}", params.len())); }
    if let Some(v) = &worker { params.push(Box::new(v.clone())); sets.push(format!("worker = ?{}", params.len())); }
    if let Some(v) = &machine { params.push(Box::new(v.clone())); sets.push(format!("machine = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, recorded_at FROM time_entries WHERE id = ?1",
            [entry_id],
            row_to_time_entry,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Zeiteintrag {entry_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    params.push(Box::new(entry_id));
    let sql = format!(
        "UPDATE time_entries SET {} WHERE id = ?{}",
        sets.join(", "),
        params.len()
    );
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Zeiteintrag {entry_id} nicht gefunden")));
    }
    conn.query_row(
        "SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, recorded_at FROM time_entries WHERE id = ?1",
        [entry_id],
        row_to_time_entry,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_time_entry(db: State<'_, DbState>, entry_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM time_entries WHERE id = ?1", [entry_id])?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Zeiteintrag {entry_id} nicht gefunden")));
    }
    Ok(())
}

fn row_to_time_entry(row: &rusqlite::Row) -> rusqlite::Result<TimeEntry> {
    Ok(TimeEntry {
        id: row.get(0)?,
        project_id: row.get(1)?,
        step_name: row.get(2)?,
        planned_minutes: row.get(3)?,
        actual_minutes: row.get(4)?,
        worker: row.get(5)?,
        machine: row.get(6)?,
        recorded_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_supplier_crud() {
        let conn = init_database_in_memory().unwrap();
        conn.execute(
            "INSERT INTO suppliers (name, contact) VALUES ('Madeira', 'info@madeira.de')",
            [],
        ).unwrap();
        let id = conn.last_insert_rowid();

        let name: String = conn.query_row(
            "SELECT name FROM suppliers WHERE id = ?1 AND deleted_at IS NULL", [id], |row| row.get(0),
        ).unwrap();
        assert_eq!(name, "Madeira");

        // Soft delete
        conn.execute("UPDATE suppliers SET deleted_at = datetime('now') WHERE id = ?1", [id]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM suppliers WHERE deleted_at IS NULL", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_material_with_inventory() {
        let conn = init_database_in_memory().unwrap();
        conn.execute(
            "INSERT INTO materials (name, material_type, unit) VALUES ('Stickgarn Rot', 'embroidery_thread', 'm')",
            [],
        ).unwrap();
        let mid = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO material_inventory (material_id, total_stock, reserved_stock) VALUES (?1, 500.0, 50.0)",
            [mid],
        ).unwrap();

        let stock: f64 = conn.query_row(
            "SELECT total_stock FROM material_inventory WHERE material_id = ?1", [mid], |row| row.get(0),
        ).unwrap();
        assert_eq!(stock, 500.0);
    }

    #[test]
    fn test_product_bom() {
        let conn = init_database_in_memory().unwrap();
        conn.execute(
            "INSERT INTO products (name, product_type) VALUES ('Besticktes T-Shirt', 'kombiprodukt')",
            [],
        ).unwrap();
        let pid = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO materials (name, unit) VALUES ('Baumwollstoff', 'm')",
            [],
        ).unwrap();
        let mid = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO bill_of_materials (product_id, material_id, quantity, unit) VALUES (?1, ?2, 1.5, 'm')",
            rusqlite::params![pid, mid],
        ).unwrap();

        let qty: f64 = conn.query_row(
            "SELECT quantity FROM bill_of_materials WHERE product_id = ?1", [pid], |row| row.get(0),
        ).unwrap();
        assert_eq!(qty, 1.5);

        // Cascade delete
        conn.execute("DELETE FROM products WHERE id = ?1", [pid]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM bill_of_materials", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_time_entries() {
        let conn = init_database_in_memory().unwrap();
        conn.execute(
            "INSERT INTO projects (name, status) VALUES ('TestProjekt', 'in_progress')",
            [],
        ).unwrap();
        let pid = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO time_entries (project_id, step_name, planned_minutes, actual_minutes, worker) \
             VALUES (?1, 'Sticken', 60.0, 75.0, 'Anna')",
            [pid],
        ).unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM time_entries WHERE project_id = ?1", [pid], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);

        // Cascade delete
        conn.execute("DELETE FROM projects WHERE id = ?1", [pid]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM time_entries", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_project_extended_fields() {
        let conn = init_database_in_memory().unwrap();
        conn.execute(
            "INSERT INTO projects (name, status, order_number, customer, priority, deadline, responsible_person, approval_status) \
             VALUES ('Hochzeitskleid', 'in_progress', 'ORD-2026-001', 'Maria Mueller', 'high', '2026-05-01', 'Anna', 'approved')",
            [],
        ).unwrap();
        let pid = conn.last_insert_rowid();

        let order: String = conn.query_row(
            "SELECT order_number FROM projects WHERE id = ?1", [pid], |row| row.get(0),
        ).unwrap();
        assert_eq!(order, "ORD-2026-001");

        let priority: String = conn.query_row(
            "SELECT priority FROM projects WHERE id = ?1", [pid], |row| row.get(0),
        ).unwrap();
        assert_eq!(priority, "high");
    }
}
