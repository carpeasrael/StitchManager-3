use serde::Deserialize;
use tauri::State;

use crate::db::models::{BillOfMaterial, Material, MaterialConsumption, MaterialInventory, NachkalkulationLine, Product, ProductVariant, Supplier, TimeEntry};
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

fn row_to_consumption(row: &rusqlite::Row) -> rusqlite::Result<MaterialConsumption> {
    Ok(MaterialConsumption {
        id: row.get(0)?,
        project_id: row.get(1)?,
        material_id: row.get(2)?,
        quantity: row.get(3)?,
        unit: row.get(4)?,
        step_name: row.get(5)?,
        recorded_by: row.get(6)?,
        notes: row.get(7)?,
        recorded_at: row.get(8)?,
    })
}

// ── Inventory Automation ──────────────────────────────────────────────────

/// Reserve materials for a project based on its BOM (called on approval).
/// Computes BOM requirements from products linked via workflow_steps → product_steps.
pub fn reserve_materials_for_project_inner(conn: &rusqlite::Connection, project_id: i64) -> Result<Vec<(i64, f64)>, AppError> {
    conn.execute_batch("SAVEPOINT reserve_materials")?;

    let result = (|| -> Result<Vec<(i64, f64)>, AppError> {
        let quantity: i64 = conn.query_row(
            "SELECT COALESCE(quantity, 1) FROM projects WHERE id = ?1 AND deleted_at IS NULL",
            [project_id],
            |row| row.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
            _ => AppError::Database(e),
        })?;
        let qty = (quantity.max(1)) as f64;

        // Release any existing reservations first (idempotent re-approval)
        release_project_reservations_inner(conn, project_id)?;

        // Get BOM materials for all products linked to this project
        let mut stmt = conn.prepare(
            "SELECT b.material_id, SUM(b.quantity) \
             FROM bill_of_materials b \
             WHERE b.product_id IN ( \
                 SELECT DISTINCT ps.product_id FROM product_steps ps \
                 JOIN workflow_steps ws ON ws.step_definition_id = ps.step_definition_id \
                 WHERE ws.project_id = ?1 \
             ) \
             GROUP BY b.material_id"
        )?;

        let reservations: Vec<(i64, f64)> = stmt.query_map([project_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;

        for (material_id, bom_qty) in &reservations {
            let reserve_qty = bom_qty * qty;

            // Ensure inventory record exists
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM material_inventory WHERE material_id = ?1",
                [material_id], |row| row.get(0),
            )?;
            if !exists {
                conn.execute(
                    "INSERT INTO material_inventory (material_id, total_stock, reserved_stock) VALUES (?1, 0, 0)",
                    [material_id],
                )?;
            }

            // Add to reserved_stock
            conn.execute(
                "UPDATE material_inventory SET reserved_stock = reserved_stock + ?1, updated_at = datetime('now') WHERE material_id = ?2",
                rusqlite::params![reserve_qty, material_id],
            )?;

            // Log transaction
            conn.execute(
                "INSERT INTO inventory_transactions (material_id, project_id, transaction_type, quantity, notes) \
                 VALUES (?1, ?2, 'reserve', ?3, 'Auto-Reservierung bei Projektfreigabe')",
                rusqlite::params![material_id, project_id, reserve_qty],
            )?;
        }

        Ok(reservations)
    })();

    match &result {
        Ok(_) => conn.execute_batch("RELEASE reserve_materials")?,
        Err(_) => conn.execute_batch("ROLLBACK TO reserve_materials")?,
    }
    result
}

#[tauri::command]
pub fn reserve_materials_for_project(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    reserve_materials_for_project_inner(&conn, project_id)?;
    Ok(())
}

