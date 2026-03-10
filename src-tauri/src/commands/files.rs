use tauri::State;

use crate::DbState;
use crate::db::models::{EmbroideryFile, FileFormat, FileThreadColor, FileUpdate, Tag};
use crate::db::queries::{FILE_SELECT, FILE_SELECT_ALIASED, row_to_file};
use crate::error::{lock_db, AppError};

/// Escape SQL LIKE wildcard characters in user input.
fn escape_like(input: &str) -> String {
    input.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

#[tauri::command]
pub fn get_files(
    db: State<'_, DbState>,
    folder_id: Option<i64>,
    search: Option<String>,
    format_filter: Option<String>,
) -> Result<Vec<EmbroideryFile>, AppError> {
    let conn = lock_db(&db)?;

    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx = 1;

    if let Some(fid) = folder_id {
        conditions.push(format!("e.folder_id = ?{param_idx}"));
        params.push(Box::new(fid));
        param_idx += 1;
    }

    if let Some(ref q) = search {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            let escaped = escape_like(trimmed);
            conditions.push(format!(
                "(e.name LIKE ?{pi} ESCAPE '\\' OR e.filename LIKE ?{pi} ESCAPE '\\')",
                pi = param_idx
            ));
            params.push(Box::new(format!("%{escaped}%")));
            param_idx += 1;
        }
    }

    if let Some(ref fmt) = format_filter {
        let trimmed = fmt.trim();
        if !trimmed.is_empty() {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM file_formats ff WHERE ff.file_id = e.id AND ff.format = ?{param_idx})"
            ));
            params.push(Box::new(trimmed.to_uppercase()));
            #[allow(unused_assignments)]
            { param_idx += 1; }
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "{FILE_SELECT_ALIASED}{where_clause} ORDER BY e.filename"
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let files = stmt
        .query_map(param_refs.as_slice(), |row| row_to_file(row))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(files)
}

#[tauri::command]
pub fn get_file(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<EmbroideryFile, AppError> {
    let conn = lock_db(&db)?;

    conn.query_row(
        &format!("{FILE_SELECT} WHERE id = ?1"),
        [file_id],
        |row| row_to_file(row),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
        }
        other => AppError::Database(other),
    })
}

