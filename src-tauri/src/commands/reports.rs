use tauri::State;

use crate::db::models::{CostBreakdown, CostRate, ProjectReport};
use crate::error::{lock_db, AppError};
use crate::DbState;

// ── Cost Rate CRUD ───────────────────────────────────────────────────

fn row_to_cost_rate(row: &rusqlite::Row) -> rusqlite::Result<CostRate> {
    Ok(CostRate {
        id: row.get(0)?,
        rate_type: row.get(1)?,
        name: row.get(2)?,
        rate_value: row.get(3)?,
        unit: row.get(4)?,
        setup_cost: row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
        notes: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

const COST_RATE_SELECT: &str =
    "SELECT id, rate_type, name, rate_value, unit, setup_cost, notes, created_at, updated_at FROM cost_rates";

const VALID_RATE_TYPES: &[&str] = &["labor", "machine", "overhead", "profit", "stitch"];

#[tauri::command]
pub fn list_cost_rates(
    db: State<'_, DbState>,
    rate_type: Option<String>,
) -> Result<Vec<CostRate>, AppError> {
    let conn = lock_db(&db)?;
    if let Some(rt) = &rate_type {
        let sql = format!("{COST_RATE_SELECT} WHERE rate_type = ?1 AND deleted_at IS NULL ORDER BY name");
        let mut stmt = conn.prepare(&sql)?;
        let rates = stmt.query_map([rt], row_to_cost_rate)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rates)
    } else {
        let sql = format!("{COST_RATE_SELECT} WHERE deleted_at IS NULL ORDER BY rate_type, name");
        let mut stmt = conn.prepare(&sql)?;
        let rates = stmt.query_map([], row_to_cost_rate)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rates)
    }
}

#[tauri::command]
pub fn create_cost_rate(
    db: State<'_, DbState>,
    rate_type: String,
    name: String,
    rate_value: f64,
    unit: Option<String>,
    setup_cost: Option<f64>,
    notes: Option<String>,
) -> Result<CostRate, AppError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation("Name darf nicht leer sein".into()));
    }
    if !VALID_RATE_TYPES.contains(&rate_type.as_str()) {
        return Err(AppError::Validation(format!("Ungueltiger Ratentyp: {rate_type}")));
    }
    if rate_value < 0.0 {
        return Err(AppError::Validation("Ratenwert darf nicht negativ sein".into()));
    }
    if setup_cost.unwrap_or(0.0) < 0.0 {
        return Err(AppError::Validation("Ruestkosten duerfen nicht negativ sein".into()));
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO cost_rates (rate_type, name, rate_value, unit, setup_cost, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![rate_type, name, rate_value, unit, setup_cost.unwrap_or(0.0), notes],
    )?;
    let id = conn.last_insert_rowid();
    let sql = format!("{COST_RATE_SELECT} WHERE id = ?1");
    conn.query_row(&sql, [id], row_to_cost_rate).map_err(AppError::Database)
}

#[tauri::command]
pub fn update_cost_rate(
    db: State<'_, DbState>,
    rate_id: i64,
    name: Option<String>,
    rate_value: Option<f64>,
    unit: Option<String>,
    setup_cost: Option<f64>,
    notes: Option<String>,
) -> Result<CostRate, AppError> {
    let conn = lock_db(&db)?;
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(n) = &name {
        let t = n.trim();
        if t.is_empty() { return Err(AppError::Validation("Name darf nicht leer sein".into())); }
        params.push(Box::new(t.to_string())); sets.push(format!("name = ?{}", params.len()));
    }
    if let Some(v) = rate_value {
        if v < 0.0 { return Err(AppError::Validation("Ratenwert darf nicht negativ sein".into())); }
        params.push(Box::new(v)); sets.push(format!("rate_value = ?{}", params.len()));
    }
    if let Some(v) = &unit { params.push(Box::new(v.clone())); sets.push(format!("unit = ?{}", params.len())); }
    if let Some(v) = setup_cost {
        if v < 0.0 { return Err(AppError::Validation("Ruestkosten duerfen nicht negativ sein".into())); }
        params.push(Box::new(v)); sets.push(format!("setup_cost = ?{}", params.len()));
    }
    if let Some(v) = &notes { params.push(Box::new(v.clone())); sets.push(format!("notes = ?{}", params.len())); }

    if sets.is_empty() {
        let sql = format!("{COST_RATE_SELECT} WHERE id = ?1 AND deleted_at IS NULL");
        return conn.query_row(&sql, [rate_id], row_to_cost_rate).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Kostensatz {rate_id} nicht gefunden")),
            _ => AppError::Database(e),
        });
    }

    sets.push("updated_at = datetime('now')".to_string());
    params.push(Box::new(rate_id));
    let sql = format!("UPDATE cost_rates SET {} WHERE id = ?{} AND deleted_at IS NULL", sets.join(", "), params.len());
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;
    if changes == 0 { return Err(AppError::NotFound(format!("Kostensatz {rate_id} nicht gefunden"))); }

    let sql = format!("{COST_RATE_SELECT} WHERE id = ?1");
    conn.query_row(&sql, [rate_id], row_to_cost_rate).map_err(AppError::Database)
}

#[tauri::command]
pub fn delete_cost_rate(db: State<'_, DbState>, rate_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE cost_rates SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [rate_id],
    )?;
    if changes == 0 { return Err(AppError::NotFound(format!("Kostensatz {rate_id} nicht gefunden"))); }
    Ok(())
}

// ── Project-License Links ────────────────────────────────────────────

#[tauri::command]
pub fn link_license_to_project(
    db: State<'_, DbState>,
    project_id: i64,
    license_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT OR IGNORE INTO project_license_links (project_id, license_id) VALUES (?1, ?2)",
        rusqlite::params![project_id, license_id],
    )?;
    Ok(())
}

#[tauri::command]
pub fn unlink_license_from_project(
    db: State<'_, DbState>,
    project_id: i64,
    license_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    conn.execute(
        "DELETE FROM project_license_links WHERE project_id = ?1 AND license_id = ?2",
        rusqlite::params![project_id, license_id],
    )?;
    Ok(())
}

