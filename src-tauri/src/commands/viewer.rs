use base64::Engine;
use tauri::State;

use crate::db::models::{InstructionBookmark, InstructionNote};
use crate::error::{lock_db, AppError};
use crate::DbState;

/// Read a file from disk and return its contents as base64-encoded data.
/// Used by the frontend document/image viewer to load files.
/// Restricted to paths known to the application (#121).
#[tauri::command]
pub fn read_file_bytes(
    db: State<'_, DbState>,
    file_path: String,
) -> Result<String, AppError> {
    super::validate_no_traversal(&file_path)?;

    // Audit Wave 1: enforce a viewer-format allow-list before any DB lookup.
    let ext = super::lower_ext(std::path::Path::new(&file_path));
    if ext.is_empty() || !super::VIEWER_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "Dateityp .{ext} kann im Viewer nicht angezeigt werden"
        )));
    }

    // Validate the path is known to the application
    let conn = lock_db(&db)?;
    let canonical = std::fs::canonicalize(&file_path).unwrap_or_else(|_| std::path::PathBuf::from(&file_path));
    let canonical_str = canonical.to_string_lossy().to_string();

    // Check: is this path in embroidery_files, file_attachments, or under library_root?
    let in_files: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM embroidery_files WHERE (filepath = ?1 OR filepath = ?2) AND deleted_at IS NULL",
        [&file_path, &canonical_str],
        |row| row.get(0),
    ).unwrap_or(false);

    let in_attachments: bool = if !in_files {
        conn.query_row(
            "SELECT COUNT(*) > 0 FROM file_attachments WHERE file_path = ?1 OR file_path = ?2",
            [&file_path, &canonical_str],
            |row| row.get(0),
        ).unwrap_or(false)
    } else { false };

    let in_library: bool = if !in_files && !in_attachments {
        if let Ok(root) = conn.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key = 'library_root'", [], |row| row.get(0),
        ) {
            let root_path = if root.starts_with("~/") {
                dirs::home_dir().map(|h| h.join(&root[2..])).unwrap_or_else(|| std::path::PathBuf::from(&root))
            } else {
                std::path::PathBuf::from(&root)
            };
            let canonical_root = std::fs::canonicalize(&root_path).unwrap_or(root_path);
            canonical.starts_with(&canonical_root)
        } else { false }
    } else { false };

    if !in_files && !in_attachments && !in_library {
        return Err(AppError::Validation(
            "Zugriff verweigert: Dateipfad ist nicht in der Bibliothek".into(),
        ));
    }
    drop(conn);
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "Datei nicht gefunden: {file_path}"
        )));
    }
    if !path.is_file() {
        return Err(AppError::Validation(format!(
            "Kein regulaere Datei: {file_path}"
        )));
    }
    // Reject files larger than 100 MB to prevent OOM from base64 bloat
    const MAX_VIEWER_FILE_SIZE: u64 = 100 * 1024 * 1024;
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_VIEWER_FILE_SIZE {
            return Err(AppError::Validation(format!(
                "Datei zu gross zum Anzeigen ({} bytes, max {} bytes)",
                meta.len(), MAX_VIEWER_FILE_SIZE
            )));
        }
    }
    let data = std::fs::read(path)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&data))
}

/// Toggle a bookmark for a specific page. Returns true if added, false if removed.
#[tauri::command]
pub fn toggle_bookmark(
    db: State<'_, DbState>,
    file_id: i64,
    page_number: i32,
    label: Option<String>,
) -> Result<bool, AppError> {
    if page_number < 1 {
        return Err(AppError::Validation("Seitennummer muss >= 1 sein".into()));
    }
    let conn = lock_db(&db)?;

    // Check if bookmark exists
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM instruction_bookmarks WHERE file_id = ?1 AND page_number = ?2",
            rusqlite::params![file_id, page_number],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing {
        conn.execute("DELETE FROM instruction_bookmarks WHERE id = ?1", [id])?;
        Ok(false)
    } else {
        conn.execute(
            "INSERT INTO instruction_bookmarks (file_id, page_number, label) VALUES (?1, ?2, ?3)",
            rusqlite::params![file_id, page_number, label],
        )?;
        Ok(true)
    }
}