#[tauri::command]
pub fn get_file_formats(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<Vec<FileFormat>, AppError> {
    let conn = lock_db(&db)?;

    let mut stmt = conn.prepare(
        "SELECT id, file_id, format, format_version, filepath, file_size_bytes, parsed \
         FROM file_formats WHERE file_id = ?1 ORDER BY format",
    )?;
    let formats = stmt
        .query_map([file_id], |row| {
            Ok(FileFormat {
                id: row.get(0)?,
                file_id: row.get(1)?,
                format: row.get(2)?,
                format_version: row.get(3)?,
                filepath: row.get(4)?,
                file_size_bytes: row.get(5)?,
                parsed: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(formats)
}

#[tauri::command]
pub fn get_file_colors(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<Vec<FileThreadColor>, AppError> {
    let conn = lock_db(&db)?;

    let mut stmt = conn.prepare(
        "SELECT id, file_id, sort_order, color_hex, color_name, brand, brand_code, is_ai \
         FROM file_thread_colors WHERE file_id = ?1 ORDER BY sort_order",
    )?;
    let colors = stmt
        .query_map([file_id], |row| {
            Ok(FileThreadColor {
                id: row.get(0)?,
                file_id: row.get(1)?,
                sort_order: row.get(2)?,
                color_hex: row.get(3)?,
                color_name: row.get(4)?,
                brand: row.get(5)?,
                brand_code: row.get(6)?,
                is_ai: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(colors)
}

#[tauri::command]
pub fn get_file_tags(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<Vec<Tag>, AppError> {
    let conn = lock_db(&db)?;

    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.created_at FROM tags t \
         INNER JOIN file_tags ft ON ft.tag_id = t.id \
         WHERE ft.file_id = ?1 ORDER BY t.name",
    )?;
    let tags = stmt
        .query_map([file_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(tags)
}

#[tauri::command]
pub fn update_file(
    db: State<'_, DbState>,
    file_id: i64,
    updates: FileUpdate,
) -> Result<EmbroideryFile, AppError> {
    if updates.name.is_none()
        && updates.theme.is_none()
        && updates.description.is_none()
        && updates.license.is_none()
    {
        return Err(AppError::Validation(
            "Mindestens ein Feld muss aktualisiert werden".into(),
        ));
    }

    let conn = lock_db(&db)?;

    let mut set_clauses = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(ref name) = updates.name {
        set_clauses.push(format!("name = ?{idx}"));
        params.push(Box::new(name.clone()));
        idx += 1;
    }
    if let Some(ref theme) = updates.theme {
        set_clauses.push(format!("theme = ?{idx}"));
        params.push(Box::new(theme.clone()));
        idx += 1;
    }
    if let Some(ref description) = updates.description {
        set_clauses.push(format!("description = ?{idx}"));
        params.push(Box::new(description.clone()));
        idx += 1;
    }
    if let Some(ref license) = updates.license {
        set_clauses.push(format!("license = ?{idx}"));
        params.push(Box::new(license.clone()));
        idx += 1;
    }

    set_clauses.push(format!("updated_at = datetime('now')"));

    let sql = format!(
        "UPDATE embroidery_files SET {} WHERE id = ?{idx}",
        set_clauses.join(", ")
    );
    params.push(Box::new(file_id));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changes = conn.execute(&sql, param_refs.as_slice())?;

    if changes == 0 {
        return Err(AppError::NotFound(format!(
            "Datei {file_id} nicht gefunden"
        )));
    }

    conn.query_row(
        &format!("{FILE_SELECT} WHERE id = ?1"),
        [file_id],
        |row| row_to_file(row),
    )
    .map_err(|e| AppError::Database(e))
}

#[tauri::command]
pub fn delete_file(db: State<'_, DbState>, file_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    // Query thumbnail path before deleting the row
    let thumbnail_path: Option<String> = conn
        .query_row(
            "SELECT thumbnail_path FROM embroidery_files WHERE id = ?1",
            [file_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?;

    let changes = conn.execute("DELETE FROM embroidery_files WHERE id = ?1", [file_id])?;
    if changes == 0 {
        return Err(AppError::NotFound(format!(
            "Datei {file_id} nicht gefunden"
        )));
    }

    // Clean up thumbnail file on disk (best-effort)
    if let Some(ref path) = thumbnail_path {
        if !path.is_empty() {
            if let Err(e) = std::fs::remove_file(path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("Failed to remove thumbnail {path}: {e}");
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn set_file_tags(
    db: State<'_, DbState>,
    file_id: i64,
    tag_names: Vec<String>,
) -> Result<Vec<Tag>, AppError> {
    // Deduplicate and clean tag names
    let unique_tags: Vec<String> = tag_names
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let conn = lock_db(&db)?;

    // Verify the file exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM embroidery_files WHERE id = ?1",
        [file_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!(
            "Datei {file_id} nicht gefunden"
        )));
    }

    // Wrap in a transaction for atomicity
    conn.execute_batch("BEGIN")?;

    let result = (|| -> Result<(), AppError> {
        // Remove all existing tags for this file
        conn.execute("DELETE FROM file_tags WHERE file_id = ?1", [file_id])?;

        // Insert each tag and create the junction
        for tag_name in &unique_tags {
            // Create tag if it doesn't exist
            conn.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                [tag_name.as_str()],
            )?;

            // Get the tag id
            let tag_id: i64 = conn.query_row(
                "SELECT id FROM tags WHERE name = ?1",
                [tag_name.as_str()],
                |row| row.get(0),
            )?;

            // Create the junction
            conn.execute(
                "INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
                rusqlite::params![file_id, tag_id],
            )?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            return Err(e);
        }
    }

    // Return the resulting tags
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.created_at FROM tags t \
         INNER JOIN file_tags ft ON ft.tag_id = t.id \
         WHERE ft.file_id = ?1 ORDER BY t.name",
    )?;
    let tags = stmt
        .query_map([file_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(tags)
}

#[tauri::command]
pub fn get_all_tags(db: State<'_, DbState>) -> Result<Vec<Tag>, AppError> {
    let conn = lock_db(&db)?;

    let mut stmt = conn.prepare(
        "SELECT id, name, created_at FROM tags ORDER BY name",
    )?;
    let tags = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(tags)
}

#[tauri::command]
pub fn get_thumbnail(db: State<'_, DbState>, file_id: i64) -> Result<String, AppError> {
    use base64::Engine;

    // Query thumbnail path from DB, then drop the lock before file I/O
    let thumbnail_path: Option<String> = {
        let conn = lock_db(&db)?;
        conn.query_row(
            "SELECT thumbnail_path FROM embroidery_files WHERE id = ?1",
            [file_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?
    };

    // Read file without holding the DB lock
    match thumbnail_path {
        Some(path) if !path.is_empty() => {
            let data = std::fs::read(&path)?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            Ok(format!("data:image/png;base64,{b64}"))
        }
        _ => Ok(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_get_files_with_folder_filter() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('A', '/a')", []).unwrap();
        let folder_a = conn.last_insert_rowid();
        conn.execute("INSERT INTO folders (name, path) VALUES ('B', '/b')", []).unwrap();
        let folder_b = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/a/a.pes')",
            [folder_a],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'b.dst', '/b/b.dst')",
            [folder_b],
        ).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM embroidery_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1",
                [folder_a],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_files_with_search() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, name) \
             VALUES (?1, 'rose.pes', '/test/rose.pes', 'Rose Design')",
            [folder_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, name) \
             VALUES (?1, 'star.dst', '/test/star.dst', 'Star Pattern')",
            [folder_id],
        ).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE name LIKE '%Rose%' OR filename LIKE '%Rose%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_file_not_found() {
        let conn = init_database_in_memory().unwrap();
        let result: Result<String, _> = conn.query_row(
            "SELECT filename FROM embroidery_files WHERE id = 9999",
            [],
            |row| row.get(0),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_file_tags_join() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/test/a.pes')",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        conn.execute("INSERT INTO tags (name) VALUES ('floral')", []).unwrap();
        let tag_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![file_id, tag_id],
        ).unwrap();

        let tag_name: String = conn
            .query_row(
                "SELECT t.name FROM tags t INNER JOIN file_tags ft ON ft.tag_id = t.id WHERE ft.file_id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tag_name, "floral");
    }

    #[test]
    fn test_escape_like_wildcards() {
        use super::escape_like;
        assert_eq!(escape_like("hello"), "hello");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_update_file() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/test/a.pes')",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        conn.execute(
            "UPDATE embroidery_files SET name = 'Updated Name', theme = 'Floral', updated_at = datetime('now') WHERE id = ?1",
            [file_id],
        ).unwrap();

        let (name, theme): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT name, theme FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, Some("Updated Name".to_string()));
        assert_eq!(theme, Some("Floral".to_string()));
    }

    #[test]
    fn test_delete_file() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/test/a.pes')",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        let changes = conn.execute("DELETE FROM embroidery_files WHERE id = ?1", [file_id]).unwrap();
        assert_eq!(changes, 1);

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists);
    }

    #[test]
    fn test_set_file_tags() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/test/a.pes')",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        // Insert tags
        conn.execute("INSERT OR IGNORE INTO tags (name) VALUES ('floral')", []).unwrap();
        conn.execute("INSERT OR IGNORE INTO tags (name) VALUES ('nature')", []).unwrap();

        let floral_id: i64 = conn.query_row("SELECT id FROM tags WHERE name = 'floral'", [], |row| row.get(0)).unwrap();
        let nature_id: i64 = conn.query_row("SELECT id FROM tags WHERE name = 'nature'", [], |row| row.get(0)).unwrap();

        conn.execute("INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)", rusqlite::params![file_id, floral_id]).unwrap();
        conn.execute("INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)", rusqlite::params![file_id, nature_id]).unwrap();

        let tag_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM file_tags WHERE file_id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tag_count, 2);

        // Replace with different tags
        conn.execute("DELETE FROM file_tags WHERE file_id = ?1", [file_id]).unwrap();
        conn.execute("INSERT OR IGNORE INTO tags (name) VALUES ('modern')", []).unwrap();
        let modern_id: i64 = conn.query_row("SELECT id FROM tags WHERE name = 'modern'", [], |row| row.get(0)).unwrap();
        conn.execute("INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)", rusqlite::params![file_id, modern_id]).unwrap();

        let tag_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM file_tags WHERE file_id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tag_count, 1);

        let tag_name: String = conn
            .query_row(
                "SELECT t.name FROM tags t INNER JOIN file_tags ft ON ft.tag_id = t.id WHERE ft.file_id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tag_name, "modern");
    }

    #[test]
    fn test_base64_encoding() {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(b"Hello");
        assert_eq!(encoded, "SGVsbG8=");

        let encoded = base64::engine::general_purpose::STANDARD.encode(b"Hi");
        assert_eq!(encoded, "SGk=");

        let encoded = base64::engine::general_purpose::STANDARD.encode(b"abc");
        assert_eq!(encoded, "YWJj");
    }

    #[test]
    fn test_delete_file_cleans_up_thumbnail() {
        let conn = init_database_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();

        // Create a fake thumbnail file on disk
        let thumb_path = tmp.path().join("42.png");
        std::fs::write(&thumb_path, b"fake png").unwrap();
        assert!(thumb_path.exists());

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, thumbnail_path) \
             VALUES (?1, 'a.pes', '/test/a.pes', ?2)",
            rusqlite::params![folder_id, thumb_path.to_string_lossy().as_ref()],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        // Query thumbnail path, delete row, then clean up — mirrors the command logic
        let thumbnail_path: Option<String> = conn
            .query_row(
                "SELECT thumbnail_path FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();

        conn.execute("DELETE FROM embroidery_files WHERE id = ?1", [file_id]).unwrap();

        // Clean up thumbnail (mirrors the new delete_file logic)
        if let Some(ref path) = thumbnail_path {
            if !path.is_empty() {
                let _ = std::fs::remove_file(path);
            }
        }

        assert!(!thumb_path.exists(), "Thumbnail should be deleted from disk");
    }

    #[test]
    fn test_delete_file_no_thumbnail_path() {
        let conn = init_database_in_memory().unwrap();

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/test/a.pes')",
            [folder_id],
        ).unwrap();
        let file_id = conn.last_insert_rowid();

        // Should not error when thumbnail_path is NULL
        let thumbnail_path: Option<String> = conn
            .query_row(
                "SELECT thumbnail_path FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(thumbnail_path.is_none());

        let changes = conn.execute("DELETE FROM embroidery_files WHERE id = ?1", [file_id]).unwrap();
        assert_eq!(changes, 1);
    }
}
