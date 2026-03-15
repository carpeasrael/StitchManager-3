use serde::Deserialize;
use tauri::State;

use crate::DbState;
use crate::error::{lock_db, AppError};
use crate::parsers::{self, StitchSegment};
use crate::services::stitch_transform;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Transform {
    Resize { scale_x: f64, scale_y: f64 },
    Rotate { degrees: f64 },
    MirrorHorizontal,
    MirrorVertical,
}

fn apply_transforms(segments: &mut [StitchSegment], transforms: &[Transform]) {
    for t in transforms {
        match t {
            Transform::Resize { scale_x, scale_y } => {
                stitch_transform::resize(segments, *scale_x, *scale_y);
            }
            Transform::Rotate { degrees } => {
                stitch_transform::rotate(segments, *degrees);
            }
            Transform::MirrorHorizontal => {
                stitch_transform::mirror_horizontal(segments);
            }
            Transform::MirrorVertical => {
                stitch_transform::mirror_vertical(segments);
            }
        }
    }
}

fn load_segments(db: &State<'_, DbState>, file_id: i64) -> Result<(Vec<StitchSegment>, String), AppError> {
    let filepath: String = {
        let conn = lock_db(db)?;
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

    let parser = parsers::get_parser(&ext)
        .ok_or_else(|| AppError::Validation(format!("Kein Parser fuer: {ext}")))?;

    let raw_data = std::fs::read(src_path)?;
    let segments = parser.extract_stitch_segments(&raw_data)?;

    if segments.is_empty() {
        return Err(AppError::Validation("Keine Stichdaten vorhanden".into()));
    }

    Ok((segments, filepath))
}

/// Preview transforms without saving — returns transformed stitch segments.
#[tauri::command]
pub fn preview_transform(
    db: State<'_, DbState>,
    file_id: i64,
    transforms: Vec<Transform>,
) -> Result<Vec<StitchSegment>, AppError> {
    let (mut segments, _) = load_segments(&db, file_id)?;
    apply_transforms(&mut segments, &transforms);
    Ok(segments)
}

/// Apply transforms and save to a new file.
#[tauri::command]
pub fn save_transformed(
    db: State<'_, DbState>,
    file_id: i64,
    transforms: Vec<Transform>,
    output_path: String,
) -> Result<String, AppError> {
    // Reject path traversal attempts
    super::validate_no_traversal(&output_path)?;
    // Auto-version before transform
    {
        let conn = lock_db(&db)?;
        let desc = transforms.iter().map(|t| match t {
            Transform::Resize { scale_x, scale_y } => format!("Resize {scale_x}x{scale_y}"),
            Transform::Rotate { degrees } => format!("Rotate {degrees}°"),
            Transform::MirrorHorizontal => "Mirror H".to_string(),
            Transform::MirrorVertical => "Mirror V".to_string(),
        }).collect::<Vec<_>>().join(", ");
        let _ = super::versions::create_version_snapshot(&conn, file_id, "transform", Some(&desc));
    }

    let (mut segments, filepath) = load_segments(&db, file_id)?;
    apply_transforms(&mut segments, &transforms);

    // Determine output format from the output path extension, fallback to source format
    let out = std::path::Path::new(&output_path);
    let target_ext = out.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_uppercase())
        .unwrap_or_else(|| {
            std::path::Path::new(&filepath)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_uppercase())
                .unwrap_or_else(|| "PES".to_string())
        });

    // Prevent accidental overwrite of existing files
    if out.exists() {
        return Err(AppError::Validation(format!(
            "Datei existiert bereits: {}", out.display()
        )));
    }

    parsers::writers::convert_segments(&segments, &target_ext, out)?;

    // Note: dimensions of the original file are NOT updated — the transformed
    // file is saved to a new path and is not yet registered in the DB.

    Ok(output_path)
}

/// Get current dimensions of a file's stitch data.
#[tauri::command]
pub fn get_stitch_dimensions(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<(f64, f64), AppError> {
    let (segments, _) = load_segments(&db, file_id)?;
    Ok(stitch_transform::dimensions(&segments))
}