#[tauri::command]
pub fn get_project_licenses(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<Vec<crate::db::models::LicenseRecord>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT l.id, l.name, l.license_type, l.valid_from, l.valid_until, l.max_uses, l.current_uses, \
         l.commercial_allowed, l.cost_per_piece, l.cost_per_series, l.cost_flat, l.source, l.notes, l.created_at, l.updated_at \
         FROM license_records l JOIN project_license_links pl ON pl.license_id = l.id \
         WHERE pl.project_id = ?1 AND l.deleted_at IS NULL ORDER BY l.name"
    )?;
    let records = stmt.query_map([project_id], |row| {
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
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

// ── Cost Breakdown Calculation ───────────────────────────────────────

/// Calculate the full cost breakdown for a project (project.md 7.2–7.3).
fn calculate_cost_breakdown(conn: &rusqlite::Connection, project_id: i64) -> Result<CostBreakdown, AppError> {
    // Project name and quantity
    let (project_name, quantity): (String, i64) = conn.query_row(
        "SELECT name, COALESCE(quantity, 1) FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
        _ => AppError::Database(e),
    })?;

    let quantity = quantity.max(1);

    // 1. Materialkosten: BOM × net_price × (1 + waste_factor)
    let material_cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM(b.quantity * COALESCE(m.net_price, 0) * (1 + COALESCE(m.waste_factor, 0))), 0) \
         FROM bill_of_materials b \
         JOIN materials m ON m.id = b.material_id AND m.deleted_at IS NULL \
         WHERE b.product_id IN ( \
             SELECT DISTINCT ps.product_id FROM product_steps ps \
             JOIN workflow_steps ws ON ws.step_definition_id = ps.step_definition_id \
             WHERE ws.project_id = ?1 \
         )",
        [project_id],
        |row| row.get(0),
    )?;

    // 2. Lizenzkosten: sum from project-linked licenses
    let license_cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM( \
             COALESCE(l.cost_per_piece, 0) * ?2 + \
             COALESCE(l.cost_per_series, 0) + \
             COALESCE(l.cost_flat, 0) \
         ), 0) \
         FROM license_records l \
         JOIN project_license_links pl ON pl.license_id = l.id \
         WHERE pl.project_id = ?1 AND l.deleted_at IS NULL",
        rusqlite::params![project_id, quantity],
        |row| row.get(0),
    )?;

    // 2b. Stickkosten: stitch_count from project's pattern file * stitch rate per 1000
    let stitch_count: i64 = conn.query_row(
        "SELECT COALESCE( \
             (SELECT COALESCE(e.stitch_count, 0) FROM embroidery_files e \
              JOIN projects p ON p.pattern_file_id = e.id \
              WHERE p.id = ?1 AND p.deleted_at IS NULL AND e.deleted_at IS NULL), \
             0)",
        [project_id],
        |row| row.get(0),
    )?;

    let stitch_rate: f64 = conn.query_row(
        "SELECT COALESCE((SELECT rate_value FROM cost_rates WHERE rate_type = 'stitch' AND deleted_at IS NULL ORDER BY id LIMIT 1), 0.0)",
        [],
        |row| row.get(0),
    )?;

    let stitch_cost = (stitch_count as f64 / 1000.0) * stitch_rate;

    // 3. Arbeitskosten: per-entry rate or default labor rate
    // Get default labor rate (first 'labor' rate or 25.0 fallback)
    let default_labor_rate: f64 = conn.query_row(
        "SELECT COALESCE((SELECT rate_value FROM cost_rates WHERE rate_type = 'labor' AND deleted_at IS NULL ORDER BY id LIMIT 1), 25.0)",
        [],
        |row| row.get(0),
    )?;

    // Calculate: entries with cost_rate_id use that rate, others use default
    let labor_cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM( \
             COALESCE(te.actual_minutes, 0) / 60.0 * \
             COALESCE(cr.rate_value, ?2) \
         ), 0) \
         FROM time_entries te \
         LEFT JOIN cost_rates cr ON cr.id = te.cost_rate_id AND cr.rate_type = 'labor' AND cr.deleted_at IS NULL \
         WHERE te.project_id = ?1 AND (te.machine IS NULL OR te.machine = '')",
        rusqlite::params![project_id, default_labor_rate],
        |row| row.get(0),
    )?;

    // 4. Maschinenkosten: entries with machine set, use machine rate + setup
    let default_machine_rate: f64 = conn.query_row(
        "SELECT COALESCE((SELECT rate_value FROM cost_rates WHERE rate_type = 'machine' AND deleted_at IS NULL ORDER BY id LIMIT 1), 0.0)",
        [],
        |row| row.get(0),
    )?;

    let machine_time_cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM( \
             COALESCE(te.actual_minutes, 0) / 60.0 * \
             COALESCE(cr.rate_value, ?2) \
         ), 0) \
         FROM time_entries te \
         LEFT JOIN cost_rates cr ON cr.id = te.cost_rate_id AND cr.rate_type = 'machine' AND cr.deleted_at IS NULL \
         WHERE te.project_id = ?1 AND te.machine IS NOT NULL AND te.machine != ''",
        rusqlite::params![project_id, default_machine_rate],
        |row| row.get(0),
    )?;

    // Setup costs: sum of setup_cost for unique machine rates used
    let machine_setup_cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM(cr.setup_cost), 0) \
         FROM cost_rates cr WHERE cr.id IN ( \
             SELECT DISTINCT te.cost_rate_id FROM time_entries te \
             WHERE te.project_id = ?1 AND te.machine IS NOT NULL AND te.machine != '' AND te.cost_rate_id IS NOT NULL \
         ) AND cr.rate_type = 'machine' AND cr.deleted_at IS NULL",
        [project_id],
        |row| row.get(0),
    )?;

    let machine_cost = machine_time_cost + machine_setup_cost;

    // 5. Beschaffungskosten: sum shipping_cost from purchase orders linked to project
    // Prefer direct project_id linkage; fall back to indirect BOM-material linkage
    let procurement_cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM(po.shipping_cost), 0) \
         FROM purchase_orders po WHERE po.deleted_at IS NULL AND ( \
             po.project_id = ?1 \
             OR (po.project_id IS NULL AND po.id IN ( \
                 SELECT DISTINCT oi.order_id FROM order_items oi \
                 WHERE oi.material_id IN ( \
                     SELECT b.material_id FROM bill_of_materials b \
                     WHERE b.product_id IN ( \
                         SELECT DISTINCT ps.product_id FROM product_steps ps \
                         JOIN workflow_steps ws ON ws.step_definition_id = ps.step_definition_id \
                         WHERE ws.project_id = ?1 \
                     ) \
                 ) \
             )) \
         )",
        [project_id],
        |row| row.get(0),
    )?;

    // 6. Herstellkosten = material + license + stitch + labor + machine + procurement
    let herstellkosten = material_cost + license_cost + stitch_cost + labor_cost + machine_cost + procurement_cost;

    // 7. Gemeinkosten: overhead percentage on Herstellkosten
    let overhead_pct: f64 = conn.query_row(
        "SELECT COALESCE((SELECT rate_value FROM cost_rates WHERE rate_type = 'overhead' AND deleted_at IS NULL ORDER BY id LIMIT 1), 0.0)",
        [],
        |row| row.get(0),
    )?;
    let overhead_cost = herstellkosten * (overhead_pct / 100.0);

    // 8. Selbstkosten = Herstellkosten + Gemeinkosten
    let selbstkosten = herstellkosten + overhead_cost;

    // 9. Gewinnzuschlag
    let profit_margin_pct: f64 = conn.query_row(
        "SELECT COALESCE((SELECT rate_value FROM cost_rates WHERE rate_type = 'profit' AND deleted_at IS NULL ORDER BY id LIMIT 1), 0.0)",
        [],
        |row| row.get(0),
    )?;
    let profit_amount = selbstkosten * (profit_margin_pct / 100.0);

    // 10. Netto-Verkaufspreis
    let netto_verkaufspreis = selbstkosten + profit_amount;

    // Per-piece
    let qty_f = quantity as f64;
    let selbstkosten_per_piece = selbstkosten / qty_f;
    let verkaufspreis_per_piece = netto_verkaufspreis / qty_f;

    Ok(CostBreakdown {
        project_id,
        project_name,
        quantity,
        material_cost,
        license_cost,
        stitch_cost,
        labor_cost,
        machine_cost,
        procurement_cost,
        herstellkosten,
        overhead_pct,
        overhead_cost,
        selbstkosten,
        profit_margin_pct,
        profit_amount,
        netto_verkaufspreis,
        selbstkosten_per_piece,
        verkaufspreis_per_piece,
    })
}

