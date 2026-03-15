use tauri::State;
use crate::DbState;
use crate::error::{lock_db, AppError};
use crate::parsers;

#[tauri::command]
pub fn get_supported_formats() -> Vec<String> {
    parsers::writers::supported_output_formats()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[tauri::command]
pub fn convert_file(
    db: State<'_, DbState>,
    file_id: i64,
    target_format: String,
    output_dir: String,
) -> Result<String, AppError> {
    convert_file_inner(&db, file_id, &target_format, &output_dir)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertBatchResult {
    pub total: i32,
    pub success: i32,
    pub failed: i32,
    pub errors: Vec<String>,
}

#[tauri::command]
pub fn convert_files_batch(
    db: State<'_, DbState>,
    file_ids: Vec<i64>,
    target_format: String,
    output_dir: String,
) -> Result<ConvertBatchResult, AppError> {
    let mut success = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    for file_id in &file_ids {
        match convert_file_inner(&db, *file_id, &target_format, &output_dir) {
            Ok(_) => success += 1,
            Err(e) => {
                failed += 1;
                errors.push(format!("Datei {file_id}: {e}"));
            }
        }
    }

    Ok(ConvertBatchResult {
        total: file_ids.len() as i32,
        success,
        failed,
        errors,
    })
}

fn convert_file_inner(
    db: &State<'_, DbState>,
    file_id: i64,
    target_format: &str,
    output_dir: &str,
) -> Result<String, AppError> {
    // Reject path traversal attempts
    super::validate_no_traversal(&output_dir)?;
    // Auto-version and fetch filepath in a single lock acquisition
    let filepath: String = {
        let conn = lock_db(db)?;
        let desc = format!("Konvertierung nach {target_format}");
        let _ = super::versions::create_version_snapshot(&conn, file_id, "convert", Some(&desc));
        conn.query_row(
            "SELECT filepath FROM embroidery_files WHERE id = ?1",
            [file_id],
            |row| row.get(0),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?
    };

    let src_path = std::path::Path::new(&filepath);
    let ext = src_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // Prevent same-format conversion (lossy re-encoding)
    if ext == target_format.to_lowercase() {
        return Err(AppError::Validation("Quell- und Zielformat sind identisch".into()));
    }

    let parser = parsers::get_parser(&ext)
        .ok_or_else(|| AppError::Validation(format!("Kein Parser fuer: {ext}")))?;

    let raw_data = std::fs::read(src_path)?;
    let segments = parser.extract_stitch_segments(&raw_data)?;

    if segments.is_empty() {
        return Err(AppError::Validation("Keine Stichdaten".into()));
    }

    let stem = src_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted");
    let target_ext = target_format.to_lowercase();
    let output_path = std::path::Path::new(output_dir).join(format!("{stem}.{target_ext}"));

    parsers::writers::convert_segments(&segments, target_format, &output_path)?;

    Ok(output_path.to_string_lossy().to_string())
}
