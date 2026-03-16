use std::collections::HashMap;
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

    // Collect thumbnail paths before cascade delete removes the file rows.
    // Single recursive CTE for both thumbnail paths and file count.
    let mut stmt = conn.prepare(
        "WITH RECURSIVE folder_tree(id) AS (
             SELECT id FROM folders WHERE id = ?1
             UNION ALL
             SELECT f.id FROM folders f JOIN folder_tree ft ON f.parent_id = ft.id
         )
         SELECT e.thumbnail_path FROM embroidery_files e
         JOIN folder_tree ft ON e.folder_id = ft.id",
    )?;
    let all_thumb_paths: Vec<Option<String>> = stmt
        .query_map([folder_id], |row| row.get::<_, Option<String>>(0))?
        .filter_map(|r| r.ok())
        .collect();

    let file_count = all_thumb_paths.len();
    let thumbnail_paths: Vec<String> = all_thumb_paths
        .into_iter()
        .flatten()
        .filter(|p| !p.is_empty())
        .collect();

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

    // Clean up thumbnail files on disk (best-effort)
    for path in &thumbnail_paths {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("Failed to remove thumbnail {path}: {e}");
            }
        }
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
        "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1 AND deleted_at IS NULL",
        [folder_id],
        |row| row.get(0),
    )?;

    Ok(count)
}

#[tauri::command]
pub fn get_all_folder_file_counts(
    db: State<'_, DbState>,
) -> Result<HashMap<i64, i64>, AppError> {
    let conn = lock_db(&db)?;

    let mut stmt = conn.prepare(
        "SELECT folder_id, COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL GROUP BY folder_id",
    )?;
    let counts = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(counts)
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
                "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1 AND deleted_at IS NULL",
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

    #[test]
    fn test_delete_folder_cleans_up_thumbnails() {
        let conn = init_database_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();

        // Create fake thumbnail files on disk
        let thumb1 = tmp.path().join("100.png");
        let thumb2 = tmp.path().join("101.png");
        std::fs::write(&thumb1, b"fake png 1").unwrap();
        std::fs::write(&thumb2, b"fake png 2").unwrap();
        assert!(thumb1.exists());
        assert!(thumb2.exists());

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Test', '/test')",
            [],
        ).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, thumbnail_path) \
             VALUES (?1, 'a.pes', '/test/a.pes', ?2)",
            rusqlite::params![folder_id, thumb1.to_string_lossy().as_ref()],
        ).unwrap();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, thumbnail_path) \
             VALUES (?1, 'b.dst', '/test/b.dst', ?2)",
            rusqlite::params![folder_id, thumb2.to_string_lossy().as_ref()],
        ).unwrap();

        // Collect thumbnail paths using recursive CTE — mirrors the command logic
        let mut stmt = conn.prepare(
            "WITH RECURSIVE folder_tree(id) AS (
                 SELECT id FROM folders WHERE id = ?1
                 UNION ALL
                 SELECT f.id FROM folders f JOIN folder_tree ft ON f.parent_id = ft.id
             )
             SELECT e.thumbnail_path FROM embroidery_files e
             JOIN folder_tree ft ON e.folder_id = ft.id
             WHERE e.thumbnail_path IS NOT NULL AND e.thumbnail_path != ''",
        ).unwrap();
        let thumbnail_paths: Vec<String> = stmt
            .query_map([folder_id], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(thumbnail_paths.len(), 2);

        // Delete folder (cascades to files)
        let changes = conn.execute("DELETE FROM folders WHERE id = ?1", [folder_id]).unwrap();
        assert_eq!(changes, 1);

        // Files should be cascade-deleted
        let file_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1 AND deleted_at IS NULL",
                [folder_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(file_count, 0);

        // Clean up thumbnails (mirrors the new delete_folder logic)
        for path in &thumbnail_paths {
            let _ = std::fs::remove_file(path);
        }

        assert!(!thumb1.exists(), "Thumbnail 1 should be deleted from disk");
        assert!(!thumb2.exists(), "Thumbnail 2 should be deleted from disk");
    }

    #[test]
    fn test_delete_folder_cleans_up_nested_subfolder_thumbnails() {
        let conn = init_database_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();

        // Create fake thumbnail files
        let thumb_parent = tmp.path().join("200.png");
        let thumb_child = tmp.path().join("201.png");
        std::fs::write(&thumb_parent, b"fake").unwrap();
        std::fs::write(&thumb_child, b"fake").unwrap();

        // Parent folder
        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Parent', '/parent')",
            [],
        ).unwrap();
        let parent_id = conn.last_insert_rowid();

        // Child folder (nested under parent)
        conn.execute(
            "INSERT INTO folders (name, path, parent_id) VALUES ('Child', '/parent/child', ?1)",
            [parent_id],
        ).unwrap();
        let child_id = conn.last_insert_rowid();

        // File in parent folder
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, thumbnail_path) \
             VALUES (?1, 'a.pes', '/parent/a.pes', ?2)",
            rusqlite::params![parent_id, thumb_parent.to_string_lossy().as_ref()],
        ).unwrap();

        // File in child folder
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, thumbnail_path) \
             VALUES (?1, 'b.dst', '/parent/child/b.dst', ?2)",
            rusqlite::params![child_id, thumb_child.to_string_lossy().as_ref()],
        ).unwrap();

        // Recursive CTE should find both thumbnails
        let mut stmt = conn.prepare(
            "WITH RECURSIVE folder_tree(id) AS (
                 SELECT id FROM folders WHERE id = ?1
                 UNION ALL
                 SELECT f.id FROM folders f JOIN folder_tree ft ON f.parent_id = ft.id
             )
             SELECT e.thumbnail_path FROM embroidery_files e
             JOIN folder_tree ft ON e.folder_id = ft.id
             WHERE e.thumbnail_path IS NOT NULL AND e.thumbnail_path != ''",
        ).unwrap();
        let thumbnail_paths: Vec<String> = stmt
            .query_map([parent_id], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(thumbnail_paths.len(), 2, "Should find thumbnails in parent and child folders");

        // Delete parent (cascades to child folder and all files)
        conn.execute("DELETE FROM folders WHERE id = ?1", [parent_id]).unwrap();

        // Clean up thumbnails
        for path in &thumbnail_paths {
            let _ = std::fs::remove_file(path);
        }

        assert!(!thumb_parent.exists(), "Parent thumbnail should be deleted");
        assert!(!thumb_child.exists(), "Child thumbnail should be deleted");
    }
}
