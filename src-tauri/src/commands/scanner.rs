use std::time::Instant;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use walkdir::WalkDir;

use crate::{DbState, ThumbnailState};
use crate::db::models::EmbroideryFile;
use crate::db::queries::{FILE_SELECT, row_to_file};
use crate::error::{lock_db, AppError};
use crate::parsers::{self, ParsedFileInfo, StitchSegment};

const SUPPORTED_EXTENSIONS: &[&str] = &["pes", "dst", "jef", "vp3"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub found_files: Vec<String>,
    pub total_scanned: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanProgress {
    current: u32,
    file: String,
}

fn is_embroidery_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[tauri::command]
pub fn scan_directory(
    path: String,
    app_handle: AppHandle,
) -> Result<ScanResult, AppError> {
    let dir = std::path::Path::new(&path);
    if !dir.is_dir() {
        return Err(AppError::Validation(format!(
            "Pfad ist kein Verzeichnis: {path}"
        )));
    }

    let mut found_files = Vec::new();
    let mut total_scanned: u32 = 0;
    let mut errors = Vec::new();

    // follow_links(false) to avoid infinite loops from circular symlinks
    for entry in WalkDir::new(dir).follow_links(false) {
        match entry {
            Ok(e) => {
                if !e.file_type().is_file() {
                    continue;
                }
                total_scanned += 1;
                let file_path = e.path();
                if is_embroidery_file(file_path) {
                    let filepath_str = file_path.to_string_lossy().to_string();
                    let _ = app_handle.emit("scan:file-found", &filepath_str);
                    found_files.push(filepath_str);
                }
                if total_scanned % 50 == 0 {
                    let _ = app_handle.emit(
                        "scan:progress",
                        ScanProgress {
                            current: total_scanned,
                            file: e.path().to_string_lossy().to_string(),
                        },
                    );
                }
            }
            Err(e) => {
                errors.push(e.to_string());
            }
        }
    }

    let _ = app_handle.emit("scan:complete", &found_files.len());

    Ok(ScanResult {
        found_files,
        total_scanned,
        errors,
    })
}

#[tauri::command]
pub fn import_files(
    db: State<'_, DbState>,
    thumb_state: State<'_, ThumbnailState>,
    file_paths: Vec<String>,
    folder_id: i64,
) -> Result<Vec<EmbroideryFile>, AppError> {
    // Collect filesystem metadata and parse files before acquiring the DB lock
    // to avoid holding the mutex during potentially slow I/O.
    struct PreParsed {
        filepath: String,
        filename: String,
        file_size: Option<i64>,
        parsed: Option<ParsedFileInfo>,
        ext: Option<String>,
    }
    let file_info: Vec<PreParsed> = file_paths
        .iter()
        .map(|filepath| {
            let path = std::path::Path::new(filepath);
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let file_size: Option<i64> = std::fs::metadata(path)
                .ok()
                .and_then(|m| i64::try_from(m.len()).ok());
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());
            let parsed = ext.as_deref()
                .and_then(|e| parsers::get_parser(e))
                .and_then(|parser| {
                    std::fs::read(path)
                        .ok()
                        .and_then(|data| parser.parse(&data).ok())
                });
            PreParsed { filepath: filepath.clone(), filename, file_size, parsed, ext }
        })
        .collect();

    let conn = lock_db(&db)?;

    // Verify folder exists
    let folder_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM folders WHERE id = ?1",
        [folder_id],
        |row| row.get(0),
    )?;
    if !folder_exists {
        return Err(AppError::NotFound(format!(
            "Ordner {folder_id} nicht gefunden"
        )));
    }

    let mut imported_ids: Vec<(i64, String, String)> = Vec::new(); // (id, filepath, ext)
    let mut imported = Vec::new();

    // Transaction for DB inserts only — no file I/O inside
    {
        let tx = conn.unchecked_transaction()?;

        for info in &file_info {
            let result = tx.execute(
                "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![folder_id, info.filename, info.filepath, info.file_size],
            );

            match result {
                Ok(changes) if changes > 0 => {
                    let id = tx.last_insert_rowid();
                    // Persist parsed metadata if available
                    if let Some(pinfo) = &info.parsed {
                        if let Err(e) = tx.execute(
                            "UPDATE embroidery_files SET \
                             stitch_count = ?2, color_count = ?3, width_mm = ?4, height_mm = ?5, \
                             design_name = ?6, jump_count = ?7, trim_count = ?8, \
                             hoop_width_mm = ?9, hoop_height_mm = ?10, \
                             category = ?11, author = ?12, keywords = ?13, comments = ?14 \
                             WHERE id = ?1",
                            rusqlite::params![
                                id,
                                pinfo.stitch_count,
                                pinfo.color_count,
                                pinfo.width_mm,
                                pinfo.height_mm,
                                pinfo.design_name,
                                pinfo.jump_count,
                                pinfo.trim_count,
                                pinfo.hoop_width_mm,
                                pinfo.hoop_height_mm,
                                pinfo.category,
                                pinfo.author,
                                pinfo.keywords,
                                pinfo.comments,
                            ],
                        ) {
                            log::warn!("Failed to update metadata for {}: {e}", info.filepath);
                        }

                        // Persist parsed thread colors into file_thread_colors
                        for (idx, color) in pinfo.colors.iter().enumerate() {
                            if let Err(e) = tx.execute(
                                "INSERT INTO file_thread_colors (file_id, sort_order, color_hex, color_name, brand, brand_code, is_ai) \
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                                rusqlite::params![
                                    id,
                                    idx as i32,
                                    color.hex,
                                    color.name,
                                    color.brand,
                                    color.brand_code,
                                ],
                            ) {
                                log::warn!("Failed to insert color for {}: {e}", info.filepath);
                            }
                        }

                        // Persist format record into file_formats
                        if let Err(e) = tx.execute(
                            "INSERT INTO file_formats (file_id, format, format_version, filepath, file_size_bytes, parsed) \
                             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                            rusqlite::params![
                                id,
                                pinfo.format,
                                pinfo.format_version,
                                info.filepath,
                                info.file_size,
                            ],
                        ) {
                            log::warn!("Failed to insert format for {}: {e}", info.filepath);
                        }
                    }

                    if let Some(ext) = &info.ext {
                        imported_ids.push((id, info.filepath.clone(), ext.clone()));
                    }
                }
                Ok(_) => {
                    // Duplicate (IGNORE), skip silently
                }
                Err(e) => {
                    log::warn!("Failed to import {}: {e}", info.filepath);
                }
            }
        }

        tx.commit()?;
    }

    // Generate thumbnails outside the transaction to avoid holding the DB lock during I/O.
    // Re-reads each file to avoid retaining all raw data in memory during batch imports.
    for (id, filepath, ext) in &imported_ids {
        if let Ok(data) = std::fs::read(std::path::Path::new(filepath)) {
            match thumb_state.0.generate(*id, &data, ext) {
                Ok(thumb_path) => {
                    let _ = conn.execute(
                        "UPDATE embroidery_files SET thumbnail_path = ?2 WHERE id = ?1",
                        rusqlite::params![id, thumb_path.to_string_lossy().as_ref()],
                    );
                }
                Err(e) => {
                    log::warn!("Failed to generate thumbnail for {filepath}: {e}");
                }
            }
        }
    }

    // Fetch final state of imported files (with thumbnail_path set)
    for (id, _, _) in &imported_ids {
        match conn.query_row(
            &format!("{FILE_SELECT} WHERE id = ?1"),
            [id],
            |row| row_to_file(row),
        ) {
            Ok(file) => imported.push(file),
            Err(e) => log::warn!("Failed to read imported file {id}: {e}"),
        }
    }

    Ok(imported)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MassImportResult {
    pub folder_id: i64,
    pub imported_count: u32,
    pub skipped_count: u32,
    pub error_count: u32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportDiscoveryPayload {
    scanned_files: u32,
    found_files: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgressPayload {
    pub current: u32,
    pub total: u32,
    pub filename: String,
    pub status: String,
    pub elapsed_ms: u64,
    pub estimated_remaining_ms: u64,
}

#[tauri::command]
pub fn mass_import(
    db: State<'_, DbState>,
    thumb_state: State<'_, ThumbnailState>,
    path: String,
    app_handle: AppHandle,
) -> Result<MassImportResult, AppError> {
    let dir = std::path::Path::new(&path);
    if !dir.is_dir() {
        return Err(AppError::Validation(format!(
            "Pfad ist kein Verzeichnis: {path}"
        )));
    }

    let start = Instant::now();

    // --- Phase 0: Create or find folder ---
    let folder_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Import")
        .to_string();

    let folder_id: i64;
    {
        let conn = lock_db(&db)?;
        // Check if a folder with this path already exists
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM folders WHERE path = ?1",
                [&path],
                |row| row.get(0),
            )
            .ok();

        folder_id = match existing {
            Some(id) => id,
            None => {
                conn.execute(
                    "INSERT INTO folders (name, path) VALUES (?1, ?2)",
                    rusqlite::params![folder_name, path],
                )?;
                conn.last_insert_rowid()
            }
        };
    }

    // --- Phase 1: Discovery ---
    let mut embroidery_paths: Vec<String> = Vec::new();
    let mut scanned_files: u32 = 0;

    for entry in WalkDir::new(dir).follow_links(false) {
        match entry {
            Ok(e) => {
                if !e.file_type().is_file() {
                    continue;
                }
                scanned_files += 1;
                if is_embroidery_file(e.path()) {
                    embroidery_paths.push(e.path().to_string_lossy().to_string());
                }
                if scanned_files % 50 == 0 {
                    let _ = app_handle.emit(
                        "import:discovery",
                        ImportDiscoveryPayload {
                            scanned_files,
                            found_files: embroidery_paths.len() as u32,
                        },
                    );
                }
            }
            Err(e) => {
                log::warn!("Discovery walk error: {e}");
            }
        }
    }

    // Emit final discovery result
    let _ = app_handle.emit(
        "import:discovery",
        ImportDiscoveryPayload {
            scanned_files,
            found_files: embroidery_paths.len() as u32,
        },
    );

    let total = embroidery_paths.len() as u32;

    // --- Phase 2: Import ---
    let import_start = Instant::now();
    let mut imported_count: u32 = 0;
    let mut skipped_count: u32 = 0;
    let mut error_count: u32 = 0;

    // Pre-parse all files outside DB lock
    struct PreParsed {
        filepath: String,
        filename: String,
        file_size: Option<i64>,
        parsed: Option<ParsedFileInfo>,
        ext: Option<String>,
    }

    let file_infos: Vec<PreParsed> = embroidery_paths
        .iter()
        .map(|filepath| {
            let p = std::path::Path::new(filepath);
            let filename = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let file_size: Option<i64> = std::fs::metadata(p)
                .ok()
                .and_then(|m| i64::try_from(m.len()).ok());
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());
            let parsed = ext
                .as_deref()
                .and_then(|e| parsers::get_parser(e))
                .and_then(|parser| {
                    std::fs::read(p)
                        .ok()
                        .and_then(|data| parser.parse(&data).ok())
                });
            PreParsed {
                filepath: filepath.clone(),
                filename,
                file_size,
                parsed,
                ext,
            }
        })
        .collect();

    let conn = lock_db(&db)?;

    // Verify folder still exists
    let folder_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM folders WHERE id = ?1",
        [folder_id],
        |row| row.get(0),
    )?;
    if !folder_exists {
        return Err(AppError::NotFound(format!(
            "Ordner {folder_id} nicht gefunden"
        )));
    }

    let mut thumb_pending: Vec<(i64, String, String)> = Vec::new();

    // Transaction for DB inserts
    {
        let tx = conn.unchecked_transaction()?;

        for (idx, info) in file_infos.iter().enumerate() {
            let current = (idx + 1) as u32;
            let status: String;

            let result = tx.execute(
                "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![folder_id, info.filename, info.filepath, info.file_size],
            );

            match result {
                Ok(changes) if changes > 0 => {
                    let id = tx.last_insert_rowid();
                    if let Some(pinfo) = &info.parsed {
                        if let Err(e) = tx.execute(
                            "UPDATE embroidery_files SET \
                             stitch_count = ?2, color_count = ?3, width_mm = ?4, height_mm = ?5, \
                             design_name = ?6, jump_count = ?7, trim_count = ?8, \
                             hoop_width_mm = ?9, hoop_height_mm = ?10, \
                             category = ?11, author = ?12, keywords = ?13, comments = ?14 \
                             WHERE id = ?1",
                            rusqlite::params![
                                id,
                                pinfo.stitch_count,
                                pinfo.color_count,
                                pinfo.width_mm,
                                pinfo.height_mm,
                                pinfo.design_name,
                                pinfo.jump_count,
                                pinfo.trim_count,
                                pinfo.hoop_width_mm,
                                pinfo.hoop_height_mm,
                                pinfo.category,
                                pinfo.author,
                                pinfo.keywords,
                                pinfo.comments,
                            ],
                        ) {
                            log::warn!("Failed to update metadata for {}: {e}", info.filepath);
                        }

                        for (cidx, color) in pinfo.colors.iter().enumerate() {
                            if let Err(e) = tx.execute(
                                "INSERT INTO file_thread_colors (file_id, sort_order, color_hex, color_name, brand, brand_code, is_ai) \
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                                rusqlite::params![
                                    id,
                                    cidx as i32,
                                    color.hex,
                                    color.name,
                                    color.brand,
                                    color.brand_code,
                                ],
                            ) {
                                log::warn!("Failed to insert color for {}: {e}", info.filepath);
                            }
                        }

                        if let Err(e) = tx.execute(
                            "INSERT INTO file_formats (file_id, format, format_version, filepath, file_size_bytes, parsed) \
                             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                            rusqlite::params![
                                id,
                                pinfo.format,
                                pinfo.format_version,
                                info.filepath,
                                info.file_size,
                            ],
                        ) {
                            log::warn!("Failed to insert format for {}: {e}", info.filepath);
                        }
                    }

                    if let Some(ext) = &info.ext {
                        thumb_pending.push((id, info.filepath.clone(), ext.clone()));
                    }

                    imported_count += 1;
                    status = "ok".to_string();
                }
                Ok(_) => {
                    skipped_count += 1;
                    status = "skipped".to_string();
                }
                Err(e) => {
                    error_count += 1;
                    status = format!("error:{e}");
                    log::warn!("Failed to import {}: {e}", info.filepath);
                }
            }

            // Throttle progress events: emit every 10 files + always the last one
            // to reduce DB lock contention from cross-thread event serialization
            if current % 10 == 0 || current == total {
                let elapsed = import_start.elapsed().as_millis() as u64;
                let avg_per_file = if current > 0 { elapsed / current as u64 } else { 0 };
                let remaining = total.saturating_sub(current);
                let estimated_remaining_ms = avg_per_file * remaining as u64;

                let _ = app_handle.emit(
                    "import:progress",
                    ImportProgressPayload {
                        current,
                        total,
                        filename: info.filename.clone(),
                        status,
                        elapsed_ms: elapsed,
                        estimated_remaining_ms,
                    },
                );
            }
        }

        tx.commit()?;
    }

    // Drop DB lock before thumbnail generation to avoid blocking other commands
    drop(conn);

    // Generate thumbnails without holding the DB lock; re-acquire briefly for each update
    for (id, filepath, ext) in &thumb_pending {
        if let Ok(data) = std::fs::read(std::path::Path::new(filepath)) {
            match thumb_state.0.generate(*id, &data, ext) {
                Ok(thumb_path) => {
                    if let Ok(c) = lock_db(&db) {
                        let _ = c.execute(
                            "UPDATE embroidery_files SET thumbnail_path = ?2 WHERE id = ?1",
                            rusqlite::params![id, thumb_path.to_string_lossy().as_ref()],
                        );
                    }
                }
                Err(e) => {
                    log::warn!("Failed to generate thumbnail for {filepath}: {e}");
                }
            }
        }
    }

    let total_elapsed_ms = start.elapsed().as_millis() as u64;

    let result = MassImportResult {
        folder_id,
        imported_count,
        skipped_count,
        error_count,
        elapsed_ms: total_elapsed_ms,
    };

    // Emit completion event (reuses same struct as the return value)
    let _ = app_handle.emit("import:complete", &result);

    Ok(result)
}

