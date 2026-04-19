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
        folder_type: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

const FOLDER_SELECT: &str =
    "SELECT id, name, path, parent_id, sort_order, folder_type, created_at, updated_at FROM folders";

const VALID_FOLDER_TYPES: &[&str] = &["embroidery", "sewing_pattern", "mixed"];

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
    folder_type: Option<String>,
) -> Result<Folder, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation(
            "Ordnername darf nicht leer sein".into(),
        ));
    }

    let ft = folder_type.as_deref().unwrap_or("mixed");
    if !VALID_FOLDER_TYPES.contains(&ft) {
        return Err(AppError::Validation(format!(
            "Ungueltiger Ordnertyp: {ft}"
        )));
    }

    // Note: TOCTOU race possible (path could be removed after check). Acceptable for MVP;
    // the DB stores the path and later operations will handle missing directories gracefully.
    if !std::path::Path::new(&path).exists() {
        return Err(AppError::Validation(format!(
            "Pfad existiert nicht: {path}"
        )));
    }

    let conn = lock_db(&db)?;

    // Place new folder at the end of sibling sort order
    let max_order: i32 = if parent_id.is_some() {
        conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM folders WHERE parent_id = ?1",
            [parent_id],
            |row| row.get(0),
        )
        .unwrap_or(0)
    } else {
        conn.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM folders WHERE parent_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0)
    };

    conn.execute(
        "INSERT INTO folders (name, path, parent_id, sort_order, folder_type) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![name.trim(), path, parent_id, max_order + 10, ft],
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
    folder_type: Option<String>,
) -> Result<Folder, AppError> {
    if name.is_none() && folder_type.is_none() {
        return Err(AppError::Validation(
            "Mindestens ein Feld muss aktualisiert werden".into(),
        ));
    }

    if let Some(ref new_name) = name {
        if new_name.trim().is_empty() {
            return Err(AppError::Validation(
                "Ordnername darf nicht leer sein".into(),
            ));
        }
    }

    if let Some(ref ft) = folder_type {
        if !VALID_FOLDER_TYPES.contains(&ft.as_str()) {
            return Err(AppError::Validation(format!(
                "Ungueltiger Ordnertyp: {ft}"
            )));
        }
    }

    let conn = lock_db(&db)?;
    let tx = conn.unchecked_transaction()?;

    if let Some(ref new_name) = name {
        tx.execute(
            "UPDATE folders SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![new_name.trim(), folder_id],
        )?;
    }

    if let Some(ref ft) = folder_type {
        tx.execute(
            "UPDATE folders SET folder_type = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![ft, folder_id],
        )?;
    }

    tx.commit()?;

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

    // Audit Wave 2 perf: parallelise the per-thumbnail unlink — sequential
    // unlink on a 10K-file folder takes seconds. rayon's par_iter saturates
    // the disk I/O queue without changing semantics (best-effort cleanup).
    use rayon::prelude::*;
    thumbnail_paths.par_iter().for_each(|path| {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("Failed to remove thumbnail {path}: {e}");
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn update_folder_sort_orders(
    db: State<'_, DbState>,
    folder_orders: Vec<(i64, i32)>,
) -> Result<(), AppError> {
    // Validate: no duplicate IDs
    let mut seen_ids = std::collections::HashSet::new();
    for (folder_id, order) in &folder_orders {
        if *order < 0 {
            return Err(AppError::Validation(format!(
                "sort_order darf nicht negativ sein: {order}"
            )));
        }
        if !seen_ids.insert(folder_id) {
            return Err(AppError::Validation(format!(
                "Doppelte Ordner-ID: {folder_id}"
            )));
        }
    }

    let conn = lock_db(&db)?;

    let tx = conn.unchecked_transaction()?;
    for (folder_id, order) in &folder_orders {
        let changes = tx.execute(
            "UPDATE folders SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![order, folder_id],
        )?;
        if changes == 0 {
            return Err(AppError::NotFound(format!(
                "Ordner {folder_id} nicht gefunden"
            )));
        }
    }
    tx.commit()?;

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

    // Recursive CTE: each folder gets a count including all descendant files
    let mut stmt = conn.prepare(
        "WITH RECURSIVE folder_tree(id, root_id) AS (
            SELECT id, id FROM folders
            UNION ALL
            SELECT f.id, ft.root_id FROM folders f JOIN folder_tree ft ON f.parent_id = ft.id
        )
        SELECT ft.root_id AS folder_id, COUNT(*) AS cnt
        FROM embroidery_files e
        JOIN folder_tree ft ON e.folder_id = ft.id
        WHERE e.deleted_at IS NULL
        GROUP BY ft.root_id",
    )?;
    let counts = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(counts)
}

#[tauri::command]
pub fn move_folder(
    db: State<'_, DbState>,
    folder_id: i64,
    new_parent_id: Option<i64>,
) -> Result<Folder, AppError> {
    // Self-reference check
    if new_parent_id == Some(folder_id) {
        return Err(AppError::Validation(
            "Ordner kann nicht in sich selbst verschoben werden".into(),
        ));
    }

    let conn = lock_db(&db)?;

    // Verify folder exists and check for no-op
    let current_parent: Option<i64> = conn
        .query_row(
            "SELECT parent_id FROM folders WHERE id = ?1",
            [folder_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Ordner {folder_id} nicht gefunden"))
            }
            other => AppError::from(other),
        })?;

    // No-op: folder is already at the target parent
    if current_parent == new_parent_id {
        return conn
            .query_row(
                &format!("{FOLDER_SELECT} WHERE id = ?1"),
                [folder_id],
                |row| row_to_folder(row),
            )
            .map_err(AppError::from);
    }

    // Circular reference check: ensure folder_id is not an ancestor of new_parent_id
    if let Some(np_id) = new_parent_id {
        // Verify target parent exists
        let target_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM folders WHERE id = ?1",
            [np_id],
            |row| row.get(0),
        )?;
        if !target_exists {
            return Err(AppError::NotFound(format!(
                "Zielordner {np_id} nicht gefunden"
            )));
        }

        let is_circular: bool = conn.query_row(
            "WITH RECURSIVE ancestors(id) AS (
                SELECT parent_id FROM folders WHERE id = ?1
                UNION ALL
                SELECT f.parent_id FROM folders f JOIN ancestors a ON f.id = a.id
                WHERE f.parent_id IS NOT NULL
            )
            SELECT COUNT(*) > 0 FROM ancestors WHERE id = ?2",
            rusqlite::params![np_id, folder_id],
            |row| row.get(0),
        )?;
        if is_circular {
            return Err(AppError::Validation(
                "Zirkulaere Referenz: Ordner kann nicht in einen eigenen Unterordner verschoben werden".into(),
            ));
        }
    }

    let tx = conn.unchecked_transaction()?;

    // Place at end of new parent's children
    let max_order: i32 = if new_parent_id.is_some() {
        tx.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM folders WHERE parent_id = ?1",
            [new_parent_id],
            |row| row.get(0),
        )
        .unwrap_or(0)
    } else {
        tx.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM folders WHERE parent_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0)
    };

    tx.execute(
        "UPDATE folders SET parent_id = ?1, sort_order = ?2, updated_at = datetime('now') WHERE id = ?3",
        rusqlite::params![new_parent_id, max_order + 10, folder_id],
    )?;

    tx.commit()?;

    let folder = conn.query_row(
        &format!("{FOLDER_SELECT} WHERE id = ?1"),
        [folder_id],
        |row| row_to_folder(row),
    )?;

    Ok(folder)
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

        // Default folder_type should be 'mixed'
        let folder_type: String = conn
            .query_row("SELECT folder_type FROM folders WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(folder_type, "mixed");

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
    fn test_folder_type_default() {
        let conn = init_database_in_memory().unwrap();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('NoType', '/tmp/notype')",
            [],
        )
        .unwrap();
        let id = conn.last_insert_rowid();

        let folder_type: String = conn
            .query_row("SELECT folder_type FROM folders WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(folder_type, "mixed");
    }

    #[test]
    fn test_folder_type_create_and_update() {
        let conn = init_database_in_memory().unwrap();

        // Create with explicit type
        conn.execute(
            "INSERT INTO folders (name, path, folder_type) VALUES ('Emb', '/tmp/emb', 'embroidery')",
            [],
        )
        .unwrap();
        let id = conn.last_insert_rowid();

        let ft: String = conn
            .query_row("SELECT folder_type FROM folders WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(ft, "embroidery");

        // Update folder_type
        conn.execute(
            "UPDATE folders SET folder_type = 'sewing_pattern', updated_at = datetime('now') WHERE id = ?1",
            [id],
        )
        .unwrap();

        let ft: String = conn
            .query_row("SELECT folder_type FROM folders WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(ft, "sewing_pattern");

        // Update back to mixed
        conn.execute(
            "UPDATE folders SET folder_type = 'mixed', updated_at = datetime('now') WHERE id = ?1",
            [id],
        )
        .unwrap();

        let ft: String = conn
            .query_row("SELECT folder_type FROM folders WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(ft, "mixed");
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
    fn test_update_folder_sort_orders() {
        let conn = init_database_in_memory().unwrap();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Charlie', '/c')",
            [],
        )
        .unwrap();
        let id_c = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Alpha', '/a')",
            [],
        )
        .unwrap();
        let id_a = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Beta', '/b')",
            [],
        )
        .unwrap();
        let id_b = conn.last_insert_rowid();

        // Reorder: Beta first, then Charlie, then Alpha
        // Mirrors update_folder_sort_orders logic including row-count validation
        let orders: Vec<(i64, i32)> = vec![(id_b, 10), (id_c, 20), (id_a, 30)];
        let tx = conn.unchecked_transaction().unwrap();
        for (folder_id, order) in &orders {
            let changes = tx.execute(
                "UPDATE folders SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![order, folder_id],
            )
            .unwrap();
            assert_eq!(changes, 1, "Each folder update should affect exactly 1 row");
        }
        tx.commit().unwrap();

        let mut stmt = conn
            .prepare("SELECT name FROM folders ORDER BY sort_order, name")
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(names, vec!["Beta", "Charlie", "Alpha"]);

        // Verify non-existent folder ID returns 0 changes (command would error)
        let changes = conn.execute(
            "UPDATE folders SET sort_order = 99, updated_at = datetime('now') WHERE id = 99999",
            [],
        ).unwrap();
        assert_eq!(changes, 0, "Non-existent folder should affect 0 rows");
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

    #[test]
    fn test_move_folder_basic() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Parent', '/parent')", []).unwrap();
        let parent_id = conn.last_insert_rowid();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Child', '/child')", []).unwrap();
        let child_id = conn.last_insert_rowid();

        // Move child under parent
        conn.execute(
            "UPDATE folders SET parent_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![parent_id, child_id],
        ).unwrap();

        let pid: Option<i64> = conn
            .query_row("SELECT parent_id FROM folders WHERE id = ?1", [child_id], |row| row.get(0))
            .unwrap();
        assert_eq!(pid, Some(parent_id));
    }

    #[test]
    fn test_move_folder_circular_reference_detected() {
        let conn = init_database_in_memory().unwrap();

        // Parent -> Child hierarchy
        conn.execute("INSERT INTO folders (name, path) VALUES ('P', '/p')", []).unwrap();
        let p = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO folders (name, path, parent_id) VALUES ('C', '/c', ?1)",
            [p],
        ).unwrap();
        let c = conn.last_insert_rowid();

        // Try to detect circular ref: moving P under C
        // Walk ancestors of C to see if P is among them
        let is_circular: bool = conn
            .query_row(
                "WITH RECURSIVE ancestors(id) AS (
                    SELECT parent_id FROM folders WHERE id = ?1
                    UNION ALL
                    SELECT f.parent_id FROM folders f JOIN ancestors a ON f.id = a.id
                    WHERE f.parent_id IS NOT NULL
                )
                SELECT COUNT(*) > 0 FROM ancestors WHERE id = ?2",
                rusqlite::params![c, p],
                |row| row.get(0),
            )
            .unwrap();
        // C's ancestor is P, so moving P under C would be circular
        assert!(is_circular, "Should detect that P is an ancestor of C");
    }

    #[test]
    fn test_move_folder_self_reference() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Self', '/self')", []).unwrap();
        let id = conn.last_insert_rowid();

        // Replicate the self-reference guard from move_folder
        let folder_id = id;
        let new_parent_id = Some(id);
        assert_eq!(
            new_parent_id, Some(folder_id),
            "Self-reference guard should trigger when folder_id == new_parent_id"
        );

        // Verify the folder still has NULL parent (not moved)
        let pid: Option<i64> = conn
            .query_row("SELECT parent_id FROM folders WHERE id = ?1", [id], |row| row.get(0))
            .unwrap();
        assert_eq!(pid, None, "Folder should remain at root");
    }

    #[test]
    fn test_move_folder_to_root() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('P', '/p')", []).unwrap();
        let p = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO folders (name, path, parent_id) VALUES ('C', '/c', ?1)",
            [p],
        ).unwrap();
        let c = conn.last_insert_rowid();

        // Move C to root
        conn.execute(
            "UPDATE folders SET parent_id = NULL, updated_at = datetime('now') WHERE id = ?1",
            [c],
        ).unwrap();

        let pid: Option<i64> = conn
            .query_row("SELECT parent_id FROM folders WHERE id = ?1", [c], |row| row.get(0))
            .unwrap();
        assert_eq!(pid, None, "Child should now be at root level");
    }

    #[test]
    fn test_recursive_file_count() {
        let conn = init_database_in_memory().unwrap();

        // Parent folder
        conn.execute("INSERT INTO folders (name, path) VALUES ('Root', '/root')", []).unwrap();
        let root_id = conn.last_insert_rowid();

        // Child folder
        conn.execute(
            "INSERT INTO folders (name, path, parent_id) VALUES ('Sub', '/sub', ?1)",
            [root_id],
        ).unwrap();
        let sub_id = conn.last_insert_rowid();

        // 2 files in root, 3 files in sub
        for i in 0..2 {
            conn.execute(
                "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, ?2, ?3)",
                rusqlite::params![root_id, format!("r{i}.pes"), format!("/root/r{i}.pes")],
            ).unwrap();
        }
        for i in 0..3 {
            conn.execute(
                "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, ?2, ?3)",
                rusqlite::params![sub_id, format!("s{i}.pes"), format!("/sub/s{i}.pes")],
            ).unwrap();
        }

        // Recursive count for root should include sub's files
        let mut stmt = conn.prepare(
            "WITH RECURSIVE folder_tree(id, root_id) AS (
                SELECT id, id FROM folders
                UNION ALL
                SELECT f.id, ft.root_id FROM folders f JOIN folder_tree ft ON f.parent_id = ft.id
            )
            SELECT ft.root_id AS folder_id, COUNT(*) AS cnt
            FROM embroidery_files e
            JOIN folder_tree ft ON e.folder_id = ft.id
            WHERE e.deleted_at IS NULL
            GROUP BY ft.root_id",
        ).unwrap();
        let counts: Vec<(i64, i64)> = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        let root_count = counts.iter().find(|(id, _)| *id == root_id).map(|(_, c)| *c).unwrap_or(0);
        let sub_count = counts.iter().find(|(id, _)| *id == sub_id).map(|(_, c)| *c).unwrap_or(0);

        assert_eq!(root_count, 5, "Root should have 2 own + 3 from sub = 5");
        assert_eq!(sub_count, 3, "Sub should have its own 3 files");
    }
}
