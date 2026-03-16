use std::io::{Read as _, Write as _};
use std::path::Path;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::error::{lock_db, AppError};
use crate::DbState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u32,
}

/// Create a backup ZIP containing the database and optionally all referenced files.
#[tauri::command]
pub async fn create_backup(
    db: State<'_, DbState>,
    app_handle: AppHandle,
    include_files: bool,
) -> Result<BackupResult, AppError> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| AppError::Internal(format!("App-Datenverzeichnis nicht gefunden: {e}")))?;
    let backup_dir = app_data_dir.join("backups");
    std::fs::create_dir_all(&backup_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let zip_name = format!("stichman_backup_{timestamp}.zip");
    let zip_path = backup_dir.join(&zip_name);

    // Create a temporary copy of the DB to avoid locking issues
    let temp_db = backup_dir.join("_temp_backup.db");
    {
        let conn = lock_db(&db)?;
        conn.execute("VACUUM INTO ?1", [temp_db.to_string_lossy().as_ref()])?;
    }

    let zip_file = std::fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut file_count: u32 = 1; // DB file

    // Add database
    let db_data = std::fs::read(&temp_db)?;
    zip.start_file("stitch_manager.db", options)
        .map_err(|e| AppError::Internal(format!("ZIP-Fehler: {e}")))?;
    zip.write_all(&db_data)?;

    // Clean up temp DB
    let _ = std::fs::remove_file(&temp_db);

    // Add manifest
    let manifest = serde_json::json!({
        "version": "1.0",
        "created_at": chrono::Local::now().to_rfc3339(),
        "app_version": env!("CARGO_PKG_VERSION"),
        "include_files": include_files,
    });
    zip.start_file("manifest.json", options)
        .map_err(|e| AppError::Internal(format!("ZIP-Fehler: {e}")))?;
    zip.write_all(manifest.to_string().as_bytes())?;
    file_count += 1;

    // Optionally include referenced files
    if include_files {
        let conn = lock_db(&db)?;
        let mut stmt = conn.prepare(
            "SELECT filepath FROM embroidery_files WHERE filepath IS NOT NULL AND filepath != ''"
        )?;
        let paths: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        drop(conn);

        for filepath in &paths {
            let path = Path::new(filepath);
            if path.exists() && path.is_file() {
                if let Ok(data) = std::fs::read(path) {
                    let entry_name = format!("files/{}", path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown"));
                    if zip.start_file(&entry_name, options).is_ok() {
                        let _ = zip.write_all(&data);
                        file_count += 1;
                    }
                }
            }
        }

        // Include thumbnails
        let thumb_dir = app_data_dir.join("thumbnails");
        if thumb_dir.exists() {
            for entry in std::fs::read_dir(&thumb_dir)? {
                if let Ok(entry) = entry {
                    if entry.path().is_file() {
                        if let Ok(data) = std::fs::read(entry.path()) {
                            let name = format!("thumbnails/{}", entry.file_name().to_string_lossy());
                            if zip.start_file(&name, options).is_ok() {
                                let _ = zip.write_all(&data);
                                file_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    zip.finish().map_err(|e| AppError::Internal(format!("ZIP-Fehler: {e}")))?;

    let size = std::fs::metadata(&zip_path)?.len();

    Ok(BackupResult {
        path: zip_path.to_string_lossy().to_string(),
        size_bytes: size,
        file_count,
    })
}

/// Restore from a backup ZIP. Returns the number of records restored.
#[tauri::command]
pub async fn restore_backup(
    app_handle: AppHandle,
    backup_path: String,
) -> Result<String, AppError> {
    super::validate_no_traversal(&backup_path)?;
    let zip_path = Path::new(&backup_path);
    if !zip_path.exists() {
        return Err(AppError::NotFound("Backup-Datei nicht gefunden".into()));
    }

    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| AppError::Internal(format!("App-Datenverzeichnis nicht gefunden: {e}")))?;

    // Read and validate manifest
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Internal(format!("Ungueltige ZIP-Datei: {e}")))?;

    let has_manifest = archive.by_name("manifest.json").is_ok();
    if !has_manifest {
        return Err(AppError::Validation("Keine gueltige Backup-Datei (manifest.json fehlt)".into()));
    }

    let has_db = archive.by_name("stitch_manager.db").is_ok();
    if !has_db {
        return Err(AppError::Validation("Keine Datenbank in der Backup-Datei".into()));
    }

    // Extract database
    let db_target = app_data_dir.join("stitch_manager.db");

    // Safety backup of current DB
    let safety_backup = app_data_dir.join("stitch_manager_pre_restore.db");
    if db_target.exists() {
        std::fs::copy(&db_target, &safety_backup)?;
    }

    // Extract DB from ZIP
    {
        let mut db_entry = archive.by_name("stitch_manager.db")
            .map_err(|e| AppError::Internal(format!("DB-Extraktion fehlgeschlagen: {e}")))?;
        let mut db_data = Vec::new();
        db_entry.read_to_end(&mut db_data)?;
        std::fs::write(&db_target, &db_data)?;
    }

    // Extract thumbnails if present
    let thumb_dir = app_data_dir.join("thumbnails");
    std::fs::create_dir_all(&thumb_dir)?;

    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Internal(format!("ZIP erneut oeffnen fehlgeschlagen: {e}")))?;

    for i in 0..archive.len() {
        if let Ok(mut entry) = archive.by_index(i) {
            let name = entry.name().to_string();
            // Validate entry name to prevent path traversal
            if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
                log::warn!("Skipping suspicious ZIP entry: {name}");
                continue;
            }
            if name.starts_with("thumbnails/") && !name.ends_with('/') {
                let filename = name.strip_prefix("thumbnails/").unwrap_or(&name);
                let target = thumb_dir.join(filename);
                let mut data = Vec::new();
                let _ = entry.read_to_end(&mut data);
                let _ = std::fs::write(&target, &data);
            }
        }
    }

    Ok("Backup erfolgreich wiederhergestellt. Bitte App neu starten.".into())
}

/// Check for missing files in the database.
#[tauri::command]
pub fn check_missing_files(
    db: State<'_, DbState>,
) -> Result<Vec<(i64, String)>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, filepath FROM embroidery_files \
         WHERE filepath IS NOT NULL AND filepath != '' AND deleted_at IS NULL"
    )?;
    let missing: Vec<(i64, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .filter(|(_, path)| !Path::new(path).exists())
        .collect();
    Ok(missing)
}

/// Re-link a single file to a new path.
#[tauri::command]
pub fn relink_file(
    db: State<'_, DbState>,
    file_id: i64,
    new_path: String,
) -> Result<(), AppError> {
    super::validate_no_traversal(&new_path)?;
    if !Path::new(&new_path).exists() {
        return Err(AppError::NotFound(format!("Neue Datei nicht gefunden: {new_path}")));
    }
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE embroidery_files SET filepath = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![new_path, file_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Datei {file_id} nicht gefunden")));
    }
    Ok(())
}

/// Batch re-link files by replacing a path prefix.
#[tauri::command]
pub fn relink_batch(
    db: State<'_, DbState>,
    old_prefix: String,
    new_prefix: String,
) -> Result<u32, AppError> {
    super::validate_no_traversal(&new_prefix)?;
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, filepath FROM embroidery_files WHERE filepath LIKE ?1"
    )?;
    let like_pattern = format!("{}%", old_prefix);
    let files: Vec<(i64, String)> = stmt
        .query_map([&like_pattern], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut count: u32 = 0;
    for (id, old_path) in &files {
        let new_path = old_path.replacen(&old_prefix, &new_prefix, 1);
        if Path::new(&new_path).exists() {
            conn.execute(
                "UPDATE embroidery_files SET filepath = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![new_path, id],
            )?;
            count += 1;
        }
    }

    Ok(count)
}

/// Export selected files as JSON.
#[tauri::command]
pub fn export_metadata_json(
    db: State<'_, DbState>,
    file_ids: Vec<i64>,
) -> Result<String, AppError> {
    let conn = lock_db(&db)?;
    let mut records = Vec::new();

    for file_id in &file_ids {
        let file: Option<serde_json::Value> = conn.query_row(
            "SELECT id, filename, filepath, name, theme, description, license, \
             width_mm, height_mm, stitch_count, color_count, file_type, \
             size_range, skill_level, language, format_type, file_source, \
             purchase_link, status, unique_id, category, author, keywords, comments \
             FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
            [file_id],
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "filename": row.get::<_, String>(1)?,
                    "filepath": row.get::<_, String>(2)?,
                    "name": row.get::<_, Option<String>>(3)?,
                    "theme": row.get::<_, Option<String>>(4)?,
                    "description": row.get::<_, Option<String>>(5)?,
                    "license": row.get::<_, Option<String>>(6)?,
                    "widthMm": row.get::<_, Option<f64>>(7)?,
                    "heightMm": row.get::<_, Option<f64>>(8)?,
                    "stitchCount": row.get::<_, Option<i32>>(9)?,
                    "colorCount": row.get::<_, Option<i32>>(10)?,
                    "fileType": row.get::<_, String>(11)?,
                    "sizeRange": row.get::<_, Option<String>>(12)?,
                    "skillLevel": row.get::<_, Option<String>>(13)?,
                    "language": row.get::<_, Option<String>>(14)?,
                    "formatType": row.get::<_, Option<String>>(15)?,
                    "fileSource": row.get::<_, Option<String>>(16)?,
                    "purchaseLink": row.get::<_, Option<String>>(17)?,
                    "status": row.get::<_, String>(18)?,
                    "uniqueId": row.get::<_, Option<String>>(19)?,
                    "category": row.get::<_, Option<String>>(20)?,
                    "author": row.get::<_, Option<String>>(21)?,
                    "keywords": row.get::<_, Option<String>>(22)?,
                    "comments": row.get::<_, Option<String>>(23)?,
                }))
            },
        ).ok();

        if let Some(record) = file {
            records.push(record);
        }
    }

    let export = serde_json::json!({
        "version": "1.0",
        "exported_at": chrono::Local::now().to_rfc3339(),
        "records": records,
    });

    Ok(serde_json::to_string_pretty(&export)
        .map_err(|e| AppError::Internal(format!("JSON-Serialisierung fehlgeschlagen: {e}")))?)
}

