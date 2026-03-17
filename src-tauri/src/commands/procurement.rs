use serde::Deserialize;
use tauri::State;

use crate::db::models::{Delivery, MaterialRequirement, OrderItem, PurchaseOrder};
use crate::error::{lock_db, AppError};
use crate::DbState;

// ── Purchase Orders ────────────────────────────────────────────────

const VALID_ORDER_STATUSES: &[&str] = &[
    "draft", "ordered", "partially_delivered", "delivered", "cancelled",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderCreate {
    pub order_number: Option<String>,
    pub supplier_id: i64,
    pub project_id: Option<i64>,
    pub order_date: Option<String>,
    pub expected_delivery: Option<String>,
    pub shipping_cost: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderUpdate {
    pub order_number: Option<String>,
    pub status: Option<String>,
    pub project_id: Option<i64>,
    pub clear_project_id: Option<bool>,
    pub order_date: Option<String>,
    pub expected_delivery: Option<String>,
    pub shipping_cost: Option<f64>,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn create_order(
    db: State<'_, DbState>,
    order: OrderCreate,
) -> Result<PurchaseOrder, AppError> {
    let conn = lock_db(&db)?;
    // Validate supplier exists and is not soft-deleted
    let supplier_active: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM suppliers WHERE id = ?1 AND deleted_at IS NULL",
        [order.supplier_id], |row| row.get(0),
    )?;
    if !supplier_active {
        return Err(AppError::Validation(format!("Lieferant {} nicht gefunden oder geloescht", order.supplier_id)));
    }
    // Validate project exists if provided
    if let Some(pid) = order.project_id {
        let project_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM projects WHERE id = ?1 AND deleted_at IS NULL",
            [pid], |row| row.get(0),
        )?;
        if !project_exists {
            return Err(AppError::NotFound(format!("Projekt {pid} nicht gefunden")));
        }
    }
    conn.execute(
        "INSERT INTO purchase_orders (order_number, supplier_id, project_id, order_date, expected_delivery, shipping_cost, notes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![order.order_number, order.supplier_id, order.project_id, order.order_date, order.expected_delivery, order.shipping_cost.unwrap_or(0.0), order.notes],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, order_number, supplier_id, project_id, status, order_date, expected_delivery, shipping_cost, notes, created_at, updated_at \
         FROM purchase_orders WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        row_to_order,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_orders(db: State<'_, DbState>) -> Result<Vec<PurchaseOrder>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, order_number, supplier_id, project_id, status, order_date, expected_delivery, shipping_cost, notes, created_at, updated_at \
         FROM purchase_orders WHERE deleted_at IS NULL ORDER BY created_at DESC"
    )?;
    let orders = stmt.query_map([], row_to_order)?.collect::<Result<Vec<_>, _>>()?;
    Ok(orders)
}

#[tauri::command]
pub fn get_order(db: State<'_, DbState>, order_id: i64) -> Result<PurchaseOrder, AppError> {
    let conn = lock_db(&db)?;
    conn.query_row(
        "SELECT id, order_number, supplier_id, project_id, status, order_date, expected_delivery, shipping_cost, notes, created_at, updated_at \
         FROM purchase_orders WHERE id = ?1 AND deleted_at IS NULL",
        [order_id],
        row_to_order,
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Bestellung {order_id} nicht gefunden")),
        _ => AppError::Database(e),
    })
}

