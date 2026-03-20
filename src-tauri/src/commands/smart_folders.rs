use tauri::State;
use crate::DbState;
use crate::db::models::SmartFolder;
use crate::error::{lock_db, AppError};

fn row_to_smart_folder(row: &rusqlite::Row) -> rusqlite::Result<SmartFolder> {
    Ok(SmartFolder {
        id: row.get(0)?,
        name: row.get(1)?,
        icon: row.get(2)?,
        filter_json: row.get(3)?,
        sort_order: row.get(4)?,
        created_at: row.get(5)?,
    })
}

const SF_SELECT: &str =
    "SELECT id, name, icon, filter_json, sort_order, created_at FROM smart_folders";

#[tauri::command]
pub fn get_smart_folders(db: State<'_, DbState>) -> Result<Vec<SmartFolder>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(&format!("{SF_SELECT} ORDER BY sort_order, name"))?;
    let folders = stmt
        .query_map([], |row| row_to_smart_folder(row))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(folders)
}

#[tauri::command]
pub fn create_smart_folder(
    db: State<'_, DbState>,
    name: String,
    icon: Option<String>,
    filter_json: String,
) -> Result<SmartFolder, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("Name darf nicht leer sein".into()));
    }

    // Validate filter_json is valid JSON
    if serde_json::from_str::<serde_json::Value>(&filter_json).is_err() {
        return Err(AppError::Validation("Ungueltiges JSON-Filter".into()));
    }

    let conn = lock_db(&db)?;

    let max_order: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM smart_folders",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let ic = icon.as_deref().unwrap_or("\u{1F50D}");

    conn.execute(
        "INSERT INTO smart_folders (name, icon, filter_json, sort_order) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![name.trim(), ic, filter_json, max_order + 10],
    )?;

    let id = conn.last_insert_rowid();
    let sf = conn.query_row(
        &format!("{SF_SELECT} WHERE id = ?1"),
        [id],
        |row| row_to_smart_folder(row),
    )?;

    Ok(sf)
}

#[tauri::command]
pub fn update_smart_folder(
    db: State<'_, DbState>,
    id: i64,
    name: Option<String>,
    icon: Option<String>,
    filter_json: Option<String>,
) -> Result<SmartFolder, AppError> {
    if name.is_none() && icon.is_none() && filter_json.is_none() {
        return Err(AppError::Validation(
            "Mindestens ein Feld muss aktualisiert werden".into(),
        ));
    }

    if let Some(ref fj) = filter_json {
        if serde_json::from_str::<serde_json::Value>(fj).is_err() {
            return Err(AppError::Validation("Ungueltiges JSON-Filter".into()));
        }
    }

    let conn = lock_db(&db)?;

    if let Some(ref n) = name {
        if n.trim().is_empty() {
            return Err(AppError::Validation("Name darf nicht leer sein".into()));
        }
    }

    let tx = conn.unchecked_transaction()?;
    if let Some(ref n) = name {
        tx.execute(
            "UPDATE smart_folders SET name = ?1 WHERE id = ?2",
            rusqlite::params![n.trim(), id],
        )?;
    }
    if let Some(ref ic) = icon {
        tx.execute(
            "UPDATE smart_folders SET icon = ?1 WHERE id = ?2",
            rusqlite::params![ic, id],
        )?;
    }
    if let Some(ref fj) = filter_json {
        tx.execute(
            "UPDATE smart_folders SET filter_json = ?1 WHERE id = ?2",
            rusqlite::params![fj, id],
        )?;
    }
    tx.commit()?;

    let sf = conn
        .query_row(
            &format!("{SF_SELECT} WHERE id = ?1"),
            [id],
            |row| row_to_smart_folder(row),
        )
        .map_err(|_| AppError::NotFound(format!("Intelligenter Ordner {id} nicht gefunden")))?;

    Ok(sf)
}

#[tauri::command]
pub fn delete_smart_folder(db: State<'_, DbState>, id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM smart_folders WHERE id = ?1", [id])?;
    if changes == 0 {
        return Err(AppError::NotFound(format!(
            "Intelligenter Ordner {id} nicht gefunden"
        )));
    }
    Ok(())
}

#[tauri::command]
pub fn update_smart_folder_sort_orders(
    db: State<'_, DbState>,
    orders: Vec<(i64, i32)>,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let tx = conn.unchecked_transaction()?;
    for (id, order) in &orders {
        tx.execute(
            "UPDATE smart_folders SET sort_order = ?1 WHERE id = ?2",
            rusqlite::params![order, id],
        )?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_smart_folder_crud() {
        let conn = init_database_in_memory().unwrap();

        // Presets should exist from migration
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM smart_folders", [], |row| row.get(0))
            .unwrap();
        assert!(count >= 3, "Should have at least 3 preset smart folders");

        // Create
        conn.execute(
            "INSERT INTO smart_folders (name, icon, filter_json, sort_order) VALUES ('Test', '📁', '{\"text\": \"hello\"}', 100)",
            [],
        ).unwrap();
        let id = conn.last_insert_rowid();

        // Read
        let name: String = conn
            .query_row("SELECT name FROM smart_folders WHERE id = ?1", [id], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "Test");

        // Update
        conn.execute("UPDATE smart_folders SET name = 'Updated' WHERE id = ?1", [id]).unwrap();
        let name: String = conn
            .query_row("SELECT name FROM smart_folders WHERE id = ?1", [id], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "Updated");

        // Delete
        conn.execute("DELETE FROM smart_folders WHERE id = ?1", [id]).unwrap();
        let exists: bool = conn
            .query_row("SELECT COUNT(*) > 0 FROM smart_folders WHERE id = ?1", [id], |row| row.get(0))
            .unwrap();
        assert!(!exists);
    }
}