/// Export selected files as CSV.
#[tauri::command]
pub fn export_metadata_csv(
    db: State<'_, DbState>,
    file_ids: Vec<i64>,
) -> Result<String, AppError> {
    let conn = lock_db(&db)?;
    let mut wtr = csv::Writer::from_writer(Vec::new());

    // Header
    wtr.write_record([
        "id", "filename", "name", "theme", "file_type", "status",
        "size_range", "skill_level", "language", "category", "author",
    ]).map_err(|e| AppError::Internal(format!("CSV-Fehler: {e}")))?;

    for file_id in &file_ids {
        let row: Option<Vec<String>> = conn.query_row(
            "SELECT id, filename, name, theme, file_type, status, \
             size_range, skill_level, language, category, author \
             FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
            [file_id],
            |row| {
                Ok(vec![
                    row.get::<_, i64>(0)?.to_string(),
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(8)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(9)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                ])
            },
        ).ok();

        if let Some(fields) = row {
            wtr.write_record(&fields)
                .map_err(|e| AppError::Internal(format!("CSV-Fehler: {e}")))?;
        }
    }

    let data = wtr.into_inner()
        .map_err(|e| AppError::Internal(format!("CSV-Fehler: {e}")))?;
    String::from_utf8(data)
        .map_err(|e| AppError::Internal(format!("UTF-8-Fehler: {e}")))
}

