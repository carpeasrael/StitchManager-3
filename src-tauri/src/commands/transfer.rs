use serde::{Deserialize, Serialize};
use tauri::State;

use crate::DbState;
use crate::error::{lock_db, AppError};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineProfile {
    pub id: i64,
    pub name: String,
    pub machine_type: String,
    pub transfer_path: String,
    pub target_format: Option<String>,
    pub last_used: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub fn list_machines(db: State<'_, DbState>) -> Result<Vec<MachineProfile>, AppError> {
    let conn = lock_db(&db)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, machine_type, transfer_path, target_format, last_used, created_at \
         FROM machine_profiles ORDER BY last_used DESC NULLS LAST, name"
    )?;
    let machines = stmt
        .query_map([], |row| {
            Ok(MachineProfile {
                id: row.get(0)?,
                name: row.get(1)?,
                machine_type: row.get(2)?,
                transfer_path: row.get(3)?,
                target_format: row.get(4)?,
                last_used: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(machines)
}

#[tauri::command]
pub fn add_machine(
    db: State<'_, DbState>,
    name: String,
    machine_type: String,
    transfer_path: String,
    target_format: Option<String>,
) -> Result<MachineProfile, AppError> {
    // Validate transfer path
    if transfer_path.is_empty() {
        return Err(AppError::Validation("Ungueltiger Uebertragungspfad".into()));
    }
    super::validate_no_traversal(&transfer_path)?;

    let conn = lock_db(&db)?;
    conn.execute(
        "INSERT INTO machine_profiles (name, machine_type, transfer_path, target_format) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![name, machine_type, transfer_path, target_format],
    )?;
    let id = conn.last_insert_rowid();
    Ok(MachineProfile {
        id,
        name,
        machine_type,
        transfer_path,
        target_format,
        last_used: None,
        created_at: String::new(),
    })
}

#[tauri::command]
pub fn delete_machine(db: State<'_, DbState>, machine_id: i64) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let affected = conn.execute("DELETE FROM machine_profiles WHERE id = ?1", [machine_id])?;
    if affected == 0 {
        return Err(AppError::NotFound("Maschine nicht gefunden".into()));
    }
    Ok(())
}

#[tauri::command]
pub fn test_machine_connection(
    db: State<'_, DbState>,
    machine_id: i64,
) -> Result<bool, AppError> {
    let conn = lock_db(&db)?;
    let path: String = conn.query_row(
        "SELECT transfer_path FROM machine_profiles WHERE id = ?1",
        [machine_id],
        |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("Maschine nicht gefunden".into()),
        other => AppError::Database(other),
    })?;

    // Check if the transfer path is accessible
    let p = std::path::Path::new(&path);
    Ok(p.exists() && p.is_dir())
}

#[tauri::command]
pub fn transfer_files(
    db: State<'_, DbState>,
    machine_id: i64,
    file_ids: Vec<i64>,
) -> Result<TransferResult, AppError> {
    let (transfer_path, target_format): (String, Option<String>) = {
        let conn = lock_db(&db)?;
        conn.query_row(
            "SELECT transfer_path, target_format FROM machine_profiles WHERE id = ?1",
            [machine_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("Maschine nicht gefunden".into()),
            other => AppError::Database(other),
        })?
    };

    let dest = std::path::Path::new(&transfer_path);
    if !dest.exists() || !dest.is_dir() {
        return Err(AppError::Validation(format!("Zielpfad nicht erreichbar: {transfer_path}")));
    }

    // Load file paths
    let file_paths: Vec<(i64, String)> = {
        let conn = lock_db(&db)?;
        let mut result = Vec::new();
        for fid in &file_ids {
            match conn.query_row(
                "SELECT filepath FROM embroidery_files WHERE id = ?1",
                [fid],
                |row| row.get::<_, String>(0),
            ) {
                Ok(fp) => result.push((*fid, fp)),
                Err(_) => {
                    // Will be counted in the difference between file_ids.len() and file_paths.len()
                    continue;
                }
            }
        }
        result
    };

    let mut success = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    for (_file_id, filepath) in &file_paths {
        let src = std::path::Path::new(filepath);
        if !src.exists() {
            failed += 1;
            errors.push(format!("Datei nicht gefunden: {filepath}"));
            continue;
        }

        let src_ext = src.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
        let needs_convert = target_format.as_ref().map(|tf| tf.to_lowercase() != src_ext).unwrap_or(false);

        if needs_convert {
            let tf = target_format.as_ref().unwrap();
            let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("file")
                .replace(['/', '\\', '.'], "_");
            let dest_file = dest.join(format!("{stem}.{}", tf.to_lowercase()));

            if dest_file.exists() {
                failed += 1;
                errors.push(format!("{}: Zieldatei existiert bereits", dest_file.display()));
                continue;
            }

            match convert_and_copy(src, &dest_file, tf) {
                Ok(_) => success += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: {e}", src.display()));
                }
            }
        } else {
            let filename = src.file_name().unwrap_or_default().to_string_lossy();
            let safe_name = filename.replace(['/', '\\'], "_");
            let dest_file = dest.join(&*safe_name);

            if dest_file.exists() {
                failed += 1;
                errors.push(format!("{}: Zieldatei existiert bereits", dest_file.display()));
                continue;
            }

            match std::fs::copy(src, &dest_file) {
                Ok(_) => success += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: {e}", src.display()));
                }
            }
        }
    }

    // Update last_used
    if let Ok(conn) = lock_db(&db) {
        let _ = conn.execute(
            "UPDATE machine_profiles SET last_used = datetime('now') WHERE id = ?1",
            [machine_id],
        );
    }

    // Count missing files as failed
    let missing = file_ids.len() as i32 - file_paths.len() as i32;
    failed += missing;

    Ok(TransferResult { total: file_ids.len() as i32, success, failed, errors })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferResult {
    pub total: i32,
    pub success: i32,
    pub failed: i32,
    pub errors: Vec<String>,
}

fn convert_and_copy(src: &std::path::Path, dest: &std::path::Path, target_format: &str) -> Result<(), AppError> {
    let ext = src.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
    let parser = crate::parsers::get_parser(&ext)
        .ok_or_else(|| AppError::Validation(format!("Kein Parser fuer: {ext}")))?;

    let raw_data = std::fs::read(src)?;
    let segments = parser.extract_stitch_segments(&raw_data)?;

    if segments.is_empty() {
        return Err(AppError::Validation("Keine Stichdaten".into()));
    }

    crate::parsers::writers::convert_segments(&segments, target_format, dest)
}
