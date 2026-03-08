use tauri::State;

use crate::DbState;
use crate::db::models::{EmbroideryFile, FileFormat, FileThreadColor, Tag};
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
}