/// Soft-delete a file (move to trash).
#[tauri::command]
pub fn soft_delete_file(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE embroidery_files SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [file_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Datei {file_id} nicht gefunden oder bereits geloescht")));
    }
    Ok(())
}

/// Restore a soft-deleted file from trash.
#[tauri::command]
pub fn restore_file(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE embroidery_files SET deleted_at = NULL WHERE id = ?1 AND deleted_at IS NOT NULL",
        [file_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Datei {file_id} nicht im Papierkorb")));
    }
    Ok(())
}

/// Get all soft-deleted files (trash contents).
#[tauri::command]
pub fn get_trash(
    db: State<'_, DbState>,
) -> Result<Vec<(i64, String, String)>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, filename, deleted_at FROM embroidery_files WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC"
    )?;
    let items = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

/// Permanently delete a trashed file.
#[tauri::command]
pub fn purge_file(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "DELETE FROM embroidery_files WHERE id = ?1 AND deleted_at IS NOT NULL",
        [file_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Datei {file_id} nicht im Papierkorb")));
    }
    Ok(())
}

/// Auto-purge trash items older than the configured retention period.
#[tauri::command]
pub fn auto_purge_trash(
    db: State<'_, DbState>,
) -> Result<u32, AppError> {
    let conn = lock_db(&db)?;

    // Get retention days from settings (default 30)
    let retention_days: i64 = conn.query_row(
        "SELECT value FROM settings WHERE key = 'trash_retention_days'",
        [],
        |row| row.get::<_, String>(0),
    ).ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let threshold = format!("-{retention_days} days");
    let deleted = conn.execute(
        "DELETE FROM embroidery_files WHERE deleted_at IS NOT NULL \
         AND deleted_at < datetime('now', ?1)",
        rusqlite::params![threshold],
    )?;

    if deleted > 0 {
        log::info!("Auto-purge: {deleted} Dateien endgueltig geloescht (aelter als {retention_days} Tage)");
    }

    Ok(deleted as u32)
}

