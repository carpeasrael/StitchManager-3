use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::db::models::EmbroideryFile;
use crate::db::queries::{FILE_SELECT, row_to_file};
use crate::error::{lock_db, AppError};
use crate::DbState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub total: i64,
    pub success: i64,
    pub failed: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgressPayload {
    pub current: i64,
    pub total: i64,
    pub filename: String,
    pub status: String,
}

/// Sanitize a path component by removing path traversal sequences and absolute paths.
/// Strips `..`, leading `/`, and replaces remaining `/` and `\` in individual values.
fn sanitize_path_component(value: &str) -> String {
    value
        .replace("..", "")
        .replace('/', "_")
        .replace('\\', "_")
        .trim()
        .to_string()
}

/// Sanitize the full output of apply_pattern for use as a path.
/// Preserves `/` that come from the pattern itself (for organize patterns like `{theme}/{name}`)
/// but sanitizes each placeholder's resolved value.
fn sanitize_pattern_output(result: &str) -> String {
    // Remove any ".." path components
    let parts: Vec<&str> = result.split('/').collect();
    let cleaned: Vec<&str> = parts.into_iter().filter(|p| *p != ".." && !p.is_empty()).collect();
    let joined = cleaned.join("/");
    // Strip leading / to prevent absolute paths
    joined.strip_prefix('/').unwrap_or(&joined).to_string()
}

/// Apply pattern substitution to a file's metadata.
/// Placeholders: {name}, {theme}, {format}
/// Individual placeholder values are sanitized to prevent path traversal.
pub fn apply_pattern(pattern: &str, file: &EmbroideryFile) -> String {
    let name = sanitize_path_component(file.name.as_deref().unwrap_or("unbenannt"));
    let theme = sanitize_path_component(file.theme.as_deref().unwrap_or("unbekannt"));
    let format = file
        .filename
        .rsplit('.')
        .next()
        .unwrap_or("bin")
        .to_lowercase();

    let result = pattern
        .replace("{name}", &name)
        .replace("{theme}", &theme)
        .replace("{format}", &format);

    sanitize_pattern_output(&result)
}

/// Given a desired path, return a collision-free path by appending `_1`, `_2`, etc.
/// Checks both the filesystem and a set of already-claimed paths from this batch.
/// Counter is capped at 100_000 to prevent infinite loops.
fn dedup_path(
    path: &std::path::Path,
    claimed: &mut HashSet<std::path::PathBuf>,
) -> std::path::PathBuf {
    let mut candidate = path.to_path_buf();
    if !candidate.exists() && !claimed.contains(&candidate) {
        claimed.insert(candidate.clone());
        return candidate;
    }

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str());
    let parent = path.parent().unwrap_or(std::path::Path::new(""));

    for counter in 1..=100_000u32 {
        candidate = if let Some(e) = ext {
            parent.join(format!("{stem}_{counter}.{e}"))
        } else {
            parent.join(format!("{stem}_{counter}"))
        };
        if !candidate.exists() && !claimed.contains(&candidate) {
            claimed.insert(candidate.clone());
            return candidate;
        }
    }

    // Fallback: return with max counter suffix (extremely unlikely)
    claimed.insert(candidate.clone());
    candidate
}