/// Get all bookmarks for a file, ordered by page number.
#[tauri::command]
pub fn get_bookmarks(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<Vec<InstructionBookmark>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, file_id, page_number, label, created_at \
         FROM instruction_bookmarks WHERE file_id = ?1 ORDER BY page_number",
    )?;
    let bookmarks = stmt
        .query_map([file_id], |row| {
            Ok(InstructionBookmark {
                id: row.get(0)?,
                file_id: row.get(1)?,
                page_number: row.get(2)?,
                label: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(bookmarks)
}

/// Update a bookmark's label.
#[tauri::command]
pub fn update_bookmark_label(
    db: State<'_, DbState>,
    bookmark_id: i64,
    label: String,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE instruction_bookmarks SET label = ?1 WHERE id = ?2",
        rusqlite::params![label, bookmark_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!(
            "Lesezeichen {bookmark_id} nicht gefunden"
        )));
    }
    Ok(())
}

/// Add a note to a specific page.
#[tauri::command]
pub fn add_note(
    db: State<'_, DbState>,
    file_id: i64,
    page_number: i32,
    note_text: String,
) -> Result<InstructionNote, AppError> {
    if page_number < 1 {
        return Err(AppError::Validation("Seitennummer muss >= 1 sein".into()));
    }
    let trimmed = note_text.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Notiztext darf nicht leer sein".into()));
    }
    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO instruction_notes (file_id, page_number, note_text) VALUES (?1, ?2, ?3)",
        rusqlite::params![file_id, page_number, trimmed],
    )?;
    let id = conn.last_insert_rowid();
    let note = conn.query_row(
        "SELECT id, file_id, page_number, note_text, created_at, updated_at \
         FROM instruction_notes WHERE id = ?1",
        [id],
        |row| {
            Ok(InstructionNote {
                id: row.get(0)?,
                file_id: row.get(1)?,
                page_number: row.get(2)?,
                note_text: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )?;
    Ok(note)
}

/// Update a note's text.
#[tauri::command]
pub fn update_note(
    db: State<'_, DbState>,
    note_id: i64,
    note_text: String,
) -> Result<(), AppError> {
    let trimmed = note_text.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Notiztext darf nicht leer sein".into()));
    }
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE instruction_notes SET note_text = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![trimmed, note_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!(
            "Notiz {note_id} nicht gefunden"
        )));
    }
    Ok(())
}

/// Delete a note.
#[tauri::command]
pub fn delete_note(
    db: State<'_, DbState>,
    note_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute("DELETE FROM instruction_notes WHERE id = ?1", [note_id])?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Notiz {note_id} nicht gefunden")));
    }
    Ok(())
}

/// Get notes for a file, optionally filtered by page number.
#[tauri::command]
pub fn get_notes(
    db: State<'_, DbState>,
    file_id: i64,
    page_number: Option<i32>,
) -> Result<Vec<InstructionNote>, AppError> {
    let conn = lock_db(&db)?;
    let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(page) = page_number {
        (
            "SELECT id, file_id, page_number, note_text, created_at, updated_at \
             FROM instruction_notes WHERE file_id = ?1 AND page_number = ?2 \
             ORDER BY created_at",
            vec![Box::new(file_id), Box::new(page)],
        )
    } else {
        (
            "SELECT id, file_id, page_number, note_text, created_at, updated_at \
             FROM instruction_notes WHERE file_id = ?1 \
             ORDER BY page_number, created_at",
            vec![Box::new(file_id)],
        )
    };

    let mut stmt = conn.prepare(sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let notes = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(InstructionNote {
                id: row.get(0)?,
                file_id: row.get(1)?,
                page_number: row.get(2)?,
                note_text: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(notes)
}

/// Save the last viewed page for a document.
#[tauri::command]
pub fn set_last_viewed_page(
    db: State<'_, DbState>,
    file_id: i64,
    page_number: i32,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let key = format!("last_page:{file_id}");
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value, updated_at) \
         VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![key, page_number.to_string()],
    )?;
    Ok(())
}

/// Get the last viewed page for a document.
#[tauri::command]
pub fn get_last_viewed_page(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<Option<i32>, AppError> {
    let conn = lock_db(&db)?;
    let key = format!("last_page:{file_id}");
    let result: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [&key],
            |row| row.get(0),
        )
        .ok();
    Ok(result.and_then(|v| v.parse().ok()))
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_toggle_bookmark_add_remove() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (1, 'test.pdf', '/test/test.pdf')",
            [],
        ).unwrap();

        // Add bookmark
        conn.execute(
            "INSERT INTO instruction_bookmarks (file_id, page_number, label) VALUES (1, 3, 'Schnittlayout')",
            [],
        ).unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM instruction_bookmarks WHERE file_id = 1",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);

        // Remove bookmark
        conn.execute("DELETE FROM instruction_bookmarks WHERE file_id = 1 AND page_number = 3", []).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM instruction_bookmarks WHERE file_id = 1",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_notes_crud() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (1, 'test.pdf', '/test/test.pdf')",
            [],
        ).unwrap();

        // Add note
        conn.execute(
            "INSERT INTO instruction_notes (file_id, page_number, note_text) VALUES (1, 5, 'Nahtzugabe anpassen')",
            [],
        ).unwrap();
        let note_id = conn.last_insert_rowid();

        // Update note
        conn.execute(
            "UPDATE instruction_notes SET note_text = 'Nahtzugabe 1.5cm', updated_at = datetime('now') WHERE id = ?1",
            [note_id],
        ).unwrap();

        let text: String = conn.query_row(
            "SELECT note_text FROM instruction_notes WHERE id = ?1",
            [note_id], |row| row.get(0),
        ).unwrap();
        assert_eq!(text, "Nahtzugabe 1.5cm");

        // Delete note
        conn.execute("DELETE FROM instruction_notes WHERE id = ?1", [note_id]).unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM instruction_notes WHERE file_id = 1",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_bookmark_cascade_delete() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (1, 'test.pdf', '/test/test.pdf')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO instruction_bookmarks (file_id, page_number) VALUES (1, 1)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO instruction_notes (file_id, page_number, note_text) VALUES (1, 1, 'Test note')",
            [],
        ).unwrap();

        // Delete the file — should cascade
        conn.execute("DELETE FROM embroidery_files WHERE id = 1", []).unwrap();

        let bm_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM instruction_bookmarks", [], |row| row.get(0),
        ).unwrap();
        let note_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM instruction_notes", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(bm_count, 0);
        assert_eq!(note_count, 0);
    }
}