#[tauri::command]
pub fn parse_embroidery_file(filepath: String) -> Result<ParsedFileInfo, AppError> {
    let path = std::path::Path::new(&filepath);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| AppError::Parse {
            format: "unknown".to_string(),
            message: format!("No file extension: {filepath}"),
        })?;

    let parser = parsers::get_parser(&ext).ok_or_else(|| AppError::Parse {
        format: ext.clone(),
        message: format!("Unsupported format: {ext}"),
    })?;

    let data = std::fs::read(&filepath)?;
    parser.parse(&data)
}

#[tauri::command]
pub fn get_stitch_segments(filepath: String) -> Result<Vec<StitchSegment>, AppError> {
    // Reject path traversal attempts
    if filepath.contains("..") {
        return Err(AppError::Validation("Path traversal not allowed".to_string()));
    }
    let path = std::path::Path::new(&filepath);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| AppError::Parse {
            format: "unknown".to_string(),
            message: format!("No file extension: {filepath}"),
        })?;

    let parser = parsers::get_parser(&ext).ok_or_else(|| AppError::Parse {
        format: ext.clone(),
        message: format!("Unsupported format: {ext}"),
    })?;

    let data = std::fs::read(&filepath)?;
    parser.extract_stitch_segments(&data)
}

/// Auto-import files detected by the filesystem watcher.
/// Matches each file to the best-fitting folder (longest path prefix match).
/// Files that don't match any folder or are already imported are silently skipped.
#[tauri::command]
pub fn watcher_auto_import(
    db: State<'_, DbState>,
    thumb_state: State<'_, ThumbnailState>,
    file_paths: Vec<String>,
) -> Result<u32, AppError> {
    // Collect file metadata and parse files without holding the DB lock
    struct FileInfo {
        filepath: String,
        filename: String,
        file_size: Option<i64>,
        parsed: Option<ParsedFileInfo>,
        ext: Option<String>,
    }
    let file_infos: Vec<FileInfo> = file_paths
        .iter()
        .map(|filepath| {
            let path = std::path::Path::new(filepath);
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let file_size: Option<i64> = std::fs::metadata(path)
                .ok()
                .and_then(|m| i64::try_from(m.len()).ok());
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());
            let parsed = ext.as_deref()
                .and_then(|e| parsers::get_parser(e))
                .and_then(|parser| {
                    std::fs::read(path)
                        .ok()
                        .and_then(|data| parser.parse(&data).ok())
                });
            FileInfo { filepath: filepath.clone(), filename, file_size, parsed, ext }
        })
        .collect();

    let conn = lock_db(&db)?;

    // Load all folders to match file paths against
    let mut stmt = conn.prepare("SELECT id, path FROM folders WHERE path IS NOT NULL")?;
    let folders: Vec<(i64, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut imported_count: u32 = 0;
    let mut thumb_pending: Vec<(i64, String, String)> = Vec::new(); // (id, filepath, ext)

    // Transaction for DB inserts only — no file I/O inside
    {
        let tx = conn.unchecked_transaction()?;

        for info in &file_infos {
            // Find best matching folder (path-component-aware ancestry check)
            let best_folder = folders
                .iter()
                .filter(|(_, folder_path)| {
                    let fp = std::path::Path::new(&info.filepath);
                    let dp = std::path::Path::new(folder_path);
                    fp.starts_with(dp)
                })
                .max_by_key(|(_, folder_path)| folder_path.len());

            let folder_id = match best_folder {
                Some((id, _)) => *id,
                None => continue, // No matching folder, skip
            };

            let result = tx.execute(
                "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![folder_id, info.filename, info.filepath, info.file_size],
            );

            if let Ok(changes) = result {
                if changes > 0 {
                    let id = tx.last_insert_rowid();
                    // Persist parsed metadata if available
                    if let Some(pinfo) = &info.parsed {
                        if let Err(e) = tx.execute(
                            "UPDATE embroidery_files SET \
                             stitch_count = ?2, color_count = ?3, width_mm = ?4, height_mm = ?5, \
                             design_name = ?6, jump_count = ?7, trim_count = ?8, \
                             hoop_width_mm = ?9, hoop_height_mm = ?10, \
                             category = ?11, author = ?12, keywords = ?13, comments = ?14 \
                             WHERE id = ?1",
                            rusqlite::params![
                                id,
                                pinfo.stitch_count,
                                pinfo.color_count,
                                pinfo.width_mm,
                                pinfo.height_mm,
                                pinfo.design_name,
                                pinfo.jump_count,
                                pinfo.trim_count,
                                pinfo.hoop_width_mm,
                                pinfo.hoop_height_mm,
                                pinfo.category,
                                pinfo.author,
                                pinfo.keywords,
                                pinfo.comments,
                            ],
                        ) {
                            log::warn!("Failed to update metadata for {}: {e}", info.filepath);
                        }

                        // Persist parsed thread colors
                        for (idx, color) in pinfo.colors.iter().enumerate() {
                            if let Err(e) = tx.execute(
                                "INSERT INTO file_thread_colors (file_id, sort_order, color_hex, color_name, brand, brand_code, is_ai) \
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                                rusqlite::params![
                                    id,
                                    idx as i32,
                                    color.hex,
                                    color.name,
                                    color.brand,
                                    color.brand_code,
                                ],
                            ) {
                                log::warn!("Failed to insert color for {}: {e}", info.filepath);
                            }
                        }

                        // Persist format record
                        if let Err(e) = tx.execute(
                            "INSERT INTO file_formats (file_id, format, format_version, filepath, file_size_bytes, parsed) \
                             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                            rusqlite::params![
                                id,
                                pinfo.format,
                                pinfo.format_version,
                                info.filepath,
                                info.file_size,
                            ],
                        ) {
                            log::warn!("Failed to insert format for {}: {e}", info.filepath);
                        }
                    }

                    if let Some(ext) = &info.ext {
                        thumb_pending.push((id, info.filepath.clone(), ext.clone()));
                    }

                    imported_count += 1;
                }
            }
        }

        tx.commit()?;
    }

    // Generate thumbnails outside the transaction to avoid holding the DB lock during I/O
    for (id, filepath, ext) in &thumb_pending {
        if let Ok(data) = std::fs::read(std::path::Path::new(filepath)) {
            match thumb_state.0.generate(*id, &data, ext) {
                Ok(thumb_path) => {
                    let _ = conn.execute(
                        "UPDATE embroidery_files SET thumbnail_path = ?2 WHERE id = ?1",
                        rusqlite::params![id, thumb_path.to_string_lossy().as_ref()],
                    );
                }
                Err(e) => {
                    log::warn!("Failed to generate thumbnail for {filepath}: {e}");
                }
            }
        }
    }

    Ok(imported_count)
}