#[tauri::command]
pub async fn batch_rename(
    db: State<'_, DbState>,
    app_handle: AppHandle,
    file_ids: Vec<i64>,
    pattern: String,
) -> Result<BatchResult, AppError> {
    let total = file_ids.len() as i64;
    let mut success: i64 = 0;
    let mut failed: i64 = 0;
    let mut errors: Vec<String> = Vec::new();
    let pattern_has_format = pattern.contains("{format}");
    let mut claimed: HashSet<std::path::PathBuf> = HashSet::new();

    // Phase 1: Load all file metadata in a single DB lock
    let files: Vec<(i64, Option<EmbroideryFile>)> = {
        let conn = lock_db(&db)?;
        file_ids
            .iter()
            .map(|id| {
                let file = match conn.query_row(
                    &format!("{FILE_SELECT} WHERE id = ?1"),
                    [id],
                    |row| row_to_file(row),
                ) {
                    Ok(f) => Some(f),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => {
                        log::warn!("batch_rename: DB error loading file {id}: {e}");
                        None
                    }
                };
                (*id, file)
            })
            .collect()
    };

    // Phase 2: Perform filesystem renames without holding the DB lock.
    // Progress events emitted here report "success" per file. If Phase 3
    // (DB transaction) fails, all renames are rolled back and the command
    // returns Err — the frontend receives the error and can discard the
    // per-file progress. This is an inherent trade-off of the 3-phase design.
    // TOCTOU window between Phase 1 read and Phase 3 write is acceptable
    // for a single-user desktop app.
    struct RenameOp {
        file_id: i64,
        new_filename: String,
        new_path: std::path::PathBuf,
        old_path: std::path::PathBuf,
        did_rename: bool,
    }
    let mut pending_updates: Vec<RenameOp> = Vec::new();

    for (i, (file_id, file_opt)) in files.iter().enumerate() {
        let result = (|| -> Result<RenameOp, AppError> {
            let file = file_opt.as_ref().ok_or_else(|| {
                AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
            })?;

            let ext = file.filename.rsplit('.').next().unwrap_or("");
            let base = apply_pattern(&pattern, file);
            let desired_filename = if pattern_has_format || ext.is_empty() {
                base
            } else {
                format!("{base}.{ext}")
            };

            let old_path = std::path::Path::new(&file.filepath);
            let canonical_old = if old_path.exists() {
                old_path.canonicalize()?
            } else {
                old_path.to_path_buf()
            };
            let parent = old_path.parent().unwrap_or(std::path::Path::new(""));
            let desired_path = parent.join(&desired_filename);
            let new_path = dedup_path(&desired_path, &mut claimed);
            let new_filename = new_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&desired_filename)
                .to_string();

            let did_rename = if old_path.exists() && canonical_old != new_path {
                std::fs::rename(old_path, &new_path)?;
                true
            } else {
                false
            };

            Ok(RenameOp {
                file_id: *file_id,
                new_filename,
                new_path,
                old_path: old_path.to_path_buf(),
                did_rename,
            })
        })();

        match result {
            Ok(op) => {
                success += 1;
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: op.new_filename.clone(),
                        status: "success".to_string(),
                    },
                );
                pending_updates.push(op);
            }
            Err(e) => {
                failed += 1;
                let msg = format!("Datei {file_id}: {e}");
                errors.push(msg);
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: format!("Datei {file_id}"),
                        status: format!("error: {e}"),
                    },
                );
            }
        }
    }

    // Phase 3: Update all successful renames in a single DB transaction
    if !pending_updates.is_empty() {
        let conn = lock_db(&db)?;
        let tx_result = (|| -> Result<(), rusqlite::Error> {
            let tx = conn.unchecked_transaction()?;
            for op in &pending_updates {
                tx.execute(
                    "UPDATE embroidery_files SET filename = ?1, filepath = ?2, \
                     updated_at = datetime('now') WHERE id = ?3",
                    rusqlite::params![op.new_filename, op.new_path.to_string_lossy().as_ref(), op.file_id],
                )?;
            }
            tx.commit()
        })();

        if let Err(e) = tx_result {
            // Rollback all filesystem renames on transaction failure
            let mut rollback_failures: Vec<String> = Vec::new();
            for op in &pending_updates {
                if op.did_rename {
                    if let Err(rb_err) = std::fs::rename(&op.new_path, &op.old_path) {
                        log::error!(
                            "Rollback failed for file {}: {} -> {}: {}",
                            op.file_id,
                            op.new_path.display(),
                            op.old_path.display(),
                            rb_err
                        );
                        rollback_failures.push(format!(
                            "{}: {}", op.new_path.display(), rb_err
                        ));
                    }
                }
            }
            if rollback_failures.is_empty() {
                return Err(AppError::Database(e));
            } else {
                return Err(AppError::Internal(format!(
                    "DB-Transaktion fehlgeschlagen: {}. Rollback fehlgeschlagen fuer {} Dateien: {}",
                    e, rollback_failures.len(), rollback_failures.join("; ")
                )));
            }
        }
    }

    Ok(BatchResult { total, success, failed, errors })
}