/// Archive a file (set status to 'archived').
#[tauri::command]
pub fn archive_file(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE embroidery_files SET status = 'archived', updated_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [file_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Datei {file_id} nicht gefunden")));
    }
    Ok(())
}

/// Unarchive a file (set status back to 'none').
#[tauri::command]
pub fn unarchive_file(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE embroidery_files SET status = 'none', updated_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        [file_id],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Datei {file_id} nicht gefunden")));
    }
    Ok(())
}

/// Import metadata from a JSON export (merge by unique_id or insert new).
#[tauri::command]
pub fn import_metadata_json(
    db: State<'_, DbState>,
    json_data: String,
) -> Result<u32, AppError> {
    let parsed: serde_json::Value = serde_json::from_str(&json_data)
        .map_err(|e| AppError::Validation(format!("Ungueltige JSON-Daten: {e}")))?;

    let records = parsed.get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| AppError::Validation("JSON muss ein 'records' Array enthalten".into()))?;

    let conn = lock_db(&db)?;
    let mut imported: u32 = 0;

    for record in records {
        let unique_id = record.get("uniqueId").and_then(|v| v.as_str());
        let name = record.get("name").and_then(|v| v.as_str());
        let theme = record.get("theme").and_then(|v| v.as_str());
        let description = record.get("description").and_then(|v| v.as_str());
        let status = record.get("status").and_then(|v| v.as_str()).unwrap_or("none");
        let category = record.get("category").and_then(|v| v.as_str());
        let author = record.get("author").and_then(|v| v.as_str());
        let keywords = record.get("keywords").and_then(|v| v.as_str());

        if let Some(uid) = unique_id {
            // Try to merge by unique_id
            let existing: Option<i64> = conn.query_row(
                "SELECT id FROM embroidery_files WHERE unique_id = ?1",
                [uid],
                |row| row.get(0),
            ).ok();

            if let Some(id) = existing {
                conn.execute(
                    "UPDATE embroidery_files SET \
                     name = COALESCE(?2, name), theme = COALESCE(?3, theme), \
                     description = COALESCE(?4, description), status = ?5, \
                     category = COALESCE(?6, category), author = COALESCE(?7, author), \
                     keywords = COALESCE(?8, keywords), updated_at = datetime('now') \
                     WHERE id = ?1",
                    rusqlite::params![id, name, theme, description, status, category, author, keywords],
                )?;
                imported += 1;
            }
        }
    }

    Ok(imported)
}

/// Bulk archive multiple files.
#[tauri::command]
pub fn archive_files_batch(
    db: State<'_, DbState>,
    file_ids: Vec<i64>,
) -> Result<u32, AppError> {
    let conn = lock_db(&db)?;
    let mut count: u32 = 0;
    for id in &file_ids {
        let changes = conn.execute(
            "UPDATE embroidery_files SET status = 'archived', updated_at = datetime('now') \
             WHERE id = ?1 AND deleted_at IS NULL AND status != 'archived'",
            [id],
        )?;
        count += changes as u32;
    }
    Ok(count)
}

/// Bulk unarchive multiple files.
#[tauri::command]
pub fn unarchive_files_batch(
    db: State<'_, DbState>,
    file_ids: Vec<i64>,
) -> Result<u32, AppError> {
    let conn = lock_db(&db)?;
    let mut count: u32 = 0;
    for id in &file_ids {
        let changes = conn.execute(
            "UPDATE embroidery_files SET status = 'none', updated_at = datetime('now') \
             WHERE id = ?1 AND deleted_at IS NULL AND status = 'archived'",
            [id],
        )?;
        count += changes as u32;
    }
    Ok(count)
}