/// Release remaining reserved stock for a project (called on completion/archive).
pub fn release_project_reservations_inner(conn: &rusqlite::Connection, project_id: i64) -> Result<(), AppError> {
    conn.execute_batch("SAVEPOINT release_reservations")?;

    let result = (|| -> Result<(), AppError> {
        let mut stmt = conn.prepare(
            "SELECT material_id, SUM(CASE WHEN transaction_type = 'reserve' THEN quantity \
                                          WHEN transaction_type = 'consume' THEN -quantity \
                                          WHEN transaction_type = 'release' THEN -quantity \
                                          WHEN transaction_type = 'reverse' THEN quantity \
                                          ELSE 0 END) as net_reserved \
             FROM inventory_transactions \
             WHERE project_id = ?1 AND transaction_type IN ('reserve', 'consume', 'release', 'reverse') \
             GROUP BY material_id \
             HAVING net_reserved > 0"
        )?;

        let releases: Vec<(i64, f64)> = stmt.query_map([project_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;

        for (material_id, release_qty) in &releases {
            conn.execute(
                "UPDATE material_inventory SET reserved_stock = MAX(0, reserved_stock - ?1), updated_at = datetime('now') WHERE material_id = ?2",
                rusqlite::params![release_qty, material_id],
            )?;
            conn.execute(
                "INSERT INTO inventory_transactions (material_id, project_id, transaction_type, quantity, notes) \
                 VALUES (?1, ?2, 'release', ?3, 'Freigabe bei Projektabschluss')",
                rusqlite::params![material_id, project_id, release_qty],
            )?;
        }
        Ok(())
    })();

    match &result {
        Ok(_) => conn.execute_batch("RELEASE release_reservations")?,
        Err(_) => conn.execute_batch("ROLLBACK TO release_reservations")?,
    }
    result
}

#[tauri::command]
pub fn release_project_reservations(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    release_project_reservations_inner(&conn, project_id)?;
    Ok(())
}

// ── Material Consumption ──────────────────────────────────────────────────

#[tauri::command]
pub fn record_consumption(
    db: State<'_, DbState>,
    project_id: i64,
    material_id: i64,
    quantity: f64,
    unit: Option<String>,
    step_name: Option<String>,
    recorded_by: Option<String>,
    notes: Option<String>,
) -> Result<MaterialConsumption, AppError> {
    if quantity <= 0.0 {
        return Err(AppError::Validation("Verbrauchsmenge muss positiv sein".into()));
    }
    let conn = lock_db(&db)?;

    // Validate project and material exist
    let project_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id], |row| row.get(0),
    )?;
    if !project_exists {
        return Err(AppError::NotFound(format!("Projekt {project_id} nicht gefunden")));
    }
    let material_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM materials WHERE id = ?1 AND deleted_at IS NULL",
        [material_id], |row| row.get(0),
    )?;
    if !material_exists {
        return Err(AppError::NotFound(format!("Material {material_id} nicht gefunden")));
    }

    conn.execute_batch("SAVEPOINT record_consume")?;

    let result = (|| -> Result<i64, AppError> {
        // Insert consumption record
        conn.execute(
            "INSERT INTO material_consumptions (project_id, material_id, quantity, unit, step_name, recorded_by, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![project_id, material_id, quantity, unit, step_name, recorded_by, notes],
        )?;
        let id = conn.last_insert_rowid();

        // Ensure inventory record exists
        let inv_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM material_inventory WHERE material_id = ?1",
            [material_id], |row| row.get(0),
        )?;
        if !inv_exists {
            conn.execute(
                "INSERT INTO material_inventory (material_id, total_stock, reserved_stock) VALUES (?1, 0, 0)",
                [material_id],
            )?;
        }

        // Reduce total_stock
        conn.execute(
            "UPDATE material_inventory SET total_stock = MAX(0, total_stock - ?1), updated_at = datetime('now') WHERE material_id = ?2",
            rusqlite::params![quantity, material_id],
        )?;

        // Reduce reserved_stock (consume from reservation if any exists for this project)
        let net_reserved: f64 = conn.query_row(
            "SELECT COALESCE(SUM(CASE WHEN transaction_type = 'reserve' THEN quantity \
                                       WHEN transaction_type = 'consume' THEN -quantity \
                                       WHEN transaction_type = 'release' THEN -quantity \
                                       WHEN transaction_type = 'reverse' THEN quantity \
                                       ELSE 0 END), 0) \
             FROM inventory_transactions \
             WHERE project_id = ?1 AND material_id = ?2",
            rusqlite::params![project_id, material_id],
            |row| row.get(0),
        )?;
        let reserved_reduction = quantity.min(net_reserved.max(0.0));
        if reserved_reduction > 0.0 {
            conn.execute(
                "UPDATE material_inventory SET reserved_stock = MAX(0, reserved_stock - ?1), updated_at = datetime('now') WHERE material_id = ?2",
                rusqlite::params![reserved_reduction, material_id],
            )?;
        }

        // Log transaction
        conn.execute(
            "INSERT INTO inventory_transactions (material_id, project_id, transaction_type, quantity, notes) \
             VALUES (?1, ?2, 'consume', ?3, ?4)",
            rusqlite::params![material_id, project_id, quantity, notes],
        )?;

        Ok(id)
    })();

    match &result {
        Ok(_) => conn.execute_batch("RELEASE record_consume")?,
        Err(_) => conn.execute_batch("ROLLBACK TO record_consume")?,
    }
    let id = result?;

    conn.query_row(
        "SELECT id, project_id, material_id, quantity, unit, step_name, recorded_by, notes, recorded_at \
         FROM material_consumptions WHERE id = ?1",
        [id],
        row_to_consumption,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_consumptions(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<MaterialConsumption>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, material_id, quantity, unit, step_name, recorded_by, notes, recorded_at \
         FROM material_consumptions WHERE project_id = ?1 ORDER BY recorded_at DESC"
    )?;
    let entries = stmt.query_map([project_id], row_to_consumption)?.collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

#[tauri::command]
pub fn delete_consumption(
    db: State<'_, DbState>,
    consumption_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    // Get consumption details before starting savepoint
    let (project_id, material_id, quantity): (i64, i64, f64) = conn.query_row(
        "SELECT project_id, material_id, quantity FROM material_consumptions WHERE id = ?1",
        [consumption_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Verbrauch {consumption_id} nicht gefunden")),
        _ => AppError::Database(e),
    })?;

    conn.execute_batch("SAVEPOINT delete_consume")?;

    let result = (|| -> Result<(), AppError> {
        // Determine how much of the original consumption was drawn from reserved stock.
        let net_reserved_before: f64 = conn.query_row(
            "SELECT COALESCE(SUM(CASE WHEN transaction_type = 'reserve' THEN quantity \
                                       WHEN transaction_type = 'consume' THEN -quantity \
                                       WHEN transaction_type = 'release' THEN -quantity \
                                       WHEN transaction_type = 'reverse' THEN quantity \
                                       ELSE 0 END), 0) \
             FROM inventory_transactions \
             WHERE project_id = ?1 AND material_id = ?2",
            rusqlite::params![project_id, material_id],
            |row| row.get(0),
        )?;

        let total_reserved: f64 = conn.query_row(
            "SELECT COALESCE(SUM(quantity), 0) FROM inventory_transactions \
             WHERE project_id = ?1 AND material_id = ?2 AND transaction_type = 'reserve'",
            rusqlite::params![project_id, material_id],
            |row| row.get(0),
        )?;

        let reserved_restore = quantity.min((total_reserved - net_reserved_before).max(0.0));

        conn.execute("DELETE FROM material_consumptions WHERE id = ?1", [consumption_id])?;

        conn.execute(
            "UPDATE material_inventory SET total_stock = total_stock + ?1, \
             reserved_stock = reserved_stock + ?2, \
             updated_at = datetime('now') WHERE material_id = ?3",
            rusqlite::params![quantity, reserved_restore, material_id],
        )?;

        conn.execute(
            "INSERT INTO inventory_transactions (material_id, project_id, transaction_type, quantity, notes) \
             VALUES (?1, ?2, 'reverse', ?3, 'Verbrauch storniert')",
            rusqlite::params![material_id, project_id, quantity],
        )?;

        Ok(())
    })();

    match &result {
        Ok(_) => conn.execute_batch("RELEASE delete_consume")?,
        Err(_) => conn.execute_batch("ROLLBACK TO delete_consume")?,
    }
    result
}

// ── Nachkalkulation ───────────────────────────────────────────────────────

#[tauri::command]
pub fn get_nachkalkulation(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<NachkalkulationLine>, AppError> {
    let conn = lock_db(&db)?;

    let quantity: i64 = conn.query_row(
        "SELECT COALESCE(quantity, 1) FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
        _ => AppError::Database(e),
    })?;
    let qty = (quantity.max(1)) as f64;

    // Get planned quantities from BOM and actual from consumptions
    let mut stmt = conn.prepare(
        "SELECT m.id, m.name, m.unit, \
             COALESCE(planned.total_qty, 0) * ?2 as planned_qty, \
             COALESCE(actual.total_qty, 0) as actual_qty, \
             COALESCE(m.net_price, 0) as net_price, \
             COALESCE(m.waste_factor, 0) as waste_factor \
         FROM materials m \
         LEFT JOIN ( \
             SELECT b.material_id, SUM(b.quantity) as total_qty \
             FROM bill_of_materials b \
             WHERE b.product_id IN ( \
                 SELECT DISTINCT ps.product_id FROM product_steps ps \
                 JOIN workflow_steps ws ON ws.step_definition_id = ps.step_definition_id \
                 WHERE ws.project_id = ?1 \
             ) GROUP BY b.material_id \
         ) planned ON planned.material_id = m.id \
         LEFT JOIN ( \
             SELECT material_id, SUM(quantity) as total_qty \
             FROM material_consumptions \
             WHERE project_id = ?1 \
             GROUP BY material_id \
         ) actual ON actual.material_id = m.id \
         WHERE m.deleted_at IS NULL AND (planned.total_qty > 0 OR actual.total_qty > 0) \
         ORDER BY m.name"
    )?;

    let lines = stmt.query_map(rusqlite::params![project_id, qty], |row| {
        let material_id: i64 = row.get(0)?;
        let material_name: String = row.get(1)?;
        let unit: Option<String> = row.get(2)?;
        let planned_quantity: f64 = row.get(3)?;
        let actual_quantity: f64 = row.get(4)?;
        let net_price: f64 = row.get(5)?;
        let waste_factor: f64 = row.get(6)?;
        let difference = actual_quantity - planned_quantity;
        let planned_cost = planned_quantity * net_price * (1.0 + waste_factor);
        let actual_cost = actual_quantity * net_price;
        let cost_difference = actual_cost - planned_cost;

        Ok(NachkalkulationLine {
            material_id,
            material_name,
            unit,
            planned_quantity,
            actual_quantity,
            difference,
            planned_cost,
            actual_cost,
            cost_difference,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(lines)
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

// ── Product Variants ──────────────────────────────────────────────────────

const VARIANT_SELECT: &str =
    "SELECT id, product_id, sku, variant_name, size, color, additional_cost, notes, status, created_at, updated_at FROM product_variants";

fn row_to_variant(row: &rusqlite::Row) -> rusqlite::Result<ProductVariant> {
    Ok(ProductVariant {
        id: row.get(0)?,
        product_id: row.get(1)?,
        sku: row.get(2)?,
        variant_name: row.get(3)?,
        size: row.get(4)?,
        color: row.get(5)?,
        additional_cost: row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
        notes: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantCreate {
    pub sku: Option<String>,
    pub variant_name: Option<String>,
    pub size: Option<String>,
    pub color: Option<String>,
    pub additional_cost: Option<f64>,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn create_variant(
    db: State<'_, DbState>,
    product_id: i64,
    variant: VariantCreate,
) -> Result<ProductVariant, AppError> {
    let conn = lock_db(&db)?;
    let product_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM products WHERE id = ?1 AND deleted_at IS NULL",
        [product_id], |row| row.get(0),
    )?;
    if !product_exists {
        return Err(AppError::NotFound(format!("Produkt {product_id} nicht gefunden")));
    }
    conn.execute(
        "INSERT INTO product_variants (product_id, sku, variant_name, size, color, additional_cost, notes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![product_id, variant.sku, variant.variant_name, variant.size, variant.color,
            variant.additional_cost.unwrap_or(0.0), variant.notes],
    )?;
    let id = conn.last_insert_rowid();
    let sql = format!("{VARIANT_SELECT} WHERE id = ?1");
    conn.query_row(&sql, [id], row_to_variant).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_product_variants(
    db: State<'_, DbState>,
    product_id: i64,
) -> Result<Vec<ProductVariant>, AppError> {
    let conn = lock_db(&db)?;
    let sql = format!("{VARIANT_SELECT} WHERE product_id = ?1 AND deleted_at IS NULL ORDER BY variant_name, size, color");
    let mut stmt = conn.prepare(&sql)?;
    let variants = stmt.query_map([product_id], row_to_variant)?.collect::<Result<Vec<_>, _>>()?;
    Ok(variants)
}

#[tauri::command]
pub fn update_variant(
    db: State<'_, DbState>,
    variant_id: i64,
    sku: Option<String>,
    variant_name: Option<String>,
    size: Option<String>,
    color: Option<String>,
    additional_cost: Option<f64>,
    notes: Option<String>,
    status: Option<String>,
) -> Result<ProductVariant, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(v) = &sku { params.push(Box::new(v.clone())); sets.push(format!("sku = ?{}", params.len())); }
    if let Some(v) = &variant_name { params.push(Box::new(v.clone())); sets.push(format!("variant_name = ?{}", params.len())); }
    if let Some(v) = &size { params.push(Box::new(v.clone())); sets.push(format!("size = ?{}", params.len())); }
    if let Some(v) = &color { params.push(Box::new(v.clone())); sets.push(format!("color = ?{}", params.len())); }
    if let Some(v) = additional_cost { params.push(Box::new(v)); sets.push(format!("additional_cost = ?{}", params.len())); }
    if let Some(v) = &notes { params.push(Box::new(v.clone())); sets.push(format!("notes = ?{}", params.len())); }
    if let Some(v) = &status { params.push(Box::new(v.clone())); sets.push(format!("status = ?{}", params.len())); }

    if sets.is_empty() {
        let sql = format!("{VARIANT_SELECT} WHERE id = ?1 AND deleted_at IS NULL");
        return conn.query_row(&sql, [variant_id], row_to_variant).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Variante {variant_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    sets.push("updated_at = datetime('now')".to_string());
    params.push(Box::new(variant_id));
    let sql = format!("UPDATE product_variants SET {} WHERE id = ?{} AND deleted_at IS NULL", sets.join(", "), params.len());
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 { return Err(AppError::NotFound(format!("Variante {variant_id} nicht gefunden"))); }

    let sql = format!("{VARIANT_SELECT} WHERE id = ?1");
    conn.query_row(&sql, [variant_id], row_to_variant).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_variant(db: State<'_, DbState>, variant_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE product_variants SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [variant_id],
    )?;
    if changes == 0 { return Err(AppError::NotFound(format!("Variante {variant_id} nicht gefunden"))); }
    Ok(())
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
    pub cost_rate_id: Option<i64>,
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
        "INSERT INTO time_entries (project_id, step_name, planned_minutes, actual_minutes, worker, machine, cost_rate_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![entry.project_id, step, entry.planned_minutes, entry.actual_minutes, entry.worker, entry.machine, entry.cost_rate_id],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, cost_rate_id, recorded_at \
         FROM time_entries WHERE id = ?1",
        [id],
        row_to_time_entry,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_time_entries(db: State<'_, DbState>, project_id: i64) -> Result<Vec<TimeEntry>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, cost_rate_id, recorded_at \
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
    cost_rate_id: Option<i64>,
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
    if let Some(v) = cost_rate_id { params.push(Box::new(v)); sets.push(format!("cost_rate_id = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, cost_rate_id, recorded_at FROM time_entries WHERE id = ?1",
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
        "SELECT id, project_id, step_name, planned_minutes, actual_minutes, worker, machine, cost_rate_id, recorded_at FROM time_entries WHERE id = ?1",
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
        cost_rate_id: row.get(7)?,
        recorded_at: row.get(8)?,
    })
}

// ── Step Definitions ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepDefCreate {
    pub name: String,
    pub description: Option<String>,
    pub default_duration_minutes: Option<f64>,
    pub sort_order: Option<i32>,
}

#[tauri::command]
pub fn create_step_def(
    db: State<'_, DbState>,
    step: StepDefCreate,
) -> Result<crate::db::models::StepDefinition, AppError> {
    let name = step.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("Schrittname darf nicht leer sein".into()));
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO step_definitions (name, description, default_duration_minutes, sort_order) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![name, step.description, step.default_duration_minutes, step.sort_order.unwrap_or(0)],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, description, default_duration_minutes, sort_order, created_at FROM step_definitions WHERE id = ?1",
        [id], row_to_step_def,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_step_defs(db: State<'_, DbState>) -> Result<Vec<crate::db::models::StepDefinition>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, description, default_duration_minutes, sort_order, created_at FROM step_definitions ORDER BY sort_order, name"
    )?;
    let defs = stmt.query_map([], row_to_step_def)?.collect::<Result<Vec<_>, _>>()?;
    Ok(defs)
}

#[tauri::command]
pub fn update_step_def(
    db: State<'_, DbState>,
    step_id: i64,
    name: Option<String>,
    description: Option<String>,
    default_duration_minutes: Option<f64>,
    sort_order: Option<i32>,
) -> Result<crate::db::models::StepDefinition, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(n) = &name {
        let trimmed = n.trim();
        if trimmed.is_empty() { return Err(AppError::Validation("Schrittname darf nicht leer sein".into())); }
        params.push(Box::new(trimmed.to_string())); sets.push(format!("name = ?{}", params.len()));
    }
    if let Some(v) = &description { params.push(Box::new(v.clone())); sets.push(format!("description = ?{}", params.len())); }
    if let Some(v) = default_duration_minutes { params.push(Box::new(v)); sets.push(format!("default_duration_minutes = ?{}", params.len())); }
    if let Some(v) = sort_order { params.push(Box::new(v)); sets.push(format!("sort_order = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, name, description, default_duration_minutes, sort_order, created_at FROM step_definitions WHERE id = ?1",
            [step_id], row_to_step_def,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Schritt {step_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    params.push(Box::new(step_id));
    let sql = format!("UPDATE step_definitions SET {} WHERE id = ?{}", sets.join(", "), params.len());
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 { return Err(AppError::NotFound(format!("Schritt {step_id} nicht gefunden"))); }

    conn.query_row(
        "SELECT id, name, description, default_duration_minutes, sort_order, created_at FROM step_definitions WHERE id = ?1",
        [step_id], row_to_step_def,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_step_def(db: State<'_, DbState>, step_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM step_definitions WHERE id = ?1", [step_id])?;
    if changes == 0 { return Err(AppError::NotFound(format!("Schritt {step_id} nicht gefunden"))); }
    Ok(())
}

fn row_to_step_def(row: &rusqlite::Row) -> rusqlite::Result<crate::db::models::StepDefinition> {
    Ok(crate::db::models::StepDefinition {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        default_duration_minutes: row.get(3)?,
        sort_order: row.get(4)?,
        created_at: row.get(5)?,
    })
}

// ── Product Steps ──────────────────────────────────────────────────

#[tauri::command]
pub fn set_product_steps(
    db: State<'_, DbState>,
    product_id: i64,
    step_def_ids: Vec<i64>,
) -> Result<Vec<crate::db::models::ProductStep>, AppError> {
    let conn = lock_db(&db)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM product_steps WHERE product_id = ?1", [product_id])?;
    for (i, sid) in step_def_ids.iter().enumerate() {
        tx.execute(
            "INSERT INTO product_steps (product_id, step_definition_id, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![product_id, sid, i as i32],
        )?;
    }
    tx.commit()?;
    get_product_steps_inner(&conn, product_id)
}

#[tauri::command]
pub fn get_product_steps(
    db: State<'_, DbState>,
    product_id: i64,
) -> Result<Vec<crate::db::models::ProductStep>, AppError> {
    let conn = lock_db(&db)?;
    get_product_steps_inner(&conn, product_id)
}

fn get_product_steps_inner(conn: &rusqlite::Connection, product_id: i64) -> Result<Vec<crate::db::models::ProductStep>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, product_id, step_definition_id, sort_order FROM product_steps WHERE product_id = ?1 ORDER BY sort_order"
    )?;
    let steps = stmt.query_map([product_id], |row| {
        Ok(crate::db::models::ProductStep {
            id: row.get(0)?,
            product_id: row.get(1)?,
            step_definition_id: row.get(2)?,
            sort_order: row.get(3)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(steps)
}

// ── Workflow Steps ─────────────────────────────────────────────────

const VALID_WF_STATUSES: &[&str] = &["pending", "in_progress", "completed", "skipped"];

#[tauri::command]
pub fn create_workflow_steps_from_product(
    db: State<'_, DbState>,
    project_id: i64,
    product_id: i64,
) -> Result<Vec<crate::db::models::WorkflowStep>, AppError> {
    let conn = lock_db(&db)?;
    let product_steps = get_product_steps_inner(&conn, product_id)?;
    let tx = conn.unchecked_transaction()?;
    for ps in &product_steps {
        tx.execute(
            "INSERT INTO workflow_steps (project_id, step_definition_id, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![project_id, ps.step_definition_id, ps.sort_order],
        )?;
    }
    tx.commit()?;
    get_workflow_steps_inner(&conn, project_id)
}

#[tauri::command]
pub fn get_workflow_steps(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<crate::db::models::WorkflowStep>, AppError> {
    let conn = lock_db(&db)?;
    get_workflow_steps_inner(&conn, project_id)
}

fn get_workflow_steps_inner(conn: &rusqlite::Connection, project_id: i64) -> Result<Vec<crate::db::models::WorkflowStep>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, step_definition_id, status, responsible, started_at, completed_at, notes, sort_order \
         FROM workflow_steps WHERE project_id = ?1 ORDER BY sort_order"
    )?;
    let steps = stmt.query_map([project_id], |row| {
        Ok(crate::db::models::WorkflowStep {
            id: row.get(0)?,
            project_id: row.get(1)?,
            step_definition_id: row.get(2)?,
            status: row.get(3)?,
            responsible: row.get(4)?,
            started_at: row.get(5)?,
            completed_at: row.get(6)?,
            notes: row.get(7)?,
            sort_order: row.get(8)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(steps)
}

#[tauri::command]
pub fn update_workflow_step(
    db: State<'_, DbState>,
    step_id: i64,
    status: Option<String>,
    responsible: Option<String>,
    notes: Option<String>,
) -> Result<crate::db::models::WorkflowStep, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(s) = &status {
        if !VALID_WF_STATUSES.contains(&s.as_str()) {
            return Err(AppError::Validation(format!("Ungueltiger Workflowstatus: {s}")));
        }
        params.push(Box::new(s.clone())); sets.push(format!("status = ?{}", params.len()));
        if s == "in_progress" {
            sets.push("started_at = COALESCE(started_at, datetime('now'))".to_string());
            sets.push("completed_at = NULL".to_string());
        } else if s == "completed" {
            sets.push("completed_at = datetime('now')".to_string());
        } else if s == "pending" {
            sets.push("started_at = NULL".to_string());
            sets.push("completed_at = NULL".to_string());
        }
    }
    if let Some(v) = &responsible { params.push(Box::new(v.clone())); sets.push(format!("responsible = ?{}", params.len())); }
    if let Some(v) = &notes { params.push(Box::new(v.clone())); sets.push(format!("notes = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, project_id, step_definition_id, status, responsible, started_at, completed_at, notes, sort_order \
             FROM workflow_steps WHERE id = ?1",
            [step_id], |row| {
                Ok(crate::db::models::WorkflowStep {
                    id: row.get(0)?, project_id: row.get(1)?, step_definition_id: row.get(2)?,
                    status: row.get(3)?, responsible: row.get(4)?, started_at: row.get(5)?,
                    completed_at: row.get(6)?, notes: row.get(7)?, sort_order: row.get(8)?,
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Workflowschritt {step_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    params.push(Box::new(step_id));
    let sql = format!("UPDATE workflow_steps SET {} WHERE id = ?{}", sets.join(", "), params.len());
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 { return Err(AppError::NotFound(format!("Workflowschritt {step_id} nicht gefunden"))); }

    conn.query_row(
        "SELECT id, project_id, step_definition_id, status, responsible, started_at, completed_at, notes, sort_order \
         FROM workflow_steps WHERE id = ?1",
        [step_id], |row| {
            Ok(crate::db::models::WorkflowStep {
                id: row.get(0)?, project_id: row.get(1)?, step_definition_id: row.get(2)?,
                status: row.get(3)?, responsible: row.get(4)?, started_at: row.get(5)?,
                completed_at: row.get(6)?, notes: row.get(7)?, sort_order: row.get(8)?,
            })
        },
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_workflow_step(db: State<'_, DbState>, step_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM workflow_steps WHERE id = ?1", [step_id])?;
    if changes == 0 { return Err(AppError::NotFound(format!("Workflowschritt {step_id} nicht gefunden"))); }
    Ok(())
}

// ── License Management ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseCreate {
    pub name: String,
    pub license_type: Option<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub max_uses: Option<i32>,
    pub commercial_allowed: Option<bool>,
    pub cost_per_piece: Option<f64>,
    pub cost_per_series: Option<f64>,
    pub cost_flat: Option<f64>,
    pub source: Option<String>,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn create_license(
    db: State<'_, DbState>,
    license: LicenseCreate,
) -> Result<crate::db::models::LicenseRecord, AppError> {
    let name = license.name.trim().to_string();
    if name.is_empty() { return Err(AppError::Validation("Lizenzname darf nicht leer sein".into())); }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO license_records (name, license_type, valid_from, valid_until, max_uses, commercial_allowed, cost_per_piece, cost_per_series, cost_flat, source, notes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![name, license.license_type, license.valid_from, license.valid_until,
            license.max_uses, license.commercial_allowed.unwrap_or(false) as i32,
            license.cost_per_piece.unwrap_or(0.0), license.cost_per_series.unwrap_or(0.0), license.cost_flat.unwrap_or(0.0),
            license.source, license.notes],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, license_type, valid_from, valid_until, max_uses, current_uses, commercial_allowed, cost_per_piece, cost_per_series, cost_flat, source, notes, created_at, updated_at \
         FROM license_records WHERE id = ?1",
        [id], row_to_license,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_licenses(db: State<'_, DbState>) -> Result<Vec<crate::db::models::LicenseRecord>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, license_type, valid_from, valid_until, max_uses, current_uses, commercial_allowed, cost_per_piece, cost_per_series, cost_flat, source, notes, created_at, updated_at \
         FROM license_records WHERE deleted_at IS NULL ORDER BY name"
    )?;
    let records = stmt.query_map([], row_to_license)?.collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

#[tauri::command]
pub fn get_license(db: State<'_, DbState>, license_id: i64) -> Result<crate::db::models::LicenseRecord, AppError> {
    let conn = lock_db(&db)?;
    conn.query_row(
        "SELECT id, name, license_type, valid_from, valid_until, max_uses, current_uses, commercial_allowed, cost_per_piece, cost_per_series, cost_flat, source, notes, created_at, updated_at \
         FROM license_records WHERE id = ?1 AND deleted_at IS NULL",
        [license_id], row_to_license,
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Lizenz {license_id} nicht gefunden")),
        _ => AppError::Database(e),
    })
}

#[tauri::command]
pub fn update_license(
    db: State<'_, DbState>,
    license_id: i64,
    name: Option<String>,
    license_type: Option<String>,
    valid_from: Option<String>,
    valid_until: Option<String>,
    max_uses: Option<i32>,
    commercial_allowed: Option<bool>,
    cost_per_piece: Option<f64>,
    cost_per_series: Option<f64>,
    cost_flat: Option<f64>,
    source: Option<String>,
    notes: Option<String>,
) -> Result<crate::db::models::LicenseRecord, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(n) = &name {
        let t = n.trim();
        if t.is_empty() { return Err(AppError::Validation("Lizenzname darf nicht leer sein".into())); }
        params.push(Box::new(t.to_string())); sets.push(format!("name = ?{}", params.len()));
    }
    if let Some(v) = &license_type { params.push(Box::new(v.clone())); sets.push(format!("license_type = ?{}", params.len())); }
    if let Some(v) = &valid_from { params.push(Box::new(v.clone())); sets.push(format!("valid_from = ?{}", params.len())); }
    if let Some(v) = &valid_until { params.push(Box::new(v.clone())); sets.push(format!("valid_until = ?{}", params.len())); }
    if let Some(v) = max_uses { params.push(Box::new(v)); sets.push(format!("max_uses = ?{}", params.len())); }
    if let Some(v) = commercial_allowed { params.push(Box::new(v as i32)); sets.push(format!("commercial_allowed = ?{}", params.len())); }
    if let Some(v) = cost_per_piece { params.push(Box::new(v)); sets.push(format!("cost_per_piece = ?{}", params.len())); }
    if let Some(v) = cost_per_series { params.push(Box::new(v)); sets.push(format!("cost_per_series = ?{}", params.len())); }
    if let Some(v) = cost_flat { params.push(Box::new(v)); sets.push(format!("cost_flat = ?{}", params.len())); }
    if let Some(v) = &source { params.push(Box::new(v.clone())); sets.push(format!("source = ?{}", params.len())); }
    if let Some(v) = &notes { params.push(Box::new(v.clone())); sets.push(format!("notes = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, name, license_type, valid_from, valid_until, max_uses, current_uses, commercial_allowed, cost_per_piece, cost_per_series, cost_flat, source, notes, created_at, updated_at \
             FROM license_records WHERE id = ?1",
            [license_id], row_to_license,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Lizenz {license_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    sets.push("updated_at = datetime('now')".to_string());
    params.push(Box::new(license_id));
    let sql = format!("UPDATE license_records SET {} WHERE id = ?{} AND deleted_at IS NULL", sets.join(", "), params.len());
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 { return Err(AppError::NotFound(format!("Lizenz {license_id} nicht gefunden"))); }

    conn.query_row(
        "SELECT id, name, license_type, valid_from, valid_until, max_uses, current_uses, commercial_allowed, cost_per_piece, cost_per_series, cost_flat, source, notes, created_at, updated_at \
         FROM license_records WHERE id = ?1",
        [license_id], row_to_license,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_license(db: State<'_, DbState>, license_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE license_records SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [license_id],
    )?;
    if changes == 0 { return Err(AppError::NotFound(format!("Lizenz {license_id} nicht gefunden"))); }
    Ok(())
}

#[tauri::command]
pub fn link_license_to_file(db: State<'_, DbState>, license_id: i64, file_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT OR IGNORE INTO license_file_links (license_id, file_id) VALUES (?1, ?2)",
        rusqlite::params![license_id, file_id],
    )?;
    Ok(())
}

#[tauri::command]
pub fn unlink_license_from_file(db: State<'_, DbState>, license_id: i64, file_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    conn.execute(
        "DELETE FROM license_file_links WHERE license_id = ?1 AND file_id = ?2",
        rusqlite::params![license_id, file_id],
    )?;
    Ok(())
}

#[tauri::command]
pub fn get_file_licenses(db: State<'_, DbState>, file_id: i64) -> Result<Vec<crate::db::models::LicenseRecord>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT l.id, l.name, l.license_type, l.valid_from, l.valid_until, l.max_uses, l.current_uses, l.commercial_allowed, l.cost_per_piece, l.cost_per_series, l.cost_flat, l.source, l.notes, l.created_at, l.updated_at \
         FROM license_records l JOIN license_file_links lf ON lf.license_id = l.id WHERE lf.file_id = ?1 AND l.deleted_at IS NULL ORDER BY l.name"
    )?;
    let records = stmt.query_map([file_id], row_to_license)?.collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

#[tauri::command]
pub fn get_expiring_licenses(db: State<'_, DbState>, days_ahead: Option<i32>) -> Result<Vec<crate::db::models::LicenseRecord>, AppError> {
    let days = days_ahead.unwrap_or(30);
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, license_type, valid_from, valid_until, max_uses, current_uses, commercial_allowed, cost_per_piece, cost_per_series, cost_flat, source, notes, created_at, updated_at \
         FROM license_records WHERE deleted_at IS NULL AND valid_until IS NOT NULL AND valid_until <= datetime('now', ?1) AND valid_until >= datetime('now') ORDER BY valid_until"
    )?;
    let threshold = format!("+{days} days");
    let records = stmt.query_map([&threshold], row_to_license)?.collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

fn row_to_license(row: &rusqlite::Row) -> rusqlite::Result<crate::db::models::LicenseRecord> {
    Ok(crate::db::models::LicenseRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        license_type: row.get(2)?,
        valid_from: row.get(3)?,
        valid_until: row.get(4)?,
        max_uses: row.get(5)?,
        current_uses: row.get(6)?,
        commercial_allowed: row.get::<_, i32>(7)? != 0,
        cost_per_piece: row.get::<_, Option<f64>>(8)?.unwrap_or(0.0),
        cost_per_series: row.get::<_, Option<f64>>(9)?.unwrap_or(0.0),
        cost_flat: row.get::<_, Option<f64>>(10)?.unwrap_or(0.0),
        source: row.get(11)?,
        notes: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

// ── Quality Inspections ────────────────────────────────────────────

const VALID_INSPECTION_RESULTS: &[&str] = &["pending", "passed", "failed", "rework"];
const VALID_DEFECT_SEVERITIES: &[&str] = &["minor", "major", "critical"];
const VALID_DEFECT_STATUSES: &[&str] = &["open", "rework", "resolved"];

#[tauri::command]
pub fn create_inspection(
    db: State<'_, DbState>,
    project_id: i64,
    workflow_step_id: Option<i64>,
    inspector: Option<String>,
    result: Option<String>,
    notes: Option<String>,
) -> Result<crate::db::models::QualityInspection, AppError> {
    let res = result.as_deref().unwrap_or("pending");
    if !VALID_INSPECTION_RESULTS.contains(&res) {
        return Err(AppError::Validation(format!("Ungueltiges Pruefergebnis: {res}")));
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO quality_inspections (project_id, workflow_step_id, inspector, result, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![project_id, workflow_step_id, inspector, res, notes],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, project_id, workflow_step_id, inspector, inspection_date, result, notes, created_at FROM quality_inspections WHERE id = ?1",
        [id], row_to_inspection,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_inspections(db: State<'_, DbState>, project_id: i64) -> Result<Vec<crate::db::models::QualityInspection>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, workflow_step_id, inspector, inspection_date, result, notes, created_at \
         FROM quality_inspections WHERE project_id = ?1 ORDER BY inspection_date DESC"
    )?;
    let inspections = stmt.query_map([project_id], row_to_inspection)?.collect::<Result<Vec<_>, _>>()?;
    Ok(inspections)
}

#[tauri::command]
pub fn update_inspection(
    db: State<'_, DbState>,
    inspection_id: i64,
    result: Option<String>,
    inspector: Option<String>,
    notes: Option<String>,
) -> Result<crate::db::models::QualityInspection, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(r) = &result {
        if !VALID_INSPECTION_RESULTS.contains(&r.as_str()) {
            return Err(AppError::Validation(format!("Ungueltiges Pruefergebnis: {r}")));
        }
        params.push(Box::new(r.clone())); sets.push(format!("result = ?{}", params.len()));
    }
    if let Some(v) = &inspector { params.push(Box::new(v.clone())); sets.push(format!("inspector = ?{}", params.len())); }
    if let Some(v) = &notes { params.push(Box::new(v.clone())); sets.push(format!("notes = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, project_id, workflow_step_id, inspector, inspection_date, result, notes, created_at FROM quality_inspections WHERE id = ?1",
            [inspection_id], row_to_inspection,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Pruefung {inspection_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    params.push(Box::new(inspection_id));
    let sql = format!("UPDATE quality_inspections SET {} WHERE id = ?{}", sets.join(", "), params.len());
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 { return Err(AppError::NotFound(format!("Pruefung {inspection_id} nicht gefunden"))); }

    conn.query_row(
        "SELECT id, project_id, workflow_step_id, inspector, inspection_date, result, notes, created_at FROM quality_inspections WHERE id = ?1",
        [inspection_id], row_to_inspection,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_inspection(db: State<'_, DbState>, inspection_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM quality_inspections WHERE id = ?1", [inspection_id])?;
    if changes == 0 { return Err(AppError::NotFound(format!("Pruefung {inspection_id} nicht gefunden"))); }
    Ok(())
}

fn row_to_inspection(row: &rusqlite::Row) -> rusqlite::Result<crate::db::models::QualityInspection> {
    Ok(crate::db::models::QualityInspection {
        id: row.get(0)?, project_id: row.get(1)?, workflow_step_id: row.get(2)?,
        inspector: row.get(3)?, inspection_date: row.get(4)?, result: row.get(5)?,
        notes: row.get(6)?, created_at: row.get(7)?,
    })
}

// ── Defect Records ─────────────────────────────────────────────────

#[tauri::command]
pub fn create_defect(
    db: State<'_, DbState>,
    inspection_id: i64,
    description: String,
    severity: Option<String>,
    notes: Option<String>,
) -> Result<crate::db::models::DefectRecord, AppError> {
    let desc = description.trim().to_string();
    if desc.is_empty() { return Err(AppError::Validation("Fehlerbeschreibung darf nicht leer sein".into())); }
    if let Some(s) = &severity {
        if !VALID_DEFECT_SEVERITIES.contains(&s.as_str()) {
            return Err(AppError::Validation(format!("Ungueltige Schwere: {s}")));
        }
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO defect_records (inspection_id, description, severity, notes) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![inspection_id, desc, severity.as_deref().unwrap_or("minor"), notes],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, inspection_id, description, severity, status, resolved_at, notes, created_at FROM defect_records WHERE id = ?1",
        [id], row_to_defect,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_defects(db: State<'_, DbState>, inspection_id: i64) -> Result<Vec<crate::db::models::DefectRecord>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, inspection_id, description, severity, status, resolved_at, notes, created_at FROM defect_records WHERE inspection_id = ?1 ORDER BY created_at DESC"
    )?;
    let defects = stmt.query_map([inspection_id], row_to_defect)?.collect::<Result<Vec<_>, _>>()?;
    Ok(defects)
}

#[tauri::command]
pub fn update_defect(
    db: State<'_, DbState>,
    defect_id: i64,
    description: Option<String>,
    severity: Option<String>,
    status: Option<String>,
    notes: Option<String>,
) -> Result<crate::db::models::DefectRecord, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(d) = &description {
        let t = d.trim();
        if t.is_empty() { return Err(AppError::Validation("Fehlerbeschreibung darf nicht leer sein".into())); }
        params.push(Box::new(t.to_string())); sets.push(format!("description = ?{}", params.len()));
    }
    if let Some(s) = &severity {
        if !VALID_DEFECT_SEVERITIES.contains(&s.as_str()) { return Err(AppError::Validation(format!("Ungueltige Schwere: {s}"))); }
        params.push(Box::new(s.clone())); sets.push(format!("severity = ?{}", params.len()));
    }
    if let Some(s) = &status {
        if !VALID_DEFECT_STATUSES.contains(&s.as_str()) { return Err(AppError::Validation(format!("Ungueltiger Fehlerstatus: {s}"))); }
        params.push(Box::new(s.clone())); sets.push(format!("status = ?{}", params.len()));
        if s == "resolved" { sets.push("resolved_at = datetime('now')".to_string()); }
    }
    if let Some(v) = &notes { params.push(Box::new(v.clone())); sets.push(format!("notes = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, inspection_id, description, severity, status, resolved_at, notes, created_at FROM defect_records WHERE id = ?1",
            [defect_id], row_to_defect,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Fehler {defect_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    params.push(Box::new(defect_id));
    let sql = format!("UPDATE defect_records SET {} WHERE id = ?{}", sets.join(", "), params.len());
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 { return Err(AppError::NotFound(format!("Fehler {defect_id} nicht gefunden"))); }

    conn.query_row(
        "SELECT id, inspection_id, description, severity, status, resolved_at, notes, created_at FROM defect_records WHERE id = ?1",
        [defect_id], row_to_defect,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_defect(db: State<'_, DbState>, defect_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM defect_records WHERE id = ?1", [defect_id])?;
    if changes == 0 { return Err(AppError::NotFound(format!("Fehler {defect_id} nicht gefunden"))); }
    Ok(())
}

fn row_to_defect(row: &rusqlite::Row) -> rusqlite::Result<crate::db::models::DefectRecord> {
    Ok(crate::db::models::DefectRecord {
        id: row.get(0)?, inspection_id: row.get(1)?, description: row.get(2)?,
        severity: row.get(3)?, status: row.get(4)?, resolved_at: row.get(5)?,
        notes: row.get(6)?, created_at: row.get(7)?,
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

    #[test]
    fn test_reservation_and_consumption_lifecycle() {
        let conn = init_database_in_memory().unwrap();

        // Setup: project, product, material, BOM, workflow
        conn.execute("INSERT INTO projects (name, status, quantity) VALUES ('Test', 'in_progress', 2)", []).unwrap();
        let pid = conn.last_insert_rowid();

        conn.execute("INSERT INTO products (name, status) VALUES ('Tasche', 'active')", []).unwrap();
        let prod_id = conn.last_insert_rowid();

        conn.execute("INSERT INTO materials (name, net_price, waste_factor) VALUES ('Stoff', 10.0, 0.05)", []).unwrap();
        let mat_id = conn.last_insert_rowid();

        conn.execute("INSERT INTO bill_of_materials (product_id, material_id, quantity) VALUES (?1, ?2, 3.0)",
            rusqlite::params![prod_id, mat_id]).unwrap();

        conn.execute("INSERT INTO step_definitions (name) VALUES ('Naehen')", []).unwrap();
        let step_id = conn.last_insert_rowid();
        conn.execute("INSERT INTO product_steps (product_id, step_definition_id) VALUES (?1, ?2)",
            rusqlite::params![prod_id, step_id]).unwrap();
        conn.execute("INSERT INTO workflow_steps (project_id, step_definition_id, status) VALUES (?1, ?2, 'pending')",
            rusqlite::params![pid, step_id]).unwrap();

        // Initial inventory: 20 units
        conn.execute("INSERT INTO material_inventory (material_id, total_stock, reserved_stock) VALUES (?1, 20.0, 0.0)",
            [mat_id]).unwrap();

        // 1. Reserve: qty=2, BOM=3.0 → reserve 6.0
        super::reserve_materials_for_project_inner(&conn, pid).unwrap();
        let reserved: f64 = conn.query_row(
            "SELECT reserved_stock FROM material_inventory WHERE material_id = ?1", [mat_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(reserved, 6.0);

        // Verify transaction logged
        let tx_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM inventory_transactions WHERE project_id = ?1 AND transaction_type = 'reserve'",
            [pid], |r| r.get(0),
        ).unwrap();
        assert_eq!(tx_count, 1);

        // 2. Consume 4.0 units
        conn.execute(
            "INSERT INTO material_consumptions (project_id, material_id, quantity) VALUES (?1, ?2, 4.0)",
            rusqlite::params![pid, mat_id],
        ).unwrap();
        // Simulate stock reduction (normally done by record_consumption command)
        conn.execute(
            "UPDATE material_inventory SET total_stock = total_stock - 4.0, reserved_stock = MAX(0, reserved_stock - 4.0) WHERE material_id = ?1",
            [mat_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO inventory_transactions (material_id, project_id, transaction_type, quantity) VALUES (?1, ?2, 'consume', 4.0)",
            rusqlite::params![mat_id, pid],
        ).unwrap();

        let (total, reserved): (f64, f64) = conn.query_row(
            "SELECT total_stock, reserved_stock FROM material_inventory WHERE material_id = ?1", [mat_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(total, 16.0); // 20 - 4
        assert_eq!(reserved, 2.0); // 6 - 4

        // 3. Release remaining reservations
        super::release_project_reservations_inner(&conn, pid).unwrap();
        let reserved_after: f64 = conn.query_row(
            "SELECT reserved_stock FROM material_inventory WHERE material_id = ?1", [mat_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(reserved_after, 0.0); // 2 - 2 = 0

        // 4. Nachkalkulation — verify data via SQL (command requires Tauri State)
        let (planned, actual): (f64, f64) = conn.query_row(
            "SELECT \
                 COALESCE((SELECT SUM(b.quantity) FROM bill_of_materials b \
                     WHERE b.product_id IN (SELECT DISTINCT ps.product_id FROM product_steps ps \
                         JOIN workflow_steps ws ON ws.step_definition_id = ps.step_definition_id \
                         WHERE ws.project_id = ?1) \
                     AND b.material_id = ?2), 0) * 2, \
                 COALESCE((SELECT SUM(c.quantity) FROM material_consumptions c \
                     WHERE c.project_id = ?1 AND c.material_id = ?2), 0)",
            rusqlite::params![pid, mat_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(planned, 6.0); // BOM 3.0 × qty 2
        assert_eq!(actual, 4.0);   // consumed 4.0
    }

    #[test]
    fn test_product_variant_crud() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO products (name, status) VALUES ('Tasche', 'active')", []).unwrap();
        let prod_id = conn.last_insert_rowid();

        // Create variant
        conn.execute(
            "INSERT INTO product_variants (product_id, sku, variant_name, size, color, additional_cost) \
             VALUES (?1, 'T-L-ROT', 'Gross Rot', 'L', 'Rot', 2.50)",
            [prod_id],
        ).unwrap();
        let vid = conn.last_insert_rowid();

        // Read variant
        let (sku, size, color): (String, String, String) = conn.query_row(
            "SELECT sku, size, color FROM product_variants WHERE id = ?1", [vid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!(sku, "T-L-ROT");
        assert_eq!(size, "L");
        assert_eq!(color, "Rot");

        // SKU uniqueness
        let dup = conn.execute(
            "INSERT INTO product_variants (product_id, sku) VALUES (?1, 'T-L-ROT')",
            [prod_id],
        );
        assert!(dup.is_err(), "Duplicate SKU should fail");

        // Soft delete
        conn.execute(
            "UPDATE product_variants SET deleted_at = datetime('now') WHERE id = ?1", [vid],
        ).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM product_variants WHERE product_id = ?1 AND deleted_at IS NULL",
            [prod_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 0);

        // Cascade delete
        conn.execute(
            "INSERT INTO product_variants (product_id, sku, size) VALUES (?1, 'T-M-BLU', 'M')", [prod_id],
        ).unwrap();
        conn.execute("DELETE FROM products WHERE id = ?1", [prod_id]).unwrap();
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM product_variants", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(total, 0, "Cascade delete should remove variants");
    }
}
