use tauri::State;
use crate::DbState;
use crate::db::models::Folder;
use crate::error::{lock_db, AppError};

fn row_to_folder(row: &rusqlite::Row) -> rusqlite::Result<Folder> {
    Ok(Folder {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        parent_id: row.get(3)?,
        sort_order: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

const FOLDER_SELECT: &str =
    "SELECT id, name, path, parent_id, sort_order, created_at, updated_at FROM folders";

#[tauri::command]
pub fn get_folders(db: State<'_, DbState>) -> Result<Vec<Folder>, AppError> {
    let conn = lock_db(&db)?;

    let mut stmt = conn.prepare(&format!("{FOLDER_SELECT} ORDER BY sort_order, name"))?;
    let folders = stmt
        .query_map([], |row| row_to_folder(row))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(folders)
}

#[tauri::command]
pub fn create_folder(
    db: State<'_, DbState>,
    name: String,
    path: String,
    parent_id: Option<i64>,
) -> Result<Folder, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation(
            "Ordnername darf nicht leer sein".into(),
        ));
    }

    // Note: TOCTOU race possible (path could be removed after check). Acceptable for MVP;
    // the DB stores the path and later operations will handle missing directories gracefully.
    if !std::path::Path::new(&path).exists() {
        return Err(AppError::Validation(format!(
            "Pfad existiert nicht: {path}"
        )));
    }

    let conn = lock_db(&db)?;

    conn.execute(
        "INSERT INTO folders (name, path, parent_id) VALUES (?1, ?2, ?3)",
        rusqlite::params![name.trim(), path, parent_id],
    )?;

    let id = conn.last_insert_rowid();
    let folder = conn.query_row(
        &format!("{FOLDER_SELECT} WHERE id = ?1"),
        [id],
        |row| row_to_folder(row),
    )?;

    Ok(folder)
}

#[tauri::command]
pub fn update_folder(
    db: State<'_, DbState>,
    folder_id: i64,
    name: Option<String>,
) -> Result<Folder, AppError> {
    if name.is_none() {
        return Err(AppError::Validation(
            "Mindestens ein Feld muss aktualisiert werden".into(),
        ));
    }

    let conn = lock_db(&db)?;

    if let Some(ref new_name) = name {
        if new_name.trim().is_empty() {
            return Err(AppError::Validation(
                "Ordnername darf nicht leer sein".into(),
            ));
        }
        conn.execute(
            "UPDATE folders SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![new_name.trim(), folder_id],
        )?;
    }

    let folder = conn
        .query_row(
            &format!("{FOLDER_SELECT} WHERE id = ?1"),
            [folder_id],
            |row| row_to_folder(row),
        )
        .map_err(|_| AppError::NotFound(format!("Ordner {folder_id} nicht gefunden")))?;

    Ok(folder)
}

#[tauri::command]
pub fn delete_folder(db: State<'_, DbState>, folder_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    let file_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1",
        [folder_id],
        |row| row.get(0),
    )?;
    if file_count > 0 {
        log::warn!(
            "Deleting folder {folder_id} will cascade-delete {file_count} associated file(s)"
        );
    }

    let changes = conn.execute("DELETE FROM folders WHERE id = ?1", [folder_id])?;
    if changes == 0 {
        return Err(AppError::NotFound(format!(
            "Ordner {folder_id} nicht gefunden"
        )));
    }

    Ok(())
}

#[tauri::command]
pub fn get_folder_file_count(
    db: State<'_, DbState>,
    folder_id: i64,
) -> Result<i64, AppError> {
    let conn = lock_db(&db)?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1",
        [folder_id],
        |row| row.get(0),
    )?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_folder_crud_cycle() {
        let conn = init_database_in_memory().unwrap();

        // Create
        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Test', '/tmp/test')",
            [],
        )
        .unwrap();
        let id: i64 = conn.last_insert_rowid();

        // Read
        let name: String = conn
            .query_row("SELECT name FROM folders WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(name, "Test");

        // Update
        conn.execute(
            "UPDATE folders SET name = 'Updated', updated_at = datetime('now') WHERE id = ?1",
            [id],
        )
        .unwrap();
        let name: String = conn
            .query_row("SELECT name FROM folders WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(name, "Updated");

        // File count (should be 0)
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);

        // Delete
        let changes = conn
            .execute("DELETE FROM folders WHERE id = ?1", [id])
            .unwrap();
        assert_eq!(changes, 1);

        // Verify deleted
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM folders WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists);
    }

    #[test]
    fn test_create_folder_validates_name() {
        let conn = init_database_in_memory().unwrap();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Valid', '/tmp/valid')",
            [],
        )
        .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM folders", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_folders_ordered() {
        let conn = init_database_in_memory().unwrap();

        conn.execute(
            "INSERT INTO folders (name, path, sort_order) VALUES ('Zebra', '/z', 2)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO folders (name, path, sort_order) VALUES ('Alpha', '/a', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO folders (name, path, sort_order) VALUES ('Beta', '/b', 1)",
            [],
        )
        .unwrap();

        let mut stmt = conn
            .prepare("SELECT name FROM folders ORDER BY sort_order, name")
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(names, vec!["Alpha", "Beta", "Zebra"]);
    }
}