#[tauri::command]
pub fn get_cost_breakdown(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<CostBreakdown, AppError> {
    let conn = lock_db(&db)?;
    calculate_cost_breakdown(&conn, project_id)
}

#[tauri::command]
pub fn calculate_selling_price(
    db: State<'_, DbState>,
    project_id: i64,
    override_profit_pct: Option<f64>,
) -> Result<CostBreakdown, AppError> {
    let conn = lock_db(&db)?;
    let mut breakdown = calculate_cost_breakdown(&conn, project_id)?;

    // Allow overriding profit margin for what-if scenarios
    if let Some(pct) = override_profit_pct {
        breakdown.profit_margin_pct = pct;
        breakdown.profit_amount = breakdown.selbstkosten * (pct / 100.0);
        breakdown.netto_verkaufspreis = breakdown.selbstkosten + breakdown.profit_amount;
        let qty_f = breakdown.quantity as f64;
        breakdown.verkaufspreis_per_piece = breakdown.netto_verkaufspreis / qty_f;
    }

    Ok(breakdown)
}

#[tauri::command]
pub fn save_cost_breakdown(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<CostBreakdown, AppError> {
    let conn = lock_db(&db)?;
    let breakdown = calculate_cost_breakdown(&conn, project_id)?;

    // Clear previous snapshot
    conn.execute("DELETE FROM project_cost_items WHERE project_id = ?1", [project_id])?;

    // Persist each cost line
    let items = [
        ("material", "Materialkosten", breakdown.material_cost),
        ("license", "Lizenzkosten", breakdown.license_cost),
        ("stitch", "Stickkosten", breakdown.stitch_cost),
        ("labor", "Arbeitskosten", breakdown.labor_cost),
        ("machine", "Maschinenkosten", breakdown.machine_cost),
        ("procurement", "Beschaffungskosten", breakdown.procurement_cost),
        ("overhead", &format!("Gemeinkosten ({:.1}%)", breakdown.overhead_pct), breakdown.overhead_cost),
        ("profit", &format!("Gewinnzuschlag ({:.1}%)", breakdown.profit_margin_pct), breakdown.profit_amount),
    ];

    for (cost_type, description, amount) in &items {
        conn.execute(
            "INSERT INTO project_cost_items (project_id, cost_type, description, amount) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![project_id, cost_type, description, amount],
        )?;
    }

    Ok(breakdown)
}

// ── Project Report (extended) ────────────────────────────────────────

/// Generate an aggregated project report with optional cost breakdown.
#[tauri::command]
pub fn get_project_report(
    db: State<'_, DbState>,
    project_id: i64,
    labor_rate: Option<f64>,
) -> Result<ProjectReport, AppError> {
    let conn = lock_db(&db)?;

    // Project name
    let project_name: String = conn.query_row(
        "SELECT name FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id],
        |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
        _ => AppError::Database(e),
    })?;

    // Time totals
    let (total_planned, total_actual): (f64, f64) = conn.query_row(
        "SELECT COALESCE(SUM(planned_minutes), 0), COALESCE(SUM(actual_minutes), 0) FROM time_entries WHERE project_id = ?1",
        [project_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    // Material cost (simple BOM aggregation for backward compat)
    let material_cost: f64 = conn.query_row(
        "SELECT COALESCE(SUM(b.quantity * COALESCE(m.net_price, 0) * (1 + COALESCE(m.waste_factor, 0))), 0) \
         FROM bill_of_materials b \
         JOIN materials m ON m.id = b.material_id AND m.deleted_at IS NULL \
         WHERE b.product_id IN ( \
             SELECT DISTINCT ps.product_id FROM product_steps ps \
             JOIN workflow_steps ws ON ws.step_definition_id = ps.step_definition_id \
             WHERE ws.project_id = ?1 \
         )",
        [project_id],
        |row| row.get(0),
    )?;

    let rate = labor_rate.unwrap_or(25.0);
    let labor_cost = (total_actual / 60.0) * rate;
    let total_cost = material_cost + labor_cost;

    // Quality stats
    let (inspection_count, pass_count, fail_count): (i64, i64, i64) = conn.query_row(
        "SELECT COUNT(*), \
         SUM(CASE WHEN result = 'passed' THEN 1 ELSE 0 END), \
         SUM(CASE WHEN result = 'failed' THEN 1 ELSE 0 END) \
         FROM quality_inspections WHERE project_id = ?1",
        [project_id],
        |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(0), row.get::<_, Option<i64>>(2)?.unwrap_or(0))),
    )?;

    let open_defects: i64 = conn.query_row(
        "SELECT COUNT(*) FROM defect_records d \
         JOIN quality_inspections qi ON qi.id = d.inspection_id \
         WHERE qi.project_id = ?1 AND d.status = 'open'",
        [project_id],
        |row| row.get(0),
    )?;

    // Workflow progress
    let (workflow_total, workflow_completed): (i64, i64) = conn.query_row(
        "SELECT COUNT(*), SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) \
         FROM workflow_steps WHERE project_id = ?1",
        [project_id],
        |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(0))),
    )?;

    // Cost breakdown (optional — don't fail the report if cost calc fails)
    let cost_breakdown = calculate_cost_breakdown(&conn, project_id).ok();

    Ok(ProjectReport {
        project_id,
        project_name,
        total_planned_minutes: total_planned,
        total_actual_minutes: total_actual,
        material_cost,
        labor_cost,
        total_cost,
        inspection_count,
        pass_count,
        fail_count,
        open_defects,
        workflow_total,
        workflow_completed,
        cost_breakdown,
    })
}