#[tauri::command]
pub async fn batch_organize(
    db: State<'_, DbState>,
    app_handle: AppHandle,
    file_ids: Vec<i64>,
    pattern: String,
) -> Result<BatchResult, AppError> {
    // Phase 1: Load library_root and all file metadata in a single DB lock
    let (library_root, files): (String, Vec<(i64, Option<EmbroideryFile>)>) = {
        let conn = lock_db(&db)?;
        let root = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'library_root'",
                [],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| AppError::Validation("library_root ist nicht konfiguriert".into()))?;
        let file_list = file_ids
            .iter()
            .map(|id| {
                let file = match conn.query_row(
                    &format!("{FILE_SELECT} WHERE id = ?1"),
                    [id],
                    |row| row_to_file(row),
                ) {
                    Ok(f) => Some(f),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => {
                        log::warn!("batch_organize: DB error loading file {id}: {e}");
                        None
                    }
                };
                (*id, file)
            })
            .collect();
        (root, file_list)
    };

    // Expand ~ to home directory
    let base_dir = if library_root.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&library_root[2..])
        } else {
            std::path::PathBuf::from(&library_root)
        }
    } else {
        std::path::PathBuf::from(&library_root)
    };

    let total = file_ids.len() as i64;
    let mut success: i64 = 0;
    let mut failed: i64 = 0;
    let mut errors: Vec<String> = Vec::new();
    let mut claimed: HashSet<std::path::PathBuf> = HashSet::new();

    let canonical_base = base_dir.canonicalize().map_err(|e| {
        AppError::Validation(format!(
            "Bibliotheksverzeichnis nicht gefunden: {}: {e}",
            base_dir.display()
        ))
    })?;

    // Phase 2: Perform filesystem moves without holding the DB lock.
    // Progress events emitted here report "success" per file. If Phase 3
    // (DB transaction) fails, all moves are rolled back and the command
    // returns Err — the frontend receives the error and can discard the
    // per-file progress. This is an inherent trade-off of the 3-phase design.
    // Note: folder_id is intentionally not updated — organize is a filesystem-only
    // operation. The file retains its original folder association in the UI.
    // TOCTOU window between Phase 1 read and Phase 3 write is acceptable
    // for a single-user desktop app.
    struct MoveOp {
        file_id: i64,
        new_filename: String,
        new_path: std::path::PathBuf,
        old_path: std::path::PathBuf,
        did_rename: bool,
    }
    let mut pending_updates: Vec<MoveOp> = Vec::new();

    for (i, (file_id, file_opt)) in files.iter().enumerate() {
        let result = (|| -> Result<MoveOp, AppError> {
            let file = file_opt.as_ref().ok_or_else(|| {
                AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
            })?;

            let sub_path = apply_pattern(&pattern, file);
            let target_dir = base_dir.join(&sub_path);

            let normalized: std::path::PathBuf = target_dir.components().collect();
            if !normalized.starts_with(&canonical_base) {
                return Err(AppError::Validation(
                    "Zielpfad liegt ausserhalb der Bibliothek".into(),
                ));
            }

            std::fs::create_dir_all(&target_dir)?;

            let old_path = std::path::Path::new(&file.filepath);
            let canonical_old = if old_path.exists() {
                old_path.canonicalize()?
            } else {
                old_path.to_path_buf()
            };
            let desired_path = target_dir.join(&file.filename);
            let new_path = dedup_path(&desired_path, &mut claimed);

            let did_rename = if old_path.exists() && canonical_old != new_path {
                std::fs::rename(old_path, &new_path)?;
                true
            } else {
                false
            };

            let new_filename = new_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&file.filename)
                .to_string();

            Ok(MoveOp {
                file_id: *file_id,
                new_filename,
                new_path,
                old_path: old_path.to_path_buf(),
                did_rename,
            })
        })();

        match result {
            Ok(op) => {
                success += 1;
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: op.new_filename.clone(),
                        status: "success".to_string(),
                    },
                );
                pending_updates.push(op);
            }
            Err(e) => {
                failed += 1;
                let msg = format!("Datei {file_id}: {e}");
                errors.push(msg);
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: format!("Datei {file_id}"),
                        status: format!("error: {e}"),
                    },
                );
            }
        }
    }

    // Phase 3: Update all successful moves in a single DB transaction
    if !pending_updates.is_empty() {
        let conn = lock_db(&db)?;
        let tx_result = (|| -> Result<(), rusqlite::Error> {
            let tx = conn.unchecked_transaction()?;
            for op in &pending_updates {
                tx.execute(
                    "UPDATE embroidery_files SET filename = ?1, filepath = ?2, \
                     updated_at = datetime('now') WHERE id = ?3",
                    rusqlite::params![op.new_filename, op.new_path.to_string_lossy().as_ref(), op.file_id],
                )?;
            }
            tx.commit()
        })();

        if let Err(e) = tx_result {
            let mut rollback_failures: Vec<String> = Vec::new();
            for op in &pending_updates {
                if op.did_rename {
                    if let Err(rb_err) = std::fs::rename(&op.new_path, &op.old_path) {
                        log::error!(
                            "Rollback failed for file {}: {} -> {}: {}",
                            op.file_id,
                            op.new_path.display(),
                            op.old_path.display(),
                            rb_err
                        );
                        rollback_failures.push(format!(
                            "{}: {}", op.new_path.display(), rb_err
                        ));
                    }
                }
            }
            if rollback_failures.is_empty() {
                return Err(AppError::Database(e));
            } else {
                return Err(AppError::Internal(format!(
                    "DB-Transaktion fehlgeschlagen: {}. Rollback fehlgeschlagen fuer {} Dateien: {}",
                    e, rollback_failures.len(), rollback_failures.join("; ")
                )));
            }
        }
    }

    Ok(BatchResult { total, success, failed, errors })
}

