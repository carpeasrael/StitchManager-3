use serde::Serialize;
use tauri::State;

use crate::DbState;
use crate::error::{lock_db, AppError};

const MAX_VERSIONS_PER_FILE: i64 = 10;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileVersion {
    pub id: i64,
    pub file_id: i64,
    pub version_number: i32,
    pub file_size: i64,
    pub operation: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// Create a version snapshot of a file before modification.
/// Called automatically before edits and conversions.
pub fn create_version_snapshot(
    conn: &rusqlite::Connection,
    file_id: i64,
    operation: &str,
    description: Option<&str>,
) -> Result<(), AppError> {
    // Read the current file data from disk
    let filepath: String = conn.query_row(
        "SELECT filepath FROM embroidery_files WHERE id = ?1",
        [file_id],
        |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
        }
        other => AppError::Database(other),
    })?;

    // Skip versioning for very large files (>10MB) to avoid memory pressure
    const MAX_VERSION_SIZE: u64 = 10 * 1024 * 1024;
    if let Ok(meta) = std::fs::metadata(&filepath) {
        if meta.len() > MAX_VERSION_SIZE {
            log::info!("Skipping version for large file ({} bytes): {filepath}", meta.len());
            return Ok(());
        }
    }

    let file_data = match std::fs::read(&filepath) {
        Ok(data) => data,
        Err(e) => {
            log::warn!("Could not read file for versioning {filepath}: {e}");
            return Ok(()); // Non-fatal: skip versioning if file can't be read
        }
    };

    let file_size = file_data.len() as i64;

    // Get next version number
    let next_version: i32 = conn.query_row(
        "SELECT COALESCE(MAX(version_number), 0) + 1 FROM file_versions WHERE file_id = ?1",
        [file_id],
        |row| row.get(0),
    )?;

    // Insert version
    conn.execute(
        "INSERT INTO file_versions (file_id, version_number, file_data, file_size, operation, description) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![file_id, next_version, file_data, file_size, operation, description],
    )?;

    // Prune old versions beyond the limit (keep newest MAX_VERSIONS_PER_FILE)
    conn.execute(
        "DELETE FROM file_versions WHERE file_id = ?1 AND id NOT IN \
         (SELECT id FROM file_versions WHERE file_id = ?1 ORDER BY version_number DESC LIMIT ?2)",
        rusqlite::params![file_id, MAX_VERSIONS_PER_FILE],
    )?;

    Ok(())
}

#[tauri::command]
pub fn get_file_versions(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<Vec<FileVersion>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, file_id, version_number, file_size, operation, description, created_at \
         FROM file_versions WHERE file_id = ?1 ORDER BY version_number DESC"
    )?;
    let versions = stmt
        .query_map([file_id], |row| {
            Ok(FileVersion {
                id: row.get(0)?,
                file_id: row.get(1)?,
                version_number: row.get(2)?,
                file_size: row.get(3)?,
                operation: row.get(4)?,
                description: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(versions)
}

#[tauri::command]
pub fn restore_version(
    db: State<'_, DbState>,
    file_id: i64,
    version_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    // Get the version data and the target filepath
    let (file_data, filepath): (Vec<u8>, String) = conn.query_row(
        "SELECT fv.file_data, ef.filepath FROM file_versions fv \
         JOIN embroidery_files ef ON ef.id = fv.file_id \
         WHERE fv.id = ?1 AND fv.file_id = ?2",
        rusqlite::params![version_id, file_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound("Version nicht gefunden".into())
        }
        other => AppError::Database(other),
    })?;

    // Save current state before restoring (skip only if the most recent version is already a restore)
    let recent_restore: bool = conn.query_row(
        "SELECT operation FROM file_versions WHERE file_id = ?1 \
         ORDER BY version_number DESC LIMIT 1",
        [file_id],
        |row| {
            let op: String = row.get(0)?;
            Ok(op == "restore")
        },
    ).unwrap_or(false);
    if !recent_restore {
        let _ = create_version_snapshot(&conn, file_id, "restore", Some("Vor Wiederherstellung"));
    }

    // Write the version data to disk
    std::fs::write(&filepath, &file_data)?;

    // Update the file's updated_at timestamp
    conn.execute(
        "UPDATE embroidery_files SET updated_at = datetime('now') WHERE id = ?1",
        [file_id],
    )?;

    Ok(())
}

#[tauri::command]
pub fn delete_version(
    db: State<'_, DbState>,
    version_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let affected = conn.execute("DELETE FROM file_versions WHERE id = ?1", [version_id])?;
    if affected == 0 {
        return Err(AppError::NotFound("Version nicht gefunden".into()));
    }
    Ok(())
}

#[tauri::command]
pub fn export_version(
    db: State<'_, DbState>,
    version_id: i64,
    path: String,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let file_data: Vec<u8> = conn.query_row(
        "SELECT file_data FROM file_versions WHERE id = ?1",
        [version_id],
        |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound("Version nicht gefunden".into())
        }
        other => AppError::Database(other),
    })?;

    // Prevent overwrite of existing files
    let p = std::path::Path::new(&path);
    if p.exists() {
        return Err(AppError::Validation(format!("Datei existiert bereits: {path}")));
    }
    // Block path traversal
    super::validate_no_traversal(&path)?;
    std::fs::write(p, &file_data)?;
    Ok(())
}
