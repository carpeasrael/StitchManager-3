use rusqlite::Connection;
use tauri::State;

use crate::db::models::AuditLogEntry;
use crate::error::{lock_db, AppError};
use crate::DbState;

/// Log a single field change to the audit_log table.
pub fn log_change(
    conn: &Connection,
    entity_type: &str,
    entity_id: i64,
    field_name: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
) -> Result<(), AppError> {
    // Only log if value actually changed
    if old_value == new_value {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO audit_log (entity_type, entity_id, field_name, old_value, new_value) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![entity_type, entity_id, field_name, old_value, new_value],
    )?;
    Ok(())
}

fn row_to_audit(row: &rusqlite::Row) -> rusqlite::Result<AuditLogEntry> {
    Ok(AuditLogEntry {
        id: row.get(0)?,
        entity_type: row.get(1)?,
        entity_id: row.get(2)?,
        field_name: row.get(3)?,
        old_value: row.get(4)?,
        new_value: row.get(5)?,
        changed_by: row.get(6)?,
        changed_at: row.get(7)?,
    })
}

#[tauri::command]
pub fn get_audit_log(
    db: State<'_, DbState>,
    entity_type: String,
    entity_id: i64,
) -> Result<Vec<AuditLogEntry>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, entity_type, entity_id, field_name, old_value, new_value, changed_by, changed_at \
         FROM audit_log WHERE entity_type = ?1 AND entity_id = ?2 ORDER BY changed_at DESC, id DESC"
    )?;
    let entries = stmt.query_map(rusqlite::params![entity_type, entity_id], row_to_audit)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}