/// Export library as a portable package with relative paths.
#[tauri::command]
pub fn export_library(
    db: State<'_, DbState>,
    app_handle: AppHandle,
) -> Result<String, AppError> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| AppError::Internal(format!("App-Datenverzeichnis nicht gefunden: {e}")))?;

    let conn = lock_db(&db)?;

    // Get library_root for relative path computation
    let library_root: Option<String> = conn.query_row(
        "SELECT value FROM settings WHERE key = 'library_root'",
        [],
        |row| row.get(0),
    ).ok();

    let root = library_root.unwrap_or_default();

    // Export all file records with relative paths
    let mut stmt = conn.prepare(
        "SELECT id, filename, filepath, name, theme, description, file_type, status, unique_id \
         FROM embroidery_files WHERE deleted_at IS NULL"
    )?;
    let records: Vec<serde_json::Value> = stmt.query_map([], |row| {
        let filepath: String = row.get(2)?;
        let relative = if !root.is_empty() && filepath.starts_with(&root) {
            filepath[root.len()..].trim_start_matches('/').trim_start_matches('\\').to_string()
        } else {
            filepath.clone()
        };
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "filename": row.get::<_, String>(1)?,
            "relativePath": relative,
            "name": row.get::<_, Option<String>>(3)?,
            "theme": row.get::<_, Option<String>>(4)?,
            "description": row.get::<_, Option<String>>(5)?,
            "fileType": row.get::<_, String>(6)?,
            "status": row.get::<_, String>(7)?,
            "uniqueId": row.get::<_, Option<String>>(8)?,
        }))
    })?.filter_map(|r| r.ok()).collect();

    let export = serde_json::json!({
        "version": "1.0",
        "type": "library_export",
        "library_root": root,
        "exported_at": chrono::Local::now().to_rfc3339(),
        "records": records,
    });

    let export_path = app_data_dir.join("backups").join(
        format!("library_export_{}.json", chrono::Local::now().format("%Y%m%d_%H%M%S"))
    );
    std::fs::create_dir_all(export_path.parent().unwrap())?;
    std::fs::write(&export_path, serde_json::to_string_pretty(&export)
        .map_err(|e| AppError::Internal(format!("JSON-Fehler: {e}")))?)?;

    Ok(export_path.to_string_lossy().to_string())
}

/// Import library from a portable export, remapping paths to a new root.
#[tauri::command]
pub fn import_library(
    db: State<'_, DbState>,
    json_path: String,
    new_library_root: String,
) -> Result<u32, AppError> {
    super::validate_no_traversal(&json_path)?;
    let data = std::fs::read_to_string(&json_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| AppError::Validation(format!("Ungueltige Export-Datei: {e}")))?;

    let records = parsed.get("records")
        .and_then(|r| r.as_array())
        .ok_or_else(|| AppError::Validation("Export muss 'records' enthalten".into()))?;

    let conn = lock_db(&db)?;
    let mut imported: u32 = 0;
    let root = new_library_root.trim_end_matches('/').trim_end_matches('\\');

    for record in records {
        let rel_path = record.get("relativePath").and_then(|v| v.as_str()).unwrap_or("");
        let abs_path = format!("{}/{}", root, rel_path);
        let filename = record.get("filename").and_then(|v| v.as_str()).unwrap_or("unknown");
        let unique_id = record.get("uniqueId").and_then(|v| v.as_str());

        // Skip if already exists by unique_id
        if let Some(uid) = unique_id {
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM embroidery_files WHERE unique_id = ?1",
                [uid],
                |row| row.get(0),
            ).unwrap_or(false);
            if exists { continue; }
        }

        // Find or create a folder for the file
        let folder_id: i64 = conn.query_row(
            "SELECT id FROM folders LIMIT 1",
            [],
            |row| row.get(0),
        ).unwrap_or(1);

        conn.execute(
            "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, unique_id, file_type, status) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                folder_id,
                filename,
                abs_path,
                unique_id,
                record.get("fileType").and_then(|v| v.as_str()).unwrap_or("embroidery"),
                record.get("status").and_then(|v| v.as_str()).unwrap_or("none"),
            ],
        )?;
        imported += 1;
    }

    Ok(imported)
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_soft_delete_and_restore() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO folders (name, path) VALUES ('T', '/t')", []).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (1, 'a.pes', '/a.pes')",
            [],
        ).unwrap();

        // Soft delete
        conn.execute(
            "UPDATE embroidery_files SET deleted_at = datetime('now') WHERE id = 1",
            [],
        ).unwrap();

        // Should not appear in normal queries
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 0);

        // Should appear in trash
        let trash_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NOT NULL", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(trash_count, 1);

        // Restore
        conn.execute(
            "UPDATE embroidery_files SET deleted_at = NULL WHERE id = 1", [],
        ).unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_archive_status() {
        let conn = init_database_in_memory().unwrap();
        conn.execute("INSERT INTO folders (name, path) VALUES ('T', '/t')", []).unwrap();
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (1, 'b.pdf', '/b.pdf')",
            [],
        ).unwrap();

        conn.execute(
            "UPDATE embroidery_files SET status = 'archived' WHERE id = 1", [],
        ).unwrap();

        let status: String = conn.query_row(
            "SELECT status FROM embroidery_files WHERE id = 1", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "archived");

        // Excluded from default view
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM embroidery_files WHERE status != 'archived' AND deleted_at IS NULL",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }
}