/// Export project report as CSV string (now with cost breakdown).
#[tauri::command]
pub fn export_project_csv(
    db: State<'_, DbState>,
    project_id: i64,
    labor_rate: Option<f64>,
) -> Result<String, AppError> {
    let report = get_project_report(db, project_id, labor_rate)?;
    let mut csv = String::new();
    csv.push_str("Feld,Wert\n");
    csv.push_str(&format!("Projekt,\"{}\"\n", report.project_name.replace('"', "\"\"")));
    csv.push_str(&format!("Geplante Minuten,{:.1}\n", report.total_planned_minutes));
    csv.push_str(&format!("Tatsaechliche Minuten,{:.1}\n", report.total_actual_minutes));
    csv.push_str(&format!("Materialkosten,{:.2}\n", report.material_cost));
    csv.push_str(&format!("Arbeitskosten,{:.2}\n", report.labor_cost));
    csv.push_str(&format!("Gesamtkosten,{:.2}\n", report.total_cost));
    csv.push_str(&format!("Pruefungen,{}\n", report.inspection_count));
    csv.push_str(&format!("Bestanden,{}\n", report.pass_count));
    csv.push_str(&format!("Fehlgeschlagen,{}\n", report.fail_count));
    csv.push_str(&format!("Offene Fehler,{}\n", report.open_defects));
    csv.push_str(&format!("Workflow Schritte,{}\n", report.workflow_total));
    csv.push_str(&format!("Workflow Abgeschlossen,{}\n", report.workflow_completed));

    if let Some(cb) = &report.cost_breakdown {
        csv.push_str("\nKalkulation,\n");
        csv.push_str(&format!("Stueckzahl,{}\n", cb.quantity));
        csv.push_str(&format!("Materialkosten netto,{:.2}\n", cb.material_cost));
        csv.push_str(&format!("Lizenzkosten netto,{:.2}\n", cb.license_cost));
        csv.push_str(&format!("Stickkosten netto,{:.2}\n", cb.stitch_cost));
        csv.push_str(&format!("Arbeitskosten netto,{:.2}\n", cb.labor_cost));
        csv.push_str(&format!("Maschinenkosten netto,{:.2}\n", cb.machine_cost));
        csv.push_str(&format!("Beschaffungskosten netto,{:.2}\n", cb.procurement_cost));
        csv.push_str(&format!("Herstellkosten,{:.2}\n", cb.herstellkosten));
        csv.push_str(&format!("Gemeinkosten ({:.1}%),{:.2}\n", cb.overhead_pct, cb.overhead_cost));
        csv.push_str(&format!("Selbstkosten netto,{:.2}\n", cb.selbstkosten));
        csv.push_str(&format!("Gewinnzuschlag ({:.1}%),{:.2}\n", cb.profit_margin_pct, cb.profit_amount));
        csv.push_str(&format!("Netto-Verkaufspreis,{:.2}\n", cb.netto_verkaufspreis));
        if cb.quantity > 1 {
            csv.push_str(&format!("Selbstkosten pro Stueck,{:.2}\n", cb.selbstkosten_per_piece));
            csv.push_str(&format!("Verkaufspreis pro Stueck,{:.2}\n", cb.verkaufspreis_per_piece));
        }
    }

    Ok(csv)
}

// ── Additional Exports (project.md 9.6) ──────────────────────────────