#[tauri::command]
pub fn update_order(
    db: State<'_, DbState>,
    order_id: i64,
    update: OrderUpdate,
) -> Result<PurchaseOrder, AppError> {
    let conn = lock_db(&db)?;

    // Capture old values for audit
    let old_status: String = conn.query_row(
        "SELECT COALESCE(status, 'draft') FROM purchase_orders WHERE id = ?1 AND deleted_at IS NULL",
        [order_id], |row| row.get(0),
    ).unwrap_or_default();

    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(v) = &update.order_number { params.push(Box::new(v.clone())); sets.push(format!("order_number = ?{}", params.len())); }
    if let Some(v) = &update.status {
        if !VALID_ORDER_STATUSES.contains(&v.as_str()) {
            return Err(AppError::Validation(format!("Ungueltiger Bestellstatus: {v}")));
        }
        params.push(Box::new(v.clone())); sets.push(format!("status = ?{}", params.len()));
    }
    if update.clear_project_id == Some(true) {
        params.push(Box::new(rusqlite::types::Null)); sets.push(format!("project_id = ?{}", params.len()));
    } else if let Some(v) = update.project_id {
        params.push(Box::new(v)); sets.push(format!("project_id = ?{}", params.len()));
    }
    if let Some(v) = &update.order_date { params.push(Box::new(v.clone())); sets.push(format!("order_date = ?{}", params.len())); }
    if let Some(v) = &update.expected_delivery { params.push(Box::new(v.clone())); sets.push(format!("expected_delivery = ?{}", params.len())); }
    if let Some(v) = update.shipping_cost { params.push(Box::new(v)); sets.push(format!("shipping_cost = ?{}", params.len())); }
    if let Some(v) = &update.notes { params.push(Box::new(v.clone())); sets.push(format!("notes = ?{}", params.len())); }

    if sets.is_empty() {
        return conn.query_row(
            "SELECT id, order_number, supplier_id, project_id, status, order_date, expected_delivery, shipping_cost, notes, created_at, updated_at \
             FROM purchase_orders WHERE id = ?1 AND deleted_at IS NULL",
            [order_id], row_to_order,
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Bestellung {order_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    sets.push("updated_at = datetime('now')".to_string());
    params.push(Box::new(order_id));
    let sql = format!("UPDATE purchase_orders SET {} WHERE id = ?{} AND deleted_at IS NULL", sets.join(", "), params.len());
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 { return Err(AppError::NotFound(format!("Bestellung {order_id} nicht gefunden"))); }

    // Audit logging for order status changes
    if let Some(v) = &update.status {
        let _ = crate::commands::audit::log_change(&conn, "order", order_id, "status", Some(&old_status), Some(v));
    }

    conn.query_row(
        "SELECT id, order_number, supplier_id, project_id, status, order_date, expected_delivery, shipping_cost, notes, created_at, updated_at \
         FROM purchase_orders WHERE id = ?1 AND deleted_at IS NULL",
        [order_id], row_to_order,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_order(db: State<'_, DbState>, order_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE purchase_orders SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [order_id],
    )?;
    if changes == 0 { return Err(AppError::NotFound(format!("Bestellung {order_id} nicht gefunden"))); }
    Ok(())
}

fn row_to_order(row: &rusqlite::Row) -> rusqlite::Result<PurchaseOrder> {
    Ok(PurchaseOrder {
        id: row.get(0)?,
        order_number: row.get(1)?,
        supplier_id: row.get(2)?,
        project_id: row.get(3)?,
        status: row.get(4)?,
        order_date: row.get(5)?,
        expected_delivery: row.get(6)?,
        shipping_cost: row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
        notes: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

// ── Order Items ────────────────────────────────────────────────────

#[tauri::command]
pub fn add_order_item(
    db: State<'_, DbState>,
    order_id: i64,
    material_id: i64,
    quantity_ordered: f64,
    unit_price: Option<f64>,
    notes: Option<String>,
) -> Result<OrderItem, AppError> {
    if quantity_ordered <= 0.0 {
        return Err(AppError::Validation("Bestellmenge muss groesser als 0 sein".into()));
    }
    let conn = lock_db(&db)?;
    // Verify order is not soft-deleted
    let order_active: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM purchase_orders WHERE id = ?1 AND deleted_at IS NULL",
        [order_id], |row| row.get(0),
    )?;
    if !order_active {
        return Err(AppError::NotFound(format!("Bestellung {order_id} nicht gefunden")));
    }
    conn.execute(
        "INSERT INTO order_items (order_id, material_id, quantity_ordered, unit_price, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![order_id, material_id, quantity_ordered, unit_price, notes],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, order_id, material_id, quantity_ordered, quantity_delivered, unit_price, notes FROM order_items WHERE id = ?1",
        [id], row_to_order_item,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_order_items(db: State<'_, DbState>, order_id: i64) -> Result<Vec<OrderItem>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, order_id, material_id, quantity_ordered, quantity_delivered, unit_price, notes FROM order_items WHERE order_id = ?1"
    )?;
    let items = stmt.query_map([order_id], row_to_order_item)?.collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

#[tauri::command]
pub fn delete_order_item(db: State<'_, DbState>, item_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM order_items WHERE id = ?1", [item_id])?;
    if changes == 0 { return Err(AppError::NotFound(format!("Position {item_id} nicht gefunden"))); }
    Ok(())
}

fn row_to_order_item(row: &rusqlite::Row) -> rusqlite::Result<OrderItem> {
    Ok(OrderItem {
        id: row.get(0)?,
        order_id: row.get(1)?,
        material_id: row.get(2)?,
        quantity_ordered: row.get(3)?,
        quantity_delivered: row.get(4)?,
        unit_price: row.get(5)?,
        notes: row.get(6)?,
    })
}

// ── Deliveries ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryItemInput {
    pub order_item_id: i64,
    pub quantity_received: f64,
}

#[tauri::command]
pub fn record_delivery(
    db: State<'_, DbState>,
    order_id: i64,
    delivery_note: Option<String>,
    notes: Option<String>,
    items: Vec<DeliveryItemInput>,
) -> Result<Delivery, AppError> {
    if items.is_empty() {
        return Err(AppError::Validation("Lieferung muss mindestens eine Position enthalten".into()));
    }
    for item in &items {
        if item.quantity_received <= 0.0 {
            return Err(AppError::Validation("Empfangene Menge muss groesser als 0 sein".into()));
        }
    }

    let conn = lock_db(&db)?;

    // Validate order exists and is not soft-deleted
    let order_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM purchase_orders WHERE id = ?1 AND deleted_at IS NULL",
        [order_id], |row| row.get(0),
    )?;
    if !order_exists {
        return Err(AppError::NotFound(format!("Bestellung {order_id} nicht gefunden")));
    }

    // Validate all order items belong to this order and check over-delivery
    for item in &items {
        let (belongs, qty_ordered, qty_delivered): (bool, f64, f64) = conn.query_row(
            "SELECT order_id = ?1, quantity_ordered, quantity_delivered FROM order_items WHERE id = ?2",
            rusqlite::params![order_id, item.order_item_id],
            |row| Ok((row.get::<_, bool>(0)?, row.get(1)?, row.get(2)?)),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(
                format!("Bestellposition {} nicht gefunden", item.order_item_id)
            ),
            _ => AppError::Database(e),
        })?;
        if !belongs {
            return Err(AppError::Validation(format!(
                "Position {} gehoert nicht zu Bestellung {order_id}", item.order_item_id
            )));
        }
        if qty_delivered + item.quantity_received > qty_ordered * 1.1 {
            return Err(AppError::Validation(format!(
                "Ueberlieferung: Position {} wuerde {:.1} von {:.1} bestellten Einheiten erreichen",
                item.order_item_id, qty_delivered + item.quantity_received, qty_ordered
            )));
        }
    }

    let tx = conn.unchecked_transaction()?;

    // Create delivery
    tx.execute(
        "INSERT INTO deliveries (order_id, delivery_note, notes) VALUES (?1, ?2, ?3)",
        rusqlite::params![order_id, delivery_note, notes],
    )?;
    let delivery_id = tx.last_insert_rowid();

    // Create delivery items and update order_items.quantity_delivered + inventory
    for item in &items {
        tx.execute(
            "INSERT INTO delivery_items (delivery_id, order_item_id, quantity_received) VALUES (?1, ?2, ?3)",
            rusqlite::params![delivery_id, item.order_item_id, item.quantity_received],
        )?;

        // Update delivered quantity on order item
        tx.execute(
            "UPDATE order_items SET quantity_delivered = quantity_delivered + ?1 WHERE id = ?2",
            rusqlite::params![item.quantity_received, item.order_item_id],
        )?;

        // Update material inventory total_stock
        let material_id: i64 = tx.query_row(
            "SELECT material_id FROM order_items WHERE id = ?1",
            [item.order_item_id],
            |row| row.get(0),
        )?;
        let inv_changes = tx.execute(
            "UPDATE material_inventory SET total_stock = total_stock + ?1, updated_at = datetime('now') WHERE material_id = ?2",
            rusqlite::params![item.quantity_received, material_id],
        )?;
        if inv_changes == 0 {
            // Create inventory record if it doesn't exist
            tx.execute(
                "INSERT INTO material_inventory (material_id, total_stock) VALUES (?1, ?2)",
                rusqlite::params![material_id, item.quantity_received],
            )?;
        }
    }

    // Auto-update order status based on delivery completeness
    let all_delivered: bool = tx.query_row(
        "SELECT COUNT(*) = 0 FROM order_items WHERE order_id = ?1 AND quantity_delivered < quantity_ordered",
        [order_id],
        |row| row.get(0),
    )?;
    let new_status = if all_delivered { "delivered" } else { "partially_delivered" };
    tx.execute(
        "UPDATE purchase_orders SET status = ?1, updated_at = datetime('now') WHERE id = ?2 AND deleted_at IS NULL",
        rusqlite::params![new_status, order_id],
    )?;

    tx.commit()?;

    conn.query_row(
        "SELECT id, order_id, delivery_date, delivery_note, notes FROM deliveries WHERE id = ?1",
        [delivery_id],
        row_to_delivery,
    ).map_err(AppError::Database)
}

#[tauri::command]
pub fn get_deliveries(db: State<'_, DbState>, order_id: i64) -> Result<Vec<Delivery>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, order_id, delivery_date, delivery_note, notes FROM deliveries WHERE order_id = ?1 ORDER BY delivery_date DESC"
    )?;
    let deliveries = stmt.query_map([order_id], row_to_delivery)?.collect::<Result<Vec<_>, _>>()?;
    Ok(deliveries)
}

fn row_to_delivery(row: &rusqlite::Row) -> rusqlite::Result<Delivery> {
    Ok(Delivery {
        id: row.get(0)?,
        order_id: row.get(1)?,
        delivery_date: row.get(2)?,
        delivery_note: row.get(3)?,
        notes: row.get(4)?,
    })
}

// ── Project-Order Queries ──────────────────────────────────────────

#[tauri::command]
pub fn get_project_orders(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<PurchaseOrder>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, order_number, supplier_id, project_id, status, order_date, expected_delivery, shipping_cost, notes, created_at, updated_at \
         FROM purchase_orders WHERE project_id = ?1 AND deleted_at IS NULL ORDER BY created_at DESC"
    )?;
    let orders = stmt.query_map([project_id], row_to_order)?.collect::<Result<Vec<_>, _>>()?;
    Ok(orders)
}

#[tauri::command]
pub fn get_project_requirements(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<MaterialRequirement>, AppError> {
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

    let mut stmt = conn.prepare(
        "SELECT m.id, m.name, m.unit, \
             COALESCE(bom.total_qty, 0) * ?2 as needed, \
             COALESCE(inv.total_stock, 0) - COALESCE(inv.reserved_stock, 0) as available, \
             m.supplier_id, \
             s.name as supplier_name \
         FROM materials m \
         JOIN ( \
             SELECT b.material_id, SUM(b.quantity) as total_qty \
             FROM bill_of_materials b \
             WHERE b.product_id IN ( \
                 SELECT pp.product_id FROM project_products pp WHERE pp.project_id = ?1 \
             ) GROUP BY b.material_id \
         ) bom ON bom.material_id = m.id \
         LEFT JOIN material_inventory inv ON inv.material_id = m.id \
         LEFT JOIN suppliers s ON s.id = m.supplier_id AND s.deleted_at IS NULL \
         WHERE m.deleted_at IS NULL \
         ORDER BY m.name"
    )?;

    let requirements = stmt.query_map(rusqlite::params![project_id, qty], |row| {
        let needed: f64 = row.get(3)?;
        let available: f64 = row.get(4)?;
        let shortage = (needed - available).max(0.0);
        Ok(MaterialRequirement {
            material_id: row.get(0)?,
            material_name: row.get(1)?,
            unit: row.get(2)?,
            needed,
            available,
            shortage,
            supplier_id: row.get(5)?,
            supplier_name: row.get(6)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(requirements)
}

#[tauri::command]
pub fn suggest_orders(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<MaterialRequirement>, AppError> {
    let requirements = get_project_requirements(db, project_id)?;
    // Return only materials with shortage > 0
    Ok(requirements.into_iter().filter(|r| r.shortage > 0.0).collect())
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_order_crud() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO suppliers (name) VALUES ('TestSupplier')", []).unwrap();

        conn.execute(
            "INSERT INTO purchase_orders (order_number, supplier_id, status) VALUES ('PO-001', 1, 'draft')",
            [],
        ).unwrap();
        let oid = conn.last_insert_rowid();

        let status: String = conn.query_row(
            "SELECT status FROM purchase_orders WHERE id = ?1 AND deleted_at IS NULL", [oid], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "draft");

        // Soft delete
        conn.execute("UPDATE purchase_orders SET deleted_at = datetime('now') WHERE id = ?1", [oid]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM purchase_orders WHERE deleted_at IS NULL", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_order_items_and_delivery() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO suppliers (name) VALUES ('S1')", []).unwrap();
        conn.execute("INSERT INTO materials (name, unit) VALUES ('Stoff', 'm')", []).unwrap();
        conn.execute("INSERT INTO material_inventory (material_id, total_stock) VALUES (1, 100.0)", []).unwrap();
        conn.execute(
            "INSERT INTO purchase_orders (supplier_id, status) VALUES (1, 'ordered')", [],
        ).unwrap();
        let oid = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO order_items (order_id, material_id, quantity_ordered) VALUES (?1, 1, 50.0)",
            [oid],
        ).unwrap();
        let item_id = conn.last_insert_rowid();

        // Record delivery
        conn.execute(
            "INSERT INTO deliveries (order_id) VALUES (?1)", [oid],
        ).unwrap();
        let did = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO delivery_items (delivery_id, order_item_id, quantity_received) VALUES (?1, ?2, 30.0)",
            rusqlite::params![did, item_id],
        ).unwrap();

        // Update delivered quantity manually (in real code, record_delivery does this)
        conn.execute(
            "UPDATE order_items SET quantity_delivered = quantity_delivered + 30.0 WHERE id = ?1",
            [item_id],
        ).unwrap();
        conn.execute(
            "UPDATE material_inventory SET total_stock = total_stock + 30.0 WHERE material_id = 1",
            [],
        ).unwrap();

        let delivered: f64 = conn.query_row(
            "SELECT quantity_delivered FROM order_items WHERE id = ?1", [item_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(delivered, 30.0);

        let stock: f64 = conn.query_row(
            "SELECT total_stock FROM material_inventory WHERE material_id = 1", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(stock, 130.0);

        // Cascade delete
        conn.execute("DELETE FROM purchase_orders WHERE id = ?1", [oid]).unwrap();
        let items: i64 = conn.query_row("SELECT COUNT(*) FROM order_items", [], |r| r.get(0)).unwrap();
        assert_eq!(items, 0);
    }

    #[test]
    fn test_project_order_linkage() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO suppliers (name) VALUES ('TestSup')", []).unwrap();
        conn.execute("INSERT INTO projects (name, status, quantity) VALUES ('ProjA', 'in_progress', 2)", []).unwrap();
        let pid = conn.last_insert_rowid();

        // Create order linked to project
        conn.execute(
            "INSERT INTO purchase_orders (supplier_id, project_id, status) VALUES (1, ?1, 'draft')",
            [pid],
        ).unwrap();
        let oid = conn.last_insert_rowid();

        let project_id: Option<i64> = conn.query_row(
            "SELECT project_id FROM purchase_orders WHERE id = ?1", [oid], |r| r.get(0),
        ).unwrap();
        assert_eq!(project_id, Some(pid));

        // Query by project
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM purchase_orders WHERE project_id = ?1 AND deleted_at IS NULL",
            [pid], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_project_requirements() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO projects (name, status, quantity) VALUES ('Req', 'in_progress', 3)", []).unwrap();
        let pid = conn.last_insert_rowid();

        conn.execute("INSERT INTO products (name, status) VALUES ('Prod', 'active')", []).unwrap();
        let prod_id = conn.last_insert_rowid();

        conn.execute("INSERT INTO materials (name, unit, net_price) VALUES ('Garn', 'm', 2.0)", []).unwrap();
        let mat_id = conn.last_insert_rowid();

        conn.execute("INSERT INTO bill_of_materials (product_id, material_id, quantity) VALUES (?1, ?2, 5.0)",
            rusqlite::params![prod_id, mat_id]).unwrap();

        // Link product to project via project_products
        conn.execute("INSERT INTO project_products (project_id, product_id) VALUES (?1, ?2)",
            rusqlite::params![pid, prod_id]).unwrap();

        conn.execute("INSERT INTO step_definitions (name) VALUES ('Step')", []).unwrap();
        let step_id = conn.last_insert_rowid();
        conn.execute("INSERT INTO product_steps (product_id, step_definition_id) VALUES (?1, ?2)",
            rusqlite::params![prod_id, step_id]).unwrap();
        conn.execute("INSERT INTO workflow_steps (project_id, step_definition_id, status) VALUES (?1, ?2, 'pending')",
            rusqlite::params![pid, step_id]).unwrap();

        // Inventory: 10 available
        conn.execute("INSERT INTO material_inventory (material_id, total_stock, reserved_stock) VALUES (?1, 10.0, 0.0)",
            [mat_id]).unwrap();

        // Requirements: needed = 5.0 * 3 = 15.0, available = 10.0, shortage = 5.0
        let needed: f64 = conn.query_row(
            "SELECT COALESCE(SUM(b.quantity), 0) * 3 FROM bill_of_materials b \
             WHERE b.product_id IN (SELECT pp.product_id FROM project_products pp \
                 WHERE pp.project_id = ?1) AND b.material_id = ?2",
            rusqlite::params![pid, mat_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(needed, 15.0);

        let available: f64 = conn.query_row(
            "SELECT total_stock - reserved_stock FROM material_inventory WHERE material_id = ?1",
            [mat_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(available, 10.0);
        // shortage = 15 - 10 = 5
    }
}