#[tauri::command]
pub async fn batch_export_usb(
    db: State<'_, DbState>,
    app_handle: AppHandle,
    file_ids: Vec<i64>,
    target_path: String,
) -> Result<BatchResult, AppError> {
    // Reject path traversal attempts
    super::validate_no_traversal(&target_path)?;
    let target_dir = std::path::Path::new(&target_path);
    if !target_dir.exists() {
        std::fs::create_dir_all(target_dir)?;
    }

    // Canonicalize the target directory to resolve symlinks and normalize the path
    let canonical_target = target_dir.canonicalize().map_err(|e| {
        AppError::Validation(format!("Zielverzeichnis kann nicht aufgeloest werden: {e}"))
    })?;

    // Phase 1: Load all file paths in a single DB lock
    let files: Vec<(i64, Option<(String, String)>)> = {
        let conn = lock_db(&db)?;
        file_ids
            .iter()
            .map(|id| {
                let result = match conn.query_row(
                    "SELECT filename, filepath FROM embroidery_files WHERE id = ?1",
                    [id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                ) {
                    Ok(pair) => Some(pair),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => {
                        log::warn!("batch_export_usb: DB error loading file {id}: {e}");
                        None
                    }
                };
                (*id, result)
            })
            .collect()
    };

    let total = file_ids.len() as i64;
    let mut success: i64 = 0;
    let mut failed: i64 = 0;
    let mut errors: Vec<String> = Vec::new();
    let mut claimed: HashSet<std::path::PathBuf> = HashSet::new();

    // Phase 2: Copy files without holding the DB lock
    for (i, (file_id, file_opt)) in files.iter().enumerate() {
        let result = (|| -> Result<String, AppError> {
            let (filename, filepath) = file_opt.as_ref().ok_or_else(|| {
                AppError::NotFound(format!("Datei {file_id} nicht gefunden"))
            })?;

            let source = std::path::Path::new(filepath);
            if !source.exists() {
                return Err(AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Quelldatei nicht gefunden: {filepath}"),
                )));
            }

            // Sanitize filename to prevent path traversal via crafted DB entries
            let safe_filename = filename
                .replace('/', "_")
                .replace('\\', "_")
                .replace("..", "");
            let desired = canonical_target.join(&safe_filename);
            let dest = dedup_path(&desired, &mut claimed);

            // Verify the resolved destination stays within the target directory.
            // The dest file doesn't exist yet, so canonicalize the parent and append the filename.
            let canonical_dest = dest.parent()
                .ok_or_else(|| AppError::Validation("Zieldatei hat kein uebergeordnetes Verzeichnis".into()))?
                .canonicalize()
                .map(|cp| cp.join(dest.file_name().unwrap_or_default()))
                .map_err(|e| AppError::Validation(format!(
                    "Zielverzeichnis kann nicht aufgeloest werden: {e}"
                )))?;
            if !canonical_dest.starts_with(&canonical_target) {
                return Err(AppError::Validation(
                    "Zieldatei liegt ausserhalb des Zielverzeichnisses".into(),
                ));
            }

            std::fs::copy(source, &dest)?;

            Ok(filename.clone())
        })();

        match result {
            Ok(ref filename) => {
                success += 1;
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: filename.clone(),
                        status: "success".to_string(),
                    },
                );
            }
            Err(e) => {
                failed += 1;
                let msg = format!("Datei {file_id}: {e}");
                errors.push(msg);
                let _ = app_handle.emit(
                    "batch:progress",
                    BatchProgressPayload {
                        current: (i + 1) as i64,
                        total,
                        filename: format!("Datei {file_id}"),
                        status: format!("error: {e}"),
                    },
                );
            }
        }
    }

    Ok(BatchResult { total, success, failed, errors })
}

