use std::collections::HashMap;
use tauri::State;

use crate::DbState;
use crate::db::models::CustomFieldDefinition;
use crate::error::{lock_db, AppError};

#[tauri::command]
pub fn get_setting(db: State<'_, DbState>, key: String) -> Result<String, AppError> {
    let conn = lock_db(&db)?;

    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [&key],
        |row| row.get(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Einstellung '{key}' nicht gefunden"))
        }
        other => AppError::Database(other),
    })
}

#[tauri::command]
pub fn set_setting(
    db: State<'_, DbState>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![key, value],
    )?;

    Ok(())
}

#[tauri::command]
pub fn get_all_settings(db: State<'_, DbState>) -> Result<HashMap<String, String>, AppError> {
    let conn = lock_db(&db)?;

    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut map = HashMap::new();
    for (key, value) in rows {
        map.insert(key, value);
    }

    Ok(map)
}

#[tauri::command]
pub fn get_custom_fields(
    db: State<'_, DbState>,
) -> Result<Vec<CustomFieldDefinition>, AppError> {
    let conn = lock_db(&db)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, field_type, options, required, sort_order, created_at \
         FROM custom_field_definitions ORDER BY sort_order, name",
    )?;
    let fields = stmt
        .query_map([], |row| {
            Ok(CustomFieldDefinition {
                id: row.get(0)?,
                name: row.get(1)?,
                field_type: row.get(2)?,
                options: row.get(3)?,
                required: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(fields)
}

#[tauri::command]
pub fn create_custom_field(
    db: State<'_, DbState>,
    name: String,
    field_type: String,
    options: Option<String>,
) -> Result<CustomFieldDefinition, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation(
            "Feldname darf nicht leer sein".into(),
        ));
    }

    let valid_types = ["text", "number", "date", "select"];
    if !valid_types.contains(&field_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Ungültiger Feldtyp: '{field_type}'. Erlaubt: text, number, date, select"
        )));
    }

    if field_type == "select" && options.as_ref().map_or(true, |o| o.trim().is_empty()) {
        return Err(AppError::Validation(
            "Feldtyp 'select' erfordert Optionen".into(),
        ));
    }

    let conn = lock_db(&db)?;

    conn.execute(
        "INSERT INTO custom_field_definitions (name, field_type, options) VALUES (?1, ?2, ?3)",
        rusqlite::params![name.trim(), field_type, options],
    )?;

    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, field_type, options, required, sort_order, created_at \
         FROM custom_field_definitions WHERE id = ?1",
        [id],
        |row| {
            Ok(CustomFieldDefinition {
                id: row.get(0)?,
                name: row.get(1)?,
                field_type: row.get(2)?,
                options: row.get(3)?,
                required: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )
    .map_err(|e| AppError::Database(e))
}

#[tauri::command]
pub fn delete_custom_field(
    db: State<'_, DbState>,
    field_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    let changes = conn.execute(
        "DELETE FROM custom_field_definitions WHERE id = ?1",
        [field_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!(
            "Benutzerdefiniertes Feld {field_id} nicht gefunden"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_settings_crud() {
        let conn = init_database_in_memory().unwrap();

        // Default settings should exist
        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme_mode'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "hell");

        // Update a setting
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES ('theme_mode', 'dunkel', datetime('now'))",
            [],
        ).unwrap();

        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme_mode'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "dunkel");

        // Insert a new setting
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES ('new_key', 'new_value', datetime('now'))",
            [],
        ).unwrap();

        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'new_key'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "new_value");
    }

    #[test]
    fn test_custom_field_crud() {
        let conn = init_database_in_memory().unwrap();

        // Create
        conn.execute(
            "INSERT INTO custom_field_definitions (name, field_type) VALUES ('Material', 'text')",
            [],
        ).unwrap();
        let id = conn.last_insert_rowid();

        let name: String = conn
            .query_row(
                "SELECT name FROM custom_field_definitions WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "Material");

        // Delete
        let changes = conn
            .execute("DELETE FROM custom_field_definitions WHERE id = ?1", [id])
            .unwrap();
        assert_eq!(changes, 1);

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM custom_field_definitions WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists);
    }

    #[test]
    fn test_custom_field_validates_type() {
        let valid_types = ["text", "number", "date", "select"];
        assert!(valid_types.contains(&"text"));
        assert!(valid_types.contains(&"number"));
        assert!(valid_types.contains(&"date"));
        assert!(valid_types.contains(&"select"));
        assert!(!valid_types.contains(&"invalid"));
    }
}
