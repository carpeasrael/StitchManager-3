use tauri::State;

use crate::{DbState, ThumbnailState};
use crate::db::models::{EmbroideryFile, FileAttachment, FileFormat, FileThreadColor, FileUpdate, SearchParams, Tag};
use crate::db::queries::{FILE_SELECT, FILE_SELECT_ALIASED, row_to_file};
use crate::error::{lock_db, AppError};

/// Escape SQL LIKE wildcard characters in user input.
fn escape_like(input: &str) -> String {
    input.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Core query-building logic shared by the Tauri command and tests.
/// Accepts a plain `&rusqlite::Connection` so it can be called without `State`.
pub(crate) fn query_files_impl(
    conn: &rusqlite::Connection,
    folder_id: Option<i64>,
    search: Option<String>,
    format_filter: Option<String>,
    search_params: Option<SearchParams>,
) -> Result<Vec<EmbroideryFile>, AppError> {
    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx: usize = 1;

    if let Some(fid) = folder_id {
        conditions.push(format!("e.folder_id = ?{param_idx}"));
        params.push(Box::new(fid));
        param_idx += 1;
    }

    // Determine the text query: prefer search_params.text, fall back to legacy `search`
    let text_query = search_params
        .as_ref()
        .and_then(|sp| sp.text.clone())
        .or(search);

    if let Some(ref q) = text_query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            let escaped = escape_like(trimmed);
            let like_val = format!("%{escaped}%");
            let text_fields = [
                "e.name", "e.filename", "e.theme", "e.description",
                "e.design_name", "e.category", "e.author", "e.keywords",
                "e.comments", "e.license", "e.unique_id",
            ];
            let clauses: Vec<String> = text_fields
                .iter()
                .map(|f| format!("{f} LIKE ?{param_idx} ESCAPE '\\'"))
                .collect();
            conditions.push(format!("({})", clauses.join(" OR ")));
            params.push(Box::new(like_val));
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
            param_idx += 1;
        }
    }

    // Advanced search params
    if let Some(ref sp) = search_params {
        // Tag filter (AND logic: file must have ALL listed tags)
        if let Some(ref tags) = sp.tags {
            for tag_name in tags {
                let trimmed = tag_name.trim();
                if !trimmed.is_empty() {
                    conditions.push(format!(
                        "EXISTS (SELECT 1 FROM file_tags ft JOIN tags t ON t.id = ft.tag_id \
                         WHERE ft.file_id = e.id AND t.name = ?{param_idx})"
                    ));
                    params.push(Box::new(trimmed.to_string()));
                    param_idx += 1;
                }
            }
        }

        // Numeric range: stitch_count
        if let Some(v) = sp.stitch_count_min {
            conditions.push(format!("e.stitch_count >= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }
        if let Some(v) = sp.stitch_count_max {
            conditions.push(format!("e.stitch_count <= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }

        // Numeric range: color_count
        if let Some(v) = sp.color_count_min {
            conditions.push(format!("e.color_count >= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }
        if let Some(v) = sp.color_count_max {
            conditions.push(format!("e.color_count <= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }

        // Numeric range: width_mm
        if let Some(v) = sp.width_mm_min {
            conditions.push(format!("e.width_mm >= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }
        if let Some(v) = sp.width_mm_max {
            conditions.push(format!("e.width_mm <= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }

        // Numeric range: height_mm
        if let Some(v) = sp.height_mm_min {
            conditions.push(format!("e.height_mm >= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }
        if let Some(v) = sp.height_mm_max {
            conditions.push(format!("e.height_mm <= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }

        // Numeric range: file_size
        if let Some(v) = sp.file_size_min {
            conditions.push(format!("e.file_size_bytes >= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }
        if let Some(v) = sp.file_size_max {
            conditions.push(format!("e.file_size_bytes <= ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }

        // Boolean: ai_analyzed
        if let Some(v) = sp.ai_analyzed {
            conditions.push(format!("e.ai_analyzed = ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }

        // Boolean: ai_confirmed
        if let Some(v) = sp.ai_confirmed {
            conditions.push(format!("e.ai_confirmed = ?{param_idx}"));
            params.push(Box::new(v));
            param_idx += 1;
        }

        // Color/brand search
        if let Some(ref cs) = sp.color_search {
            let trimmed = cs.trim();
            if !trimmed.is_empty() {
                let escaped = escape_like(trimmed);
                let like_val = format!("%{escaped}%");
                conditions.push(format!(
                    "EXISTS (SELECT 1 FROM file_thread_colors ftc WHERE ftc.file_id = e.id \
                     AND (ftc.color_name LIKE ?{param_idx} ESCAPE '\\' OR ftc.brand LIKE ?{param_idx} ESCAPE '\\'))"
                ));
                params.push(Box::new(like_val));
                #[allow(unused_assignments)]
                { param_idx += 1; }
            }
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
pub fn get_files(
    db: State<'_, DbState>,
    folder_id: Option<i64>,
    search: Option<String>,
    format_filter: Option<String>,
    search_params: Option<SearchParams>,
) -> Result<Vec<EmbroideryFile>, AppError> {
    let conn = lock_db(&db)?;
    query_files_impl(&conn, folder_id, search, format_filter, search_params)
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
pub fn get_thumbnail(
    db: State<'_, DbState>,
    thumb_state: State<'_, ThumbnailState>,
    file_id: i64,
) -> Result<String, AppError> {
    use base64::Engine;

    // Query thumbnail path and filepath from DB, then drop the lock before file I/O
    let (thumbnail_path, filepath): (Option<String>, String) = {
        let conn = lock_db(&db)?;
        conn.query_row(
            "SELECT thumbnail_path, filepath FROM embroidery_files WHERE id = ?1",
            [file_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?
    };

    // If cached thumbnail exists on disk, return it
    if let Some(ref path) = thumbnail_path {
        if !path.is_empty() && std::path::Path::new(path).exists() {
            let data = std::fs::read(path)?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            return Ok(format!("data:image/png;base64,{b64}"));
        }
    }

    // On-demand generation: read the original file and generate a thumbnail
    let src_path = std::path::Path::new(&filepath);
    let ext = src_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if ext.is_empty() {
        return Ok(String::new());
    }

    let raw_data = match std::fs::read(src_path) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("Failed to read source file for thumbnail {}: {e}", filepath);
            return Ok(String::new());
        }
    };

    match thumb_state.0.generate(file_id, &raw_data, &ext) {
        Ok(thumb_path) => {
            // Persist the path in DB for future cache hits
            let conn = lock_db(&db)?;
            let _ = conn.execute(
                "UPDATE embroidery_files SET thumbnail_path = ?2 WHERE id = ?1",
                rusqlite::params![file_id, thumb_path.to_string_lossy().as_ref()],
            );

            let data = std::fs::read(&thumb_path)?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            Ok(format!("data:image/png;base64,{b64}"))
        }
        Err(e) => {
            log::warn!("Failed to generate thumbnail for file {file_id}: {e}");
            Ok(String::new())
        }
    }
}

/// Generate a QR code PNG for the given unique ID string.
#[tauri::command]
pub fn generate_qr_code(unique_id: String) -> Result<Vec<u8>, AppError> {
    use qrcode::QrCode;
    use image::Luma;

    let code = QrCode::new(unique_id.as_bytes()).map_err(|e| {
        AppError::Internal(format!("QR-Code Fehler: {e}"))
    })?;

    let img = code.render::<Luma<u8>>().quiet_zone(false).module_dimensions(4, 4).build();

    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).map_err(|e| {
        AppError::Internal(format!("PNG-Encoding Fehler: {e}"))
    })?;

    Ok(buf.into_inner())
}

/// Attach a file to an embroidery file entry.
#[tauri::command]
pub fn attach_file(
    db: State<'_, DbState>,
    file_id: i64,
    source_path: String,
    attachment_type: String,
) -> Result<FileAttachment, AppError> {
    // Reject path traversal
    if source_path.contains("..") {
        return Err(AppError::Validation("Path traversal not allowed".to_string()));
    }

    let src = std::path::Path::new(&source_path);
    if !src.exists() {
        return Err(AppError::NotFound(format!("Datei nicht gefunden: {source_path}")));
    }

    let filename = src
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment")
        .to_string();

    let mime_type = match src.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("pdf") => Some("application/pdf".to_string()),
        Some("png") => Some("image/png".to_string()),
        Some("jpg" | "jpeg") => Some("image/jpeg".to_string()),
        Some("txt") => Some("text/plain".to_string()),
        _ => None,
    };

    // Determine attachment storage directory
    let conn = lock_db(&db)?;

    let library_root: String = conn
        .query_row("SELECT value FROM settings WHERE key = 'library_root'", [], |row| row.get(0))
        .unwrap_or_else(|_| "~/Stickdateien".to_string());

    let base_dir = if library_root.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&library_root[2..])
        } else {
            std::path::PathBuf::from(&library_root)
        }
    } else {
        std::path::PathBuf::from(&library_root)
    };

    let attach_dir = base_dir.join(".stichman").join("attachments").join(file_id.to_string());
    std::fs::create_dir_all(&attach_dir)?;

    // Deduplicate filename to avoid overwriting existing attachments
    let mut dest = attach_dir.join(&filename);
    if dest.exists() {
        let stem = std::path::Path::new(&filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let ext = std::path::Path::new(&filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let mut counter = 1u32;
        loop {
            let new_name = if ext.is_empty() {
                format!("{stem}_{counter}")
            } else {
                format!("{stem}_{counter}.{ext}")
            };
            dest = attach_dir.join(&new_name);
            if !dest.exists() {
                break;
            }
            counter += 1;
        }
    }
    std::fs::copy(src, &dest)?;

    let dest_str = dest.to_string_lossy().to_string();

    let actual_filename = dest.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&filename)
        .to_string();

    conn.execute(
        "INSERT INTO file_attachments (file_id, filename, mime_type, file_path, attachment_type) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![file_id, actual_filename, mime_type, dest_str, attachment_type],
    )?;

    let id = conn.last_insert_rowid();

    conn.query_row(
        "SELECT id, file_id, filename, mime_type, file_path, attachment_type, created_at \
         FROM file_attachments WHERE id = ?1",
        [id],
        |row| Ok(FileAttachment {
            id: row.get(0)?,
            file_id: row.get(1)?,
            filename: row.get(2)?,
            mime_type: row.get(3)?,
            file_path: row.get(4)?,
            attachment_type: row.get(5)?,
            created_at: row.get(6)?,
        }),
    ).map_err(|e| AppError::Database(e))
}

/// Get all attachments for a file.
#[tauri::command]
pub fn get_attachments(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<Vec<FileAttachment>, AppError> {
    let conn = lock_db(&db)?;

    let mut stmt = conn.prepare(
        "SELECT id, file_id, filename, mime_type, file_path, attachment_type, created_at \
         FROM file_attachments WHERE file_id = ?1 ORDER BY created_at",
    )?;
    let attachments = stmt
        .query_map([file_id], |row| {
            Ok(FileAttachment {
                id: row.get(0)?,
                file_id: row.get(1)?,
                filename: row.get(2)?,
                mime_type: row.get(3)?,
                file_path: row.get(4)?,
                attachment_type: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(attachments)
}

/// Delete an attachment (DB record + file on disk).
#[tauri::command]
pub fn delete_attachment(
    db: State<'_, DbState>,
    attachment_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    let file_path: String = conn
        .query_row(
            "SELECT file_path FROM file_attachments WHERE id = ?1",
            [attachment_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Anhang {attachment_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?;

    conn.execute("DELETE FROM file_attachments WHERE id = ?1", [attachment_id])?;

    // Best-effort file deletion
    if let Err(e) = std::fs::remove_file(&file_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            log::warn!("Failed to remove attachment file {file_path}: {e}");
        }
    }

    Ok(())
}

/// Open an attachment with the system default application.
#[tauri::command]
pub fn open_attachment(
    db: State<'_, DbState>,
    attachment_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    let file_path: String = conn
        .query_row(
            "SELECT file_path FROM file_attachments WHERE id = ?1",
            [attachment_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Anhang {attachment_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?;

    drop(conn);

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&file_path)
            .spawn()
            .map_err(|e| AppError::Internal(format!("Fehler beim Öffnen: {e}")))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&file_path)
            .spawn()
            .map_err(|e| AppError::Internal(format!("Fehler beim Öffnen: {e}")))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&file_path)
            .spawn()
            .map_err(|e| AppError::Internal(format!("Fehler beim Öffnen: {e}")))?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        return Err(AppError::Internal("Plattform nicht unterstützt".to_string()));
    }
    Ok(())
}

/// Get attachment count for a file.
#[tauri::command]
pub fn get_attachment_count(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<i64, AppError> {
    let conn = lock_db(&db)?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM file_attachments WHERE file_id = ?1",
        [file_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Get attachment counts for multiple files in a single query.
#[tauri::command]
pub fn get_attachment_counts(
    db: State<'_, DbState>,
    file_ids: Vec<i64>,
) -> Result<std::collections::HashMap<i64, i64>, AppError> {
    let conn = lock_db(&db)?;
    let mut result = std::collections::HashMap::new();
    if file_ids.is_empty() {
        return Ok(result);
    }
    let placeholders: Vec<String> = file_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT file_id, COUNT(*) FROM file_attachments WHERE file_id IN ({}) GROUP BY file_id",
        placeholders.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<Box<dyn rusqlite::types::ToSql>> = file_ids.iter().map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>).collect();
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (file_id, count) = row?;
        result.insert(file_id, count);
    }
    Ok(result)
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

    // ── SearchParams tests ──────────────────────────────────────────

    use crate::db::models::SearchParams;

    /// Thin wrapper around the production query builder for test use.
    fn query_files(
        conn: &rusqlite::Connection,
        folder_id: Option<i64>,
        search: Option<String>,
        format_filter: Option<String>,
        search_params: Option<SearchParams>,
    ) -> Vec<crate::db::models::EmbroideryFile> {
        super::query_files_impl(conn, folder_id, search, format_filter, search_params).unwrap()
    }

    /// Seed helper: inserts a folder and returns its id.
    fn seed_folder(conn: &rusqlite::Connection) -> i64 {
        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn test_search_text_across_multiple_fields() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        // File 1: "Flowers" only in theme
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, theme) \
             VALUES (?1, 'a.pes', '/test/a.pes', 'Flowers')",
            [fid],
        ).unwrap();

        // File 2: "Flowers" only in description
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, description) \
             VALUES (?1, 'b.pes', '/test/b.pes', 'Beautiful Flowers')",
            [fid],
        ).unwrap();

        // File 3: no match
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, theme) \
             VALUES (?1, 'c.pes', '/test/c.pes', 'Stars')",
            [fid],
        ).unwrap();

        let results = query_files(&conn, None, None, None, Some(SearchParams {
            text: Some("Flowers".into()),
            ..Default::default()
        }));
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|f| f.filename != "c.pes"));
    }

    #[test]
    fn test_search_text_in_author_and_keywords() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, author) \
             VALUES (?1, 'a.pes', '/test/a.pes', 'Maria')",
            [fid],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, keywords) \
             VALUES (?1, 'b.pes', '/test/b.pes', 'maria, rose')",
            [fid],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, name) \
             VALUES (?1, 'c.pes', '/test/c.pes', 'Unrelated')",
            [fid],
        ).unwrap();

        let results = query_files(&conn, None, None, None, Some(SearchParams {
            text: Some("Maria".into()),
            ..Default::default()
        }));
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_by_tag_filter() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) \
             VALUES (?1, 'a.pes', '/test/a.pes')",
            [fid],
        ).unwrap();
        let file_a = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) \
             VALUES (?1, 'b.pes', '/test/b.pes')",
            [fid],
        ).unwrap();
        let file_b = conn.last_insert_rowid();

        // Create tags
        conn.execute("INSERT INTO tags (name) VALUES ('floral')", []).unwrap();
        let tag_floral = conn.last_insert_rowid();
        conn.execute("INSERT INTO tags (name) VALUES ('nature')", []).unwrap();
        let tag_nature = conn.last_insert_rowid();

        // File A has both tags, File B has only 'nature'
        conn.execute("INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![file_a, tag_floral]).unwrap();
        conn.execute("INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![file_a, tag_nature]).unwrap();
        conn.execute("INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![file_b, tag_nature]).unwrap();

        // Filter by 'floral' only → file A
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            tags: Some(vec!["floral".into()]),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "a.pes");

        // Filter by both tags (AND logic) → only file A has both
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            tags: Some(vec!["floral".into(), "nature".into()]),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "a.pes");

        // Filter by 'nature' only → both files
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            tags: Some(vec!["nature".into()]),
            ..Default::default()
        }));
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_numeric_range_stitch_count() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, stitch_count) \
             VALUES (?1, 'small.pes', '/test/small.pes', 1000)",
            [fid],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, stitch_count) \
             VALUES (?1, 'medium.pes', '/test/medium.pes', 5000)",
            [fid],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, stitch_count) \
             VALUES (?1, 'large.pes', '/test/large.pes', 20000)",
            [fid],
        ).unwrap();

        // Min only
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            stitch_count_min: Some(5000),
            ..Default::default()
        }));
        assert_eq!(results.len(), 2);

        // Max only
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            stitch_count_max: Some(5000),
            ..Default::default()
        }));
        assert_eq!(results.len(), 2);

        // Range: 2000..=10000
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            stitch_count_min: Some(2000),
            stitch_count_max: Some(10000),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "medium.pes");
    }

    #[test]
    fn test_search_numeric_range_dimensions() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, width_mm, height_mm) \
             VALUES (?1, 'a.pes', '/test/a.pes', 50.0, 80.0)",
            [fid],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, width_mm, height_mm) \
             VALUES (?1, 'b.pes', '/test/b.pes', 120.0, 200.0)",
            [fid],
        ).unwrap();

        let results = query_files(&conn, None, None, None, Some(SearchParams {
            width_mm_max: Some(100.0),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "a.pes");

        let results = query_files(&conn, None, None, None, Some(SearchParams {
            height_mm_min: Some(100.0),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "b.pes");
    }

    #[test]
    fn test_search_boolean_ai_analyzed() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, ai_analyzed) \
             VALUES (?1, 'analyzed.pes', '/test/analyzed.pes', 1)",
            [fid],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) \
             VALUES (?1, 'not_analyzed.pes', '/test/not_analyzed.pes')",
            [fid],
        ).unwrap();

        let results = query_files(&conn, None, None, None, Some(SearchParams {
            ai_analyzed: Some(true),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "analyzed.pes");

        let results = query_files(&conn, None, None, None, Some(SearchParams {
            ai_analyzed: Some(false),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "not_analyzed.pes");
    }

    #[test]
    fn test_search_boolean_ai_confirmed() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, ai_analyzed, ai_confirmed) \
             VALUES (?1, 'confirmed.pes', '/test/confirmed.pes', 1, 1)",
            [fid],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, ai_analyzed, ai_confirmed) \
             VALUES (?1, 'pending.pes', '/test/pending.pes', 1, 0)",
            [fid],
        ).unwrap();

        let results = query_files(&conn, None, None, None, Some(SearchParams {
            ai_confirmed: Some(true),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "confirmed.pes");
    }

    #[test]
    fn test_search_color_brand() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) \
             VALUES (?1, 'a.pes', '/test/a.pes')",
            [fid],
        ).unwrap();
        let file_a = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) \
             VALUES (?1, 'b.pes', '/test/b.pes')",
            [fid],
        ).unwrap();

        // Add thread color to file A
        conn.execute(
            "INSERT INTO file_thread_colors (file_id, sort_order, color_hex, color_name, brand) \
             VALUES (?1, 1, '#FF0000', 'Red', 'Madeira')",
            [file_a],
        ).unwrap();

        // Search by color name
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            color_search: Some("Red".into()),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "a.pes");

        // Search by brand
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            color_search: Some("Madeira".into()),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "a.pes");

        // No match
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            color_search: Some("Isacord".into()),
            ..Default::default()
        }));
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_combined_filters() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        // File A: theme=Flowers, stitch_count=5000, ai_analyzed=true
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, theme, stitch_count, ai_analyzed) \
             VALUES (?1, 'a.pes', '/test/a.pes', 'Flowers', 5000, 1)",
            [fid],
        ).unwrap();
        let file_a = conn.last_insert_rowid();
        conn.execute("INSERT INTO tags (name) VALUES ('floral')", []).unwrap();
        let tag_id = conn.last_insert_rowid();
        conn.execute("INSERT INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![file_a, tag_id]).unwrap();

        // File B: theme=Flowers, stitch_count=500, ai_analyzed=false
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, theme, stitch_count) \
             VALUES (?1, 'b.pes', '/test/b.pes', 'Flowers', 500)",
            [fid],
        ).unwrap();

        // File C: theme=Stars, stitch_count=8000, ai_analyzed=true
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, theme, stitch_count, ai_analyzed) \
             VALUES (?1, 'c.pes', '/test/c.pes', 'Stars', 8000, 1)",
            [fid],
        ).unwrap();

        // Combine text + stitch range + boolean: only file A matches all three
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            text: Some("Flowers".into()),
            stitch_count_min: Some(1000),
            ai_analyzed: Some(true),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "a.pes");

        // Combine text + tag: only file A has both "Flowers" text and "floral" tag
        let results = query_files(&conn, None, None, None, Some(SearchParams {
            text: Some("Flowers".into()),
            tags: Some(vec!["floral".into()]),
            ..Default::default()
        }));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "a.pes");
    }

    #[test]
    fn test_search_empty_params_returns_all() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/test/a.pes')",
            [fid],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'b.pes', '/test/b.pes')",
            [fid],
        ).unwrap();

        // Default (empty) SearchParams should return all files
        let results = query_files(&conn, None, None, None, Some(SearchParams::default()));
        assert_eq!(results.len(), 2);

        // None search_params should also return all
        let results = query_files(&conn, None, None, None, None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_legacy_search_still_works() {
        let conn = init_database_in_memory().unwrap();
        let fid = seed_folder(&conn);

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, name) \
             VALUES (?1, 'rose.pes', '/test/rose.pes', 'Rose Design')",
            [fid],
        ).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, name) \
             VALUES (?1, 'star.dst', '/test/star.dst', 'Star Pattern')",
            [fid],
        ).unwrap();

        // Legacy search param (no SearchParams struct)
        let results = query_files(&conn, None, Some("Rose".into()), None, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "rose.pes");
    }
}