/// Quote a string for CSV: wrap in double quotes, escape internal quotes.
fn csv_quote(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Export BOM for a product as CSV.
#[tauri::command]
pub fn export_bom_csv(
    db: State<'_, DbState>,
    product_id: i64,
) -> Result<String, AppError> {
    let conn = lock_db(&db)?;
    let product_name: String = conn.query_row(
        "SELECT name FROM products WHERE id = ?1 AND deleted_at IS NULL",
        [product_id], |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Produkt {product_id} nicht gefunden")),
        _ => AppError::Database(e),
    })?;

    let mut csv = String::new();
    csv.push_str(&format!("Stueckliste fuer: \"{}\"\n", product_name.replace('"', "\"\"")));
    csv.push_str("Material,Materialnummer,Menge,Einheit,Nettopreis,Verschnitt(%),Kosten\n");

    let mut stmt = conn.prepare(
        "SELECT m.name, COALESCE(m.material_number,''), b.quantity, COALESCE(b.unit, m.unit, ''), \
         COALESCE(m.net_price, 0), COALESCE(m.waste_factor, 0) \
         FROM bill_of_materials b \
         JOIN materials m ON m.id = b.material_id AND m.deleted_at IS NULL \
         WHERE b.product_id = ?1 ORDER BY m.name"
    )?;

    let rows: Vec<(String, String, f64, String, f64, f64)> = stmt.query_map([product_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
    })?.collect::<Result<Vec<_>, _>>()?;

    for (name, num, qty, unit, price, waste) in &rows {
        let cost = qty * price * (1.0 + waste);
        csv.push_str(&format!("\"{}\",\"{}\",{:.2},{},{:.2},{:.0},{:.2}\n",
            name.replace('"', "\"\""), num.replace('"', "\"\""), qty, unit, price, waste * 100.0, cost));
    }
    Ok(csv)
}

