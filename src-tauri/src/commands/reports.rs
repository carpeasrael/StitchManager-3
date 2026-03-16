use tauri::State;

use crate::db::models::ProjectReport;
use crate::error::{lock_db, AppError};
use crate::DbState;

/// Generate an aggregated project report.
#[tauri::command]
pub fn get_project_report(
    db: State<'_, DbState>,
    project_id: i64,
    labor_rate: Option<f64>,
) -> Result<ProjectReport, AppError> {
    let rate = labor_rate.unwrap_or(25.0);
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

    // Material cost scoped to project: aggregate BOM costs for products whose steps
    // are assigned to this project's workflow. Falls back to 0 if no workflow exists.
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
    })
}

/// Export project report as CSV string.
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
}