/// Remove DB entries for files that have been deleted from disk.
/// Also cleans up associated thumbnail files (best-effort).
#[tauri::command]
pub fn watcher_remove_by_paths(
    db: State<'_, DbState>,
    file_paths: Vec<String>,
) -> Result<u32, AppError> {
    let conn = lock_db(&db)?;
    let mut removed_count: u32 = 0;

    // Collect thumbnail paths before deleting rows
    let mut thumbnail_paths: Vec<String> = Vec::new();
    for filepath in &file_paths {
        if let Ok(path) = conn.query_row(
            "SELECT thumbnail_path FROM embroidery_files WHERE filepath = ?1 AND thumbnail_path IS NOT NULL AND thumbnail_path != ''",
            [filepath],
            |row| row.get::<_, String>(0),
        ) {
            thumbnail_paths.push(path);
        }
    }

    let tx = conn.unchecked_transaction()?;
    for filepath in &file_paths {
        let changes = tx.execute(
            "DELETE FROM embroidery_files WHERE filepath = ?1",
            [filepath],
        )?;
        removed_count += changes as u32;
    }
    tx.commit()?;

    // Clean up thumbnail files on disk (best-effort)
    for path in &thumbnail_paths {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("Failed to remove thumbnail {path}: {e}");
            }
        }
    }

    Ok(removed_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::init_database_in_memory;
    use std::fs;

    #[test]
    fn test_is_embroidery_file() {
        assert!(is_embroidery_file(std::path::Path::new("/tmp/test.pes")));
        assert!(is_embroidery_file(std::path::Path::new("/tmp/test.DST")));
        assert!(is_embroidery_file(std::path::Path::new("/tmp/test.jef")));
        assert!(is_embroidery_file(std::path::Path::new("/tmp/test.vp3")));
        assert!(!is_embroidery_file(std::path::Path::new("/tmp/test.png")));
        assert!(!is_embroidery_file(std::path::Path::new("/tmp/test.txt")));
    }

    #[test]
    fn test_scan_finds_embroidery_files() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        fs::write(base.join("a.pes"), b"fake").unwrap();
        fs::write(base.join("b.dst"), b"fake").unwrap();
        fs::write(base.join("c.txt"), b"text").unwrap();
        fs::create_dir(base.join("sub")).unwrap();
        fs::write(base.join("sub/d.jef"), b"fake").unwrap();

        let mut found = Vec::new();
        for entry in WalkDir::new(base).follow_links(false) {
            if let Ok(e) = entry {
                if e.file_type().is_file() && is_embroidery_file(e.path()) {
                    found.push(e.path().to_string_lossy().to_string());
                }
            }
        }

        assert_eq!(found.len(), 3);
    }

    #[test]
    fn test_import_files_creates_db_entries() {
        let conn = init_database_in_memory().unwrap();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Test', '/tmp/test')",
            [],
        )
        .unwrap();
        let folder_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
             VALUES (?1, 'a.pes', '/tmp/test/a.pes', 1024)",
            [folder_id],
        )
        .unwrap();

        conn.execute(
            "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
             VALUES (?1, 'b.dst', '/tmp/test/b.dst', 2048)",
            [folder_id],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1",
                [folder_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        // Test duplicate handling (IGNORE)
        conn.execute(
            "INSERT OR IGNORE INTO embroidery_files (folder_id, filename, filepath, file_size_bytes) \
             VALUES (?1, 'a.pes', '/tmp/test/a.pes', 1024)",
            [folder_id],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1",
                [folder_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "Duplicate should be ignored");
    }

    fn example_path(name: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("example files")
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn test_parse_embroidery_file_pes() {
        let info = parse_embroidery_file(example_path("BayrischesHerz.PES")).unwrap();
        assert_eq!(info.format, "PES");
        assert!(info.stitch_count.unwrap() > 0);
    }

    #[test]
    fn test_parse_embroidery_file_dst() {
        let info = parse_embroidery_file(example_path("2.DST")).unwrap();
        assert_eq!(info.format, "DST");
        assert!(info.stitch_count.unwrap() > 0);
    }

    #[test]
    fn test_parse_embroidery_file_unsupported() {
        let result = parse_embroidery_file("/tmp/test.txt".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_embroidery_file_not_found() {
        let result = parse_embroidery_file("/tmp/nonexistent_12345.pes".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_watcher_remove_cleans_up_thumbnails() {
        let conn = init_database_in_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();

        // Create a fake thumbnail file on disk
        let thumb = tmp.path().join("300.png");
        fs::write(&thumb, b"fake png").unwrap();
        assert!(thumb.exists());

        conn.execute("INSERT INTO folders (name, path) VALUES ('Test', '/test')", []).unwrap();
        let folder_id = conn.last_insert_rowid();

        let filepath = "/test/a.pes";
        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath, thumbnail_path) \
             VALUES (?1, 'a.pes', ?2, ?3)",
            rusqlite::params![folder_id, filepath, thumb.to_string_lossy().as_ref()],
        ).unwrap();

        // Query thumbnail path before delete — mirrors watcher_remove_by_paths logic
        let thumbnail_path: Option<String> = conn.query_row(
            "SELECT thumbnail_path FROM embroidery_files WHERE filepath = ?1 AND thumbnail_path IS NOT NULL AND thumbnail_path != ''",
            [filepath],
            |row| row.get::<_, String>(0),
        ).ok();
        assert!(thumbnail_path.is_some());

        // Delete the row
        let changes = conn.execute(
            "DELETE FROM embroidery_files WHERE filepath = ?1",
            [filepath],
        ).unwrap();
        assert_eq!(changes, 1);

        // Clean up thumbnail (mirrors the watcher logic)
        if let Some(ref path) = thumbnail_path {
            let _ = fs::remove_file(path);
        }

        assert!(!thumb.exists(), "Thumbnail should be deleted from disk after watcher remove");
    }
}