/// Generate a PDF report for the given file IDs.
/// Returns the file path where the PDF was saved.
#[tauri::command]
pub async fn generate_pdf_report(
    db: State<'_, DbState>,
    file_ids: Vec<i64>,
) -> Result<String, AppError> {
    use crate::db::models::FileThreadColor;

    // Load all file and color data from DB, then drop the lock before CPU-bound QR generation
    let mut db_data: Vec<(EmbroideryFile, Vec<FileThreadColor>)> = Vec::new();

    {
        let conn = lock_db(&db)?;

        for file_id in &file_ids {
            let file = match conn.query_row(
                &format!("{FILE_SELECT} WHERE id = ?1"),
                [file_id],
                |row| row_to_file(row),
            ) {
                Ok(f) => f,
                Err(rusqlite::Error::QueryReturnedNoRows) => continue,
                Err(e) => return Err(AppError::Database(e)),
            };

            // Load thread colors
            let mut stmt = conn.prepare(
                "SELECT id, file_id, sort_order, color_hex, color_name, brand, brand_code, is_ai \
                 FROM file_thread_colors WHERE file_id = ?1 ORDER BY sort_order",
            )?;
            let colors: Vec<FileThreadColor> = stmt
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

            db_data.push((file, colors));
        }
    } // DB lock dropped here

    // Generate QR codes and load thumbnails outside the DB lock (CPU/IO-bound)
    let mut report_data = Vec::new();
    for (file, colors) in db_data {
        let qr_png = if let Some(ref uid) = file.unique_id {
            match crate::commands::files::generate_qr_code(uid.clone()) {
                Ok(data) => Some(data),
                Err(_) => None,
            }
        } else {
            None
        };
        let thumb_png = file
            .thumbnail_path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok());
        report_data.push((file, colors, qr_png, thumb_png));
    }

    let pdf_bytes = crate::services::pdf_report::generate_report(&report_data)?;

    // Save to temp directory
    let temp_dir = std::env::temp_dir();
    let filename = format!("stichman_report_{}.pdf", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = temp_dir.join(&filename);
    std::fs::write(&path, &pdf_bytes)?;

    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_apply_pattern_basic() {
        let file = EmbroideryFile {
            id: 1,
            folder_id: 1,
            filename: "rose.pes".to_string(),
            filepath: "/test/rose.pes".to_string(),
            name: Some("Rose Design".to_string()),
            theme: Some("Blumen".to_string()),
            description: None,
            license: None,
            width_mm: None,
            height_mm: None,
            stitch_count: None,
            color_count: None,
            file_size_bytes: None,
            thumbnail_path: None,
            design_name: None,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
            unique_id: None,
            is_favorite: false,
            file_type: "embroidery".to_string(),
            size_range: None,
            skill_level: None,
            language: None,
            format_type: None,
            file_source: None,
            purchase_link: None,
            status: "none".to_string(),
            ai_analyzed: false,
            ai_confirmed: false,
            created_at: String::new(),
            updated_at: String::new(),
        };

        assert_eq!(
            apply_pattern("{name}_{theme}", &file),
            "Rose Design_Blumen"
        );
        assert_eq!(
            apply_pattern("{theme}/{name}", &file),
            "Blumen/Rose Design"
        );
        assert_eq!(
            apply_pattern("{name}.{format}", &file),
            "Rose Design.pes"
        );
    }

    #[test]
    fn test_apply_pattern_missing_metadata() {
        let file = EmbroideryFile {
            id: 1,
            folder_id: 1,
            filename: "test.dst".to_string(),
            filepath: "/test/test.dst".to_string(),
            name: None,
            theme: None,
            description: None,
            license: None,
            width_mm: None,
            height_mm: None,
            stitch_count: None,
            color_count: None,
            file_size_bytes: None,
            thumbnail_path: None,
            design_name: None,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
            unique_id: None,
            is_favorite: false,
            file_type: "embroidery".to_string(),
            size_range: None,
            skill_level: None,
            language: None,
            format_type: None,
            file_source: None,
            purchase_link: None,
            status: "none".to_string(),
            ai_analyzed: false,
            ai_confirmed: false,
            created_at: String::new(),
            updated_at: String::new(),
        };

        assert_eq!(
            apply_pattern("{name}_{theme}", &file),
            "unbenannt_unbekannt"
        );
    }

    #[test]
    fn test_apply_pattern_path_traversal_sanitized() {
        let file = EmbroideryFile {
            id: 1,
            folder_id: 1,
            filename: "test.pes".to_string(),
            filepath: "/test/test.pes".to_string(),
            name: Some("../../etc/passwd".to_string()),
            theme: Some("../secrets".to_string()),
            description: None,
            license: None,
            width_mm: None,
            height_mm: None,
            stitch_count: None,
            color_count: None,
            file_size_bytes: None,
            thumbnail_path: None,
            design_name: None,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
            unique_id: None,
            is_favorite: false,
            file_type: "embroidery".to_string(),
            size_range: None,
            skill_level: None,
            language: None,
            format_type: None,
            file_source: None,
            purchase_link: None,
            status: "none".to_string(),
            ai_analyzed: false,
            ai_confirmed: false,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let result = apply_pattern("{theme}/{name}", &file);
        // Path traversal components should be sanitized
        assert!(!result.contains(".."));
        assert!(!result.starts_with('/'));
    }

    #[test]
    fn test_batch_organize_path_construction() {
        let file = EmbroideryFile {
            id: 1,
            folder_id: 1,
            filename: "rose.pes".to_string(),
            filepath: "/test/rose.pes".to_string(),
            name: Some("Rose".to_string()),
            theme: Some("Blumen".to_string()),
            description: None,
            license: None,
            width_mm: None,
            height_mm: None,
            stitch_count: None,
            color_count: None,
            file_size_bytes: None,
            thumbnail_path: None,
            design_name: None,
            jump_count: None,
            trim_count: None,
            hoop_width_mm: None,
            hoop_height_mm: None,
            category: None,
            author: None,
            keywords: None,
            comments: None,
            unique_id: None,
            is_favorite: false,
            file_type: "embroidery".to_string(),
            size_range: None,
            skill_level: None,
            language: None,
            format_type: None,
            file_source: None,
            purchase_link: None,
            status: "none".to_string(),
            ai_analyzed: false,
            ai_confirmed: false,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let pattern = "{theme}/{name}";
        let sub_path = apply_pattern(pattern, &file);
        let base = std::path::Path::new("/library");
        let target = base.join(&sub_path);
        assert_eq!(target.to_string_lossy(), "/library/Blumen/Rose");
    }

    #[test]
    fn test_batch_result_serialization() {
        let result = BatchResult {
            total: 5,
            success: 4,
            failed: 1,
            errors: vec!["Datei 3: nicht gefunden".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"total\":5"));
        assert!(json.contains("\"success\":4"));
        assert!(json.contains("\"failed\":1"));
    }

    #[test]
    fn test_rename_pattern_db_integration() {
        let conn = init_database_in_memory().unwrap();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Test', '/test')",
            [],
        )
        .unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, name, theme) \
             VALUES (?1, 'original.pes', '/test/original.pes', 'Stern', 'Geometrie')",
            [folder_id],
        )
        .unwrap();
        let file_id = conn.last_insert_rowid();

        // Simulate rename: build new name from pattern
        let file = conn
            .query_row(
                &format!("{FILE_SELECT} WHERE id = ?1"),
                [file_id],
                |row| row_to_file(row),
            )
            .unwrap();

        let new_base = apply_pattern("{name}_{theme}", &file);
        let new_filename = format!("{new_base}.pes");
        assert_eq!(new_filename, "Stern_Geometrie.pes");

        // Update in DB
        conn.execute(
            "UPDATE embroidery_files SET filename = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![new_filename, file_id],
        )
        .unwrap();

        let updated_name: String = conn
            .query_row(
                "SELECT filename FROM embroidery_files WHERE id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(updated_name, "Stern_Geometrie.pes");
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("normal"), "normal");
        assert_eq!(sanitize_path_component("../etc"), "_etc");
        assert_eq!(sanitize_path_component("../../passwd"), "__passwd");
        assert_eq!(sanitize_path_component("foo/bar"), "foo_bar");
        assert_eq!(sanitize_path_component("foo\\bar"), "foo_bar");
    }

    #[test]
    fn test_dedup_path_no_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("unique_file.pes");
        let mut claimed = HashSet::new();
        let result = dedup_path(&path, &mut claimed);
        assert_eq!(result, path);
        assert!(claimed.contains(&result));
    }

    #[test]
    fn test_dedup_path_batch_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test_dedup.pes");
        let mut claimed = HashSet::new();

        // First claim succeeds with original name
        let first = dedup_path(&path, &mut claimed);
        assert_eq!(first, path);

        // Second claim gets _1 suffix
        let second = dedup_path(&path, &mut claimed);
        assert_eq!(second, tmp.path().join("test_dedup_1.pes"));

        // Third claim gets _2 suffix
        let third = dedup_path(&path, &mut claimed);
        assert_eq!(third, tmp.path().join("test_dedup_2.pes"));
    }

    #[test]
    fn test_dedup_path_no_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("noext");
        let mut claimed = HashSet::new();

        let first = dedup_path(&path, &mut claimed);
        assert_eq!(first, path);

        let second = dedup_path(&path, &mut claimed);
        assert_eq!(second, tmp.path().join("noext_1"));
    }

    #[test]
    fn test_dedup_path_existing_file_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("existing.pes");
        // Create the file on disk
        std::fs::write(&path, b"test").unwrap();

        let mut claimed = HashSet::new();
        let result = dedup_path(&path, &mut claimed);
        // Should skip the existing file and get _1 suffix
        assert_eq!(result, tmp.path().join("existing_1.pes"));
    }
}