/// Export purchase orders as CSV, optionally filtered by project.
#[tauri::command]
pub fn export_orders_csv(
    db: State<'_, DbState>,
    project_id: Option<i64>,
) -> Result<String, AppError> {
    let conn = lock_db(&db)?;
    let mut csv = String::new();
    csv.push_str("Bestellnummer,Lieferant,Projekt,Status,Bestelldatum,Lieferdatum,Versandkosten,Material,Menge,Stueckpreis\n");

    let base_sql = "SELECT po.order_number, s.name, p.name, po.status, po.order_date, po.expected_delivery, po.shipping_cost, \
        m.name, oi.quantity_ordered, oi.unit_price \
        FROM purchase_orders po \
        JOIN suppliers s ON s.id = po.supplier_id \
        LEFT JOIN projects p ON p.id = po.project_id \
        LEFT JOIN order_items oi ON oi.order_id = po.id \
        LEFT JOIN materials m ON m.id = oi.material_id \
        WHERE po.deleted_at IS NULL";

    if let Some(pid) = project_id {
        let sql = format!("{base_sql} AND po.project_id = ?1 ORDER BY po.created_at DESC, oi.id");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([pid], |row| {
            Ok((row.get::<_,Option<String>>(0)?, row.get::<_,String>(1)?, row.get::<_,Option<String>>(2)?,
                row.get::<_,String>(3)?, row.get::<_,Option<String>>(4)?, row.get::<_,Option<String>>(5)?,
                row.get::<_,Option<f64>>(6)?, row.get::<_,Option<String>>(7)?,
                row.get::<_,Option<f64>>(8)?, row.get::<_,Option<f64>>(9)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        for r in &rows {
            csv.push_str(&format!("{},{},{},{},{},{},{:.2},{},{},{}\n",
                csv_quote(r.0.as_deref().unwrap_or("")), csv_quote(&r.1), csv_quote(r.2.as_deref().unwrap_or("")),
                r.3, r.4.as_deref().unwrap_or(""), r.5.as_deref().unwrap_or(""),
                r.6.unwrap_or(0.0), csv_quote(r.7.as_deref().unwrap_or("")),
                r.8.map(|v| format!("{:.2}", v)).unwrap_or_default(),
                r.9.map(|v| format!("{:.2}", v)).unwrap_or_default()));
        }
    } else {
        let sql = format!("{base_sql} ORDER BY po.created_at DESC, oi.id");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_,Option<String>>(0)?, row.get::<_,String>(1)?, row.get::<_,Option<String>>(2)?,
                row.get::<_,String>(3)?, row.get::<_,Option<String>>(4)?, row.get::<_,Option<String>>(5)?,
                row.get::<_,Option<f64>>(6)?, row.get::<_,Option<String>>(7)?,
                row.get::<_,Option<f64>>(8)?, row.get::<_,Option<f64>>(9)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        for r in &rows {
            csv.push_str(&format!("{},{},{},{},{},{},{:.2},{},{},{}\n",
                csv_quote(r.0.as_deref().unwrap_or("")), csv_quote(&r.1), csv_quote(r.2.as_deref().unwrap_or("")),
                r.3, r.4.as_deref().unwrap_or(""), r.5.as_deref().unwrap_or(""),
                r.6.unwrap_or(0.0), csv_quote(r.7.as_deref().unwrap_or("")),
                r.8.map(|v| format!("{:.2}", v)).unwrap_or_default(),
                r.9.map(|v| format!("{:.2}", v)).unwrap_or_default()));
        }
    }
    Ok(csv)
}

/// Export comprehensive project data as CSV.
#[tauri::command]
pub fn export_project_full_csv(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<String, AppError> {
    let conn = lock_db(&db)?;
    let project_name: String = conn.query_row(
        "SELECT name FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id], |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
        _ => AppError::Database(e),
    })?;

    let mut csv = String::new();
    csv.push_str(&format!("Projektexport: \"{}\"\n\n", project_name.replace('"', "\"\"")));

    // Time entries
    csv.push_str("Zeiterfassung\nSchritt,Geplant (min),Tatsaechlich (min),Mitarbeiter,Maschine\n");
    let mut stmt = conn.prepare(
        "SELECT step_name, COALESCE(planned_minutes,0), COALESCE(actual_minutes,0), \
         COALESCE(worker,''), COALESCE(machine,'') FROM time_entries WHERE project_id = ?1 ORDER BY recorded_at"
    )?;
    let time_rows: Vec<(String, f64, f64, String, String)> = stmt.query_map([project_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    })?.collect::<Result<Vec<_>, _>>()?;
    for r in &time_rows {
        csv.push_str(&format!("{},{:.1},{:.1},{},{}\n", csv_quote(&r.0), r.1, r.2, csv_quote(&r.3), csv_quote(&r.4)));
    }

    // Workflow steps
    csv.push_str("\nWorkflow\nSchritt,Status,Verantwortlich\n");
    let mut stmt = conn.prepare(
        "SELECT sd.name, ws.status, COALESCE(ws.responsible,'') \
         FROM workflow_steps ws JOIN step_definitions sd ON sd.id = ws.step_definition_id \
         WHERE ws.project_id = ?1 ORDER BY ws.sort_order"
    )?;
    let wf_rows: Vec<(String, String, String)> = stmt.query_map([project_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?.collect::<Result<Vec<_>, _>>()?;
    for r in &wf_rows {
        csv.push_str(&format!("{},{},{}\n", csv_quote(&r.0), r.1, csv_quote(&r.2)));
    }

    // Material consumption
    csv.push_str("\nMaterialverbrauch\nMaterial,Menge,Einheit,Schritt,Datum\n");
    let mut stmt = conn.prepare(
        "SELECT m.name, mc.quantity, COALESCE(mc.unit, m.unit, ''), COALESCE(mc.step_name,''), mc.recorded_at \
         FROM material_consumptions mc JOIN materials m ON m.id = mc.material_id \
         WHERE mc.project_id = ?1 ORDER BY mc.recorded_at"
    )?;
    let mc_rows: Vec<(String, f64, String, String, String)> = stmt.query_map([project_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    })?.collect::<Result<Vec<_>, _>>()?;
    for r in &mc_rows {
        csv.push_str(&format!("{},{:.2},{},{},{}\n", csv_quote(&r.0), r.1, r.2, csv_quote(&r.3), r.4));
    }

    // Quality inspections
    csv.push_str("\nQualitaetspruefungen\nDatum,Pruefer,Ergebnis,Notizen\n");
    let mut stmt = conn.prepare(
        "SELECT qi.inspection_date, COALESCE(qi.inspector,''), qi.result, COALESCE(qi.notes,'') \
         FROM quality_inspections qi WHERE qi.project_id = ?1 ORDER BY qi.inspection_date"
    )?;
    let qi_rows: Vec<(String, String, String, String)> = stmt.query_map([project_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?.collect::<Result<Vec<_>, _>>()?;
    for r in &qi_rows {
        csv.push_str(&format!("{},{},{},{}\n", r.0, csv_quote(&r.1), r.2, csv_quote(&r.3)));
    }

    // Cost breakdown (if available)
    if let Ok(cb) = calculate_cost_breakdown(&conn, project_id) {
        csv.push_str("\nKalkulation\n");
        csv.push_str(&format!("Materialkosten,{:.2}\n", cb.material_cost));
        csv.push_str(&format!("Lizenzkosten,{:.2}\n", cb.license_cost));
        csv.push_str(&format!("Stickkosten,{:.2}\n", cb.stitch_cost));
        csv.push_str(&format!("Arbeitskosten,{:.2}\n", cb.labor_cost));
        csv.push_str(&format!("Maschinenkosten,{:.2}\n", cb.machine_cost));
        csv.push_str(&format!("Beschaffungskosten,{:.2}\n", cb.procurement_cost));
        csv.push_str(&format!("Gemeinkosten,{:.2}\n", cb.overhead_cost));
        csv.push_str(&format!("Selbstkosten,{:.2}\n", cb.selbstkosten));
        csv.push_str(&format!("Gewinnzuschlag,{:.2}\n", cb.profit_amount));
        csv.push_str(&format!("Netto-Verkaufspreis,{:.2}\n", cb.netto_verkaufspreis));
    }

    Ok(csv)
}

/// Export material consumption per project as CSV (Nachkalkulation).
#[tauri::command]
pub fn export_material_usage_csv(
    db: State<'_, DbState>,
    project_id: i64,
) -> Result<String, AppError> {
    let conn = lock_db(&db)?;
    let project_name: String = conn.query_row(
        "SELECT name FROM projects WHERE id = ?1 AND deleted_at IS NULL",
        [project_id], |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Projekt {project_id} nicht gefunden")),
        _ => AppError::Database(e),
    })?;

    let quantity: i64 = conn.query_row(
        "SELECT COALESCE(quantity, 1) FROM projects WHERE id = ?1",
        [project_id], |row| row.get(0),
    )?;
    let qty = (quantity.max(1)) as f64;

    let mut csv = String::new();
    csv.push_str(&format!("Materialverbrauch: \"{}\"\n", project_name.replace('"', "\"\"")));
    csv.push_str("Material,Einheit,Soll-Menge,Ist-Menge,Differenz,Soll-Kosten,Ist-Kosten,Kosten-Differenz\n");

    let mut stmt = conn.prepare(
        "SELECT m.name, m.unit, \
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
             FROM material_consumptions WHERE project_id = ?1 GROUP BY material_id \
         ) actual ON actual.material_id = m.id \
         WHERE m.deleted_at IS NULL AND (planned.total_qty > 0 OR actual.total_qty > 0) \
         ORDER BY m.name"
    )?;

    let rows = stmt.query_map(rusqlite::params![project_id, qty], |row| {
        let name: String = row.get(0)?;
        let unit: Option<String> = row.get(1)?;
        let planned: f64 = row.get(2)?;
        let actual: f64 = row.get(3)?;
        let price: f64 = row.get(4)?;
        let waste: f64 = row.get(5)?;
        Ok((name, unit, planned, actual, price, waste))
    })?.collect::<Result<Vec<_>, _>>()?;

    for (name, unit, planned, actual, price, waste) in &rows {
        let diff = actual - planned;
        let planned_cost = planned * price * (1.0 + waste);
        let actual_cost = actual * price;
        let cost_diff = actual_cost - planned_cost;
        csv.push_str(&format!("\"{}\",{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}\n",
            name.replace('"', "\"\""), unit.as_deref().unwrap_or(""), planned, actual, diff,
            planned_cost, actual_cost, cost_diff));
    }

    Ok(csv)
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_quality_inspection_crud() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO projects (name, status) VALUES ('QTest', 'in_progress')", []).unwrap();
        let pid = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO quality_inspections (project_id, inspector, result) VALUES (?1, 'Anna', 'passed')",
            [pid],
        ).unwrap();
        let iid = conn.last_insert_rowid();

        let result: String = conn.query_row(
            "SELECT result FROM quality_inspections WHERE id = ?1", [iid], |r| r.get(0),
        ).unwrap();
        assert_eq!(result, "passed");

        // Cascade delete
        conn.execute("DELETE FROM projects WHERE id = ?1", [pid]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM quality_inspections", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_defect_record_crud() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO projects (name, status) VALUES ('DTest', 'in_progress')", []).unwrap();
        let pid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO quality_inspections (project_id, result) VALUES (?1, 'failed')", [pid],
        ).unwrap();
        let iid = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO defect_records (inspection_id, description, severity) VALUES (?1, 'Faden locker', 'minor')",
            [iid],
        ).unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM defect_records WHERE inspection_id = ?1", [iid], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 1);

        // Cascade via inspection
        conn.execute("DELETE FROM quality_inspections WHERE id = ?1", [iid]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM defect_records", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_report_aggregation() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO projects (name, status) VALUES ('Report', 'in_progress')", []).unwrap();
        let pid = conn.last_insert_rowid();

        // Time entries
        conn.execute(
            "INSERT INTO time_entries (project_id, step_name, planned_minutes, actual_minutes) VALUES (?1, 'Sticken', 60.0, 75.0)",
            [pid],
        ).unwrap();

        // Verify time aggregation
        let (planned, actual): (f64, f64) = conn.query_row(
            "SELECT COALESCE(SUM(planned_minutes), 0), COALESCE(SUM(actual_minutes), 0) FROM time_entries WHERE project_id = ?1",
            [pid], |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(planned, 60.0);
        assert_eq!(actual, 75.0);
    }

    #[test]
    fn test_cost_breakdown_kosmetiktasche() {
        // Reproduce the project.md section 7.4 example: Bestickte Kosmetiktasche
        let conn = init_database_in_memory().unwrap();

        // Create project with quantity=1
        conn.execute("INSERT INTO projects (name, status, quantity) VALUES ('Kosmetiktasche', 'in_progress', 1)", []).unwrap();
        let pid = conn.last_insert_rowid();

        // Create cost rates
        // Labor rate: 36 EUR/h
        conn.execute(
            "INSERT INTO cost_rates (rate_type, name, rate_value, unit) VALUES ('labor', 'Standard', 36.0, 'EUR/h')",
            [],
        ).unwrap();
        let labor_rate_id = conn.last_insert_rowid();

        // Machine rate: 12 EUR/h
        conn.execute(
            "INSERT INTO cost_rates (rate_type, name, rate_value, unit, setup_cost) VALUES ('machine', 'Stickmaschine', 12.0, 'EUR/h', 0)",
            [],
        ).unwrap();
        let machine_rate_id = conn.last_insert_rowid();

        // Overhead: 15%
        conn.execute(
            "INSERT INTO cost_rates (rate_type, name, rate_value, unit) VALUES ('overhead', 'Gemeinkosten', 15.0, '%')",
            [],
        ).unwrap();

        // Profit: 25%
        conn.execute(
            "INSERT INTO cost_rates (rate_type, name, rate_value, unit) VALUES ('profit', 'Gewinn', 25.0, '%')",
            [],
        ).unwrap();

        // Create product and materials (11 EUR total, 7% waste)
        conn.execute(
            "INSERT INTO products (name, status) VALUES ('Kosmetiktasche', 'active')",
            [],
        ).unwrap();
        let product_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO materials (name, net_price, waste_factor) VALUES ('Stoff + Zubeh\u{f6}r', 11.0, 0.07)",
            [],
        ).unwrap();
        let mat_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO bill_of_materials (product_id, material_id, quantity) VALUES (?1, ?2, 1.0)",
            rusqlite::params![product_id, mat_id],
        ).unwrap();

        // Create step definition + product_step + workflow_step to link product to project
        conn.execute(
            "INSERT INTO step_definitions (name) VALUES ('Fertigung')",
            [],
        ).unwrap();
        let step_def_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO product_steps (product_id, step_definition_id) VALUES (?1, ?2)",
            rusqlite::params![product_id, step_def_id],
        ).unwrap();

        conn.execute(
            "INSERT INTO workflow_steps (project_id, step_definition_id, status) VALUES (?1, ?2, 'pending')",
            rusqlite::params![pid, step_def_id],
        ).unwrap();

        // License: 1.20 EUR per piece
        conn.execute(
            "INSERT INTO license_records (name, cost_per_piece) VALUES ('Design-Lizenz', 1.20)",
            [],
        ).unwrap();
        let lic_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO project_license_links (project_id, license_id) VALUES (?1, ?2)",
            rusqlite::params![pid, lic_id],
        ).unwrap();

        // Time entries: 42 min labor (no machine), 15 min machine
        conn.execute(
            "INSERT INTO time_entries (project_id, step_name, actual_minutes, cost_rate_id) VALUES (?1, 'Naehen', 42.0, ?2)",
            rusqlite::params![pid, labor_rate_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO time_entries (project_id, step_name, actual_minutes, machine, cost_rate_id) VALUES (?1, 'Sticken', 15.0, 'Brother', ?2)",
            rusqlite::params![pid, machine_rate_id],
        ).unwrap();

        // Procurement cost: 0.80 EUR shipping
        conn.execute(
            "INSERT INTO suppliers (name) VALUES ('Stoffe GmbH')",
            [],
        ).unwrap();
        let sup_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO purchase_orders (supplier_id, status, shipping_cost) VALUES (?1, 'delivered', 0.80)",
            [sup_id],
        ).unwrap();
        let po_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO order_items (order_id, material_id, quantity_ordered) VALUES (?1, ?2, 1.0)",
            rusqlite::params![po_id, mat_id],
        ).unwrap();

        // Calculate
        let breakdown = super::calculate_cost_breakdown(&conn, pid).unwrap();

        // Verify per project.md 7.4
        // Material: 11.0 * 1.07 = 11.77
        assert!((breakdown.material_cost - 11.77).abs() < 0.01, "material_cost: {}", breakdown.material_cost);
        // License: 1.20
        assert!((breakdown.license_cost - 1.20).abs() < 0.01, "license_cost: {}", breakdown.license_cost);
        // Stitch cost: 0.0 (no pattern_file_id)
        assert_eq!(breakdown.stitch_cost, 0.0);
        // Labor: 42/60 * 36 = 25.20
        assert!((breakdown.labor_cost - 25.20).abs() < 0.01, "labor_cost: {}", breakdown.labor_cost);
        // Machine: 15/60 * 12 = 3.00
        assert!((breakdown.machine_cost - 3.00).abs() < 0.01, "machine_cost: {}", breakdown.machine_cost);
        // Procurement: 0.80
        assert!((breakdown.procurement_cost - 0.80).abs() < 0.01, "procurement_cost: {}", breakdown.procurement_cost);
        // Herstellkosten: 11.77 + 1.20 + 25.20 + 3.00 + 0.80 = 41.97
        assert!((breakdown.herstellkosten - 41.97).abs() < 0.01, "herstellkosten: {}", breakdown.herstellkosten);
        // Overhead: 41.97 * 0.15 = 6.2955 ≈ 6.30
        assert!((breakdown.overhead_cost - 6.30).abs() < 0.02, "overhead_cost: {}", breakdown.overhead_cost);
        // Selbstkosten: 41.97 + 6.30 ≈ 48.27
        assert!((breakdown.selbstkosten - 48.27).abs() < 0.02, "selbstkosten: {}", breakdown.selbstkosten);
        // Profit: 48.27 * 0.25 ≈ 12.07
        assert!((breakdown.profit_amount - 12.07).abs() < 0.02, "profit_amount: {}", breakdown.profit_amount);
        // Verkaufspreis: 48.27 + 12.07 ≈ 60.34
        assert!((breakdown.netto_verkaufspreis - 60.34).abs() < 0.03, "netto_verkaufspreis: {}", breakdown.netto_verkaufspreis);
    }

    #[test]
    fn test_cost_breakdown_empty_project() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO projects (name, status) VALUES ('Empty', 'not_started')", []).unwrap();
        let pid = conn.last_insert_rowid();

        let breakdown = super::calculate_cost_breakdown(&conn, pid).unwrap();
        assert_eq!(breakdown.material_cost, 0.0);
        assert_eq!(breakdown.license_cost, 0.0);
        assert_eq!(breakdown.stitch_cost, 0.0);
        assert_eq!(breakdown.labor_cost, 0.0);
        assert_eq!(breakdown.machine_cost, 0.0);
        assert_eq!(breakdown.procurement_cost, 0.0);
        assert_eq!(breakdown.selbstkosten, 0.0);
        assert_eq!(breakdown.netto_verkaufspreis, 0.0);
    }

    #[test]
    fn test_cost_breakdown_with_stitch_cost() {
        let conn = init_database_in_memory().unwrap();

        // Create folder + embroidery file with stitch_count = 15000
        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('TestFolder', '/test')",
            [],
        ).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, file_type, status, stitch_count) \
             VALUES (?1, 'test.pes', '/test/test.pes', 'embroidery', 'active', 15000)",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        // Create project with pattern_file_id pointing to that file
        conn.execute(
            "INSERT INTO projects (name, status, pattern_file_id, quantity) VALUES ('StitchTest', 'in_progress', ?1, 1)",
            [file_id],
        ).unwrap();
        let pid = conn.last_insert_rowid();

        // Create stitch rate: 5.0 EUR/1000 Stiche
        conn.execute(
            "INSERT INTO cost_rates (rate_type, name, rate_value, unit) VALUES ('stitch', 'Standard', 5.0, 'EUR/1000 Stiche')",
            [],
        ).unwrap();

        let breakdown = super::calculate_cost_breakdown(&conn, pid).unwrap();

        // stitch_cost = 15000 / 1000 * 5.0 = 75.0
        assert!((breakdown.stitch_cost - 75.0).abs() < 0.01, "stitch_cost: {}", breakdown.stitch_cost);
        // herstellkosten should include stitch_cost
        assert!((breakdown.herstellkosten - 75.0).abs() < 0.01, "herstellkosten: {}", breakdown.herstellkosten);
    }

    #[test]
    fn test_cost_breakdown_no_stitch_rate() {
        let conn = init_database_in_memory().unwrap();

        // Create folder + file with stitch_count
        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('TestFolder', '/test2')",
            [],
        ).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, file_type, status, stitch_count) \
             VALUES (?1, 'test2.pes', '/test2/test2.pes', 'embroidery', 'active', 10000)",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        // Create project with pattern_file_id but no stitch rate defined
        conn.execute(
            "INSERT INTO projects (name, status, pattern_file_id) VALUES ('NoRateTest', 'in_progress', ?1)",
            [file_id],
        ).unwrap();
        let pid = conn.last_insert_rowid();

        let breakdown = super::calculate_cost_breakdown(&conn, pid).unwrap();
        assert_eq!(breakdown.stitch_cost, 0.0);
    }

    #[test]
    fn test_cost_rate_crud() {
        let conn = init_database_in_memory().unwrap();
        conn.execute(
            "INSERT INTO cost_rates (rate_type, name, rate_value, unit) VALUES ('labor', 'Test', 30.0, 'EUR/h')",
            [],
        ).unwrap();
        let id = conn.last_insert_rowid();

        let rate_value: f64 = conn.query_row(
            "SELECT rate_value FROM cost_rates WHERE id = ?1", [id], |r| r.get(0),
        ).unwrap();
        assert_eq!(rate_value, 30.0);

        // Soft delete
        conn.execute("UPDATE cost_rates SET deleted_at = datetime('now') WHERE id = ?1", [id]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM cost_rates WHERE deleted_at IS NULL", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }
}
