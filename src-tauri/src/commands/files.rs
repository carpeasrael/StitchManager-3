use tauri::State;

use crate::{DbState, ThumbnailState};
use crate::db::models::{EmbroideryFile, FileAttachment, FileFormat, FileThreadColor, FileUpdate, PaginatedFiles, SearchParams, Tag};
use crate::db::queries::{FILE_SELECT, FILE_SELECT_ALIASED, FILE_SELECT_LIVE_BY_ID, row_to_file};
use crate::error::{lock_db, AppError};

/// Escape SQL LIKE wildcard characters in user input.
fn escape_like(input: &str) -> String {
    input.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Build WHERE clause conditions from query parameters.
/// Shared by both `query_files_impl` and `get_files_paginated`.
fn build_query_conditions(
    conn: &rusqlite::Connection,
    folder_id: Option<i64>,
    search: Option<String>,
    format_filter: Option<String>,
    search_params: Option<SearchParams>,
    conditions: &mut Vec<String>,
    params: &mut Vec<Box<dyn rusqlite::types::ToSql>>,
    param_idx: &mut usize,
) {
    // Always exclude soft-deleted files from normal queries
    conditions.push("e.deleted_at IS NULL".to_string());

    if let Some(fid) = folder_id {
        conditions.push(format!("e.folder_id = ?{}", *param_idx));
        params.push(Box::new(fid));
        *param_idx += 1;
    }

    // Determine the text query: prefer search_params.text, fall back to legacy `search`
    let text_query = search_params
        .as_ref()
        .and_then(|sp| sp.text.clone())
        .or(search);

    if let Some(ref q) = text_query {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            // Try FTS5 first; fall back to LIKE if FTS table doesn't exist
            let fts_exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='files_fts'",
                [],
                |row| row.get(0),
            ).unwrap_or(false);

            if fts_exists {
                // Strip all FTS5 special characters to prevent query injection
                let sanitized: String = trimmed.chars()
                    .filter(|c| !matches!(c, '"' | '*' | '+' | '-' | '^' | '(' | ')' | '{' | '}' | ':'))
                    .collect();
                if !sanitized.is_empty() {
                    let fts_query = format!("\"{sanitized}\"*");
                    conditions.push(format!(
                        "e.id IN (SELECT rowid FROM files_fts WHERE files_fts MATCH ?{})", *param_idx
                    ));
                    params.push(Box::new(fts_query));
                    *param_idx += 1;
                }
                // If sanitized is empty (all special chars), skip — no condition added
            } else {
                let escaped = escape_like(trimmed);
                let like_val = format!("%{escaped}%");
                let text_fields = [
                    "e.name", "e.filename", "e.theme", "e.description",
                    "e.design_name", "e.category", "e.author", "e.keywords",
                    "e.comments", "e.license", "e.unique_id",
                    "e.language", "e.file_source", "e.size_range",
                ];
                let clauses: Vec<String> = text_fields
                    .iter()
                    .map(|f| format!("{f} LIKE ?{} ESCAPE '\\\\'", *param_idx))
                    .collect();
                conditions.push(format!("({})", clauses.join(" OR ")));
                params.push(Box::new(like_val));
                *param_idx += 1;
            }
        }
    }

    if let Some(ref fmt) = format_filter {
        let trimmed = fmt.trim();
        if !trimmed.is_empty() {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM file_formats ff WHERE ff.file_id = e.id AND ff.format = ?{})", *param_idx
            ));
            params.push(Box::new(trimmed.to_uppercase()));
            *param_idx += 1;
        }
    }

    // Advanced search params
    if let Some(ref sp) = search_params {
        if let Some(ref tags) = sp.tags {
            for tag_name in tags {
                let trimmed = tag_name.trim();
                if !trimmed.is_empty() {
                    conditions.push(format!(
                        "EXISTS (SELECT 1 FROM file_tags ft JOIN tags t ON t.id = ft.tag_id \
                         WHERE ft.file_id = e.id AND t.name = ?{})", *param_idx
                    ));
                    params.push(Box::new(trimmed.to_string()));
                    *param_idx += 1;
                }
            }
        }

        if let Some(v) = sp.stitch_count_min {
            conditions.push(format!("e.stitch_count >= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.stitch_count_max {
            conditions.push(format!("e.stitch_count <= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.color_count_min {
            conditions.push(format!("e.color_count >= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.color_count_max {
            conditions.push(format!("e.color_count <= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.width_mm_min {
            conditions.push(format!("e.width_mm >= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.width_mm_max {
            conditions.push(format!("e.width_mm <= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.height_mm_min {
            conditions.push(format!("e.height_mm >= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.height_mm_max {
            conditions.push(format!("e.height_mm <= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.file_size_min {
            conditions.push(format!("e.file_size_bytes >= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.file_size_max {
            conditions.push(format!("e.file_size_bytes <= ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.ai_analyzed {
            conditions.push(format!("e.ai_analyzed = ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(v) = sp.ai_confirmed {
            conditions.push(format!("e.ai_confirmed = ?{}", *param_idx));
            params.push(Box::new(v));
            *param_idx += 1;
        }
        if let Some(ref cs) = sp.color_search {
            let trimmed = cs.trim();
            if !trimmed.is_empty() {
                let escaped = escape_like(trimmed);
                let like_val = format!("%{escaped}%");
                conditions.push(format!(
                    "EXISTS (SELECT 1 FROM file_thread_colors ftc WHERE ftc.file_id = e.id \
                     AND (ftc.color_name LIKE ?{pi} ESCAPE '\\' OR ftc.brand LIKE ?{pi} ESCAPE '\\'))",
                    pi = *param_idx
                ));
                params.push(Box::new(like_val));
                *param_idx += 1;
            }
        }
        if let Some(ref file_type) = sp.file_type {
            let trimmed = file_type.trim();
            if !trimmed.is_empty() {
                conditions.push(format!("e.file_type = ?{}", *param_idx));
                params.push(Box::new(trimmed.to_string()));
                *param_idx += 1;
            }
        }
        if let Some(ref status) = sp.status {
            let trimmed = status.trim();
            if !trimmed.is_empty() {
                conditions.push(format!("e.status = ?{}", *param_idx));
                params.push(Box::new(trimmed.to_string()));
                *param_idx += 1;
            }
        } else {
            // Exclude archived files by default when no explicit status filter
            conditions.push("e.status != 'archived'".to_string());
        }
        if let Some(ref skill_level) = sp.skill_level {
            let trimmed = skill_level.trim();
            if !trimmed.is_empty() {
                conditions.push(format!("e.skill_level = ?{}", *param_idx));
                params.push(Box::new(trimmed.to_string()));
                *param_idx += 1;
            }
        }
        if let Some(ref language) = sp.language {
            let trimmed = language.trim();
            if !trimmed.is_empty() {
                conditions.push(format!("e.language = ?{}", *param_idx));
                params.push(Box::new(trimmed.to_string()));
                *param_idx += 1;
            }
        }
        if let Some(ref file_source) = sp.file_source {
            let trimmed = file_source.trim();
            if !trimmed.is_empty() {
                conditions.push(format!("e.file_source = ?{}", *param_idx));
                params.push(Box::new(trimmed.to_string()));
                *param_idx += 1;
            }
        }
        if let Some(ref category) = sp.category {
            let trimmed = category.trim();
            if !trimmed.is_empty() {
                conditions.push(format!("e.category LIKE '%' || ?{} || '%' ESCAPE '\\'", *param_idx));
                params.push(Box::new(escape_like(trimmed)));
                *param_idx += 1;
            }
        }
        if let Some(ref author) = sp.author {
            let trimmed = author.trim();
            if !trimmed.is_empty() {
                conditions.push(format!("e.author LIKE '%' || ?{} || '%' ESCAPE '\\'", *param_idx));
                params.push(Box::new(escape_like(trimmed)));
                *param_idx += 1;
            }
        }
        if let Some(ref size_range) = sp.size_range {
            let trimmed = size_range.trim();
            if !trimmed.is_empty() {
                conditions.push(format!("e.size_range LIKE '%' || ?{} || '%' ESCAPE '\\'", *param_idx));
                params.push(Box::new(escape_like(trimmed)));
                *param_idx += 1;
            }
        }
        if let Some(rating_min) = sp.rating_min {
            conditions.push(format!("e.rating >= ?{}", *param_idx));
            params.push(Box::new(rating_min));
            *param_idx += 1;
        }
        if let Some(rating_max) = sp.rating_max {
            conditions.push(format!("e.rating <= ?{}", *param_idx));
            params.push(Box::new(rating_max));
            *param_idx += 1;
        }
        if let Some(is_fav) = sp.is_favorite {
            conditions.push(format!("e.is_favorite = ?{}", *param_idx));
            params.push(Box::new(is_fav as i32));
            *param_idx += 1;
        }
    }
}

/// Build a safe ORDER BY clause from search params.
fn build_order_clause(search_params: &Option<SearchParams>) -> String {
    let allowed = [
        "filename", "name", "created_at", "updated_at", "author", "category",
        "stitch_count", "color_count", "file_type", "status",
    ];
    if let Some(sp) = search_params {
        if let Some(ref field) = sp.sort_field {
            let f = field.trim();
            if allowed.contains(&f) {
                let dir = match sp.sort_direction.as_deref() {
                    Some("desc") => "DESC",
                    _ => "ASC",
                };
                return format!("ORDER BY e.{f} {dir}");
            }
        }
    }
    "ORDER BY e.filename ASC".to_string()
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

    let order = build_order_clause(&search_params);

    build_query_conditions(
        conn, folder_id, search, format_filter, search_params,
        &mut conditions, &mut params, &mut param_idx,
    );

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "{FILE_SELECT_ALIASED}{where_clause} {order}"
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
pub fn get_files_by_ids(
    db: State<'_, DbState>,
    file_ids: Vec<i64>,
) -> Result<Vec<EmbroideryFile>, AppError> {
    if file_ids.is_empty() {
        return Ok(Vec::new());
    }
    let conn = lock_db(&db)?;
    let placeholders: Vec<String> = file_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "{FILE_SELECT} WHERE id IN ({}) AND deleted_at IS NULL",
        placeholders.join(",")
    );
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = file_ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
    let mut stmt = conn.prepare(&sql)?;
    let files = stmt
        .query_map(param_refs.as_slice(), |row| row_to_file(row))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(files)
}

#[tauri::command]
pub fn get_files_paginated(
    db: State<'_, DbState>,
    folder_id: Option<i64>,
    search: Option<String>,
    format_filter: Option<String>,
    search_params: Option<SearchParams>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<PaginatedFiles, AppError> {
    let conn = lock_db(&db)?;
    let pg = page.unwrap_or(0);
    let ps = page_size.unwrap_or(200).max(1);

    // Build WHERE clause (reuse query_files_impl's condition logic)
    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx: usize = 1;

    let order = build_order_clause(&search_params);

    build_query_conditions(
        &conn, folder_id, search, format_filter, search_params,
        &mut conditions, &mut params, &mut param_idx,
    );

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    // COUNT query
    let count_sql = format!("SELECT COUNT(*) FROM embroidery_files e{where_clause}");
    let total_count: i64 = conn.query_row(&count_sql, param_refs.as_slice(), |row| row.get(0))?;

    // Paginated data query with LIMIT/OFFSET
    let data_sql = format!(
        "{FILE_SELECT_ALIASED}{where_clause} {order} LIMIT ?{param_idx} OFFSET ?{}",
        param_idx + 1
    );
    let mut data_params = params;
    data_params.push(Box::new(ps));
    data_params.push(Box::new(pg * ps));
    let data_refs: Vec<&dyn rusqlite::types::ToSql> = data_params.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&data_sql)?;
    let files = stmt
        .query_map(data_refs.as_slice(), |row| row_to_file(row))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PaginatedFiles { files, total_count, page: pg, page_size: ps })
}

#[tauri::command]
pub fn get_thumbnails_batch(
    db: State<'_, DbState>,
    thumb_state: State<'_, ThumbnailState>,
    file_ids: Vec<i64>,
) -> Result<std::collections::HashMap<i64, String>, AppError> {
    use base64::Engine;

    // Batch-load thumbnail paths from DB in one query
    if file_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let paths: Vec<(i64, Option<String>, String)> = {
        let conn = lock_db(&db)?;
        let placeholders: Vec<String> = file_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
        let sql = format!(
            "SELECT id, thumbnail_path, filepath FROM embroidery_files WHERE id IN ({}) AND deleted_at IS NULL",
            placeholders.join(",")
        );
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = file_ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows: Vec<_> = stmt.query_map(param_refs.as_slice(), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, String>(2)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        rows
    };

    let mut result = std::collections::HashMap::new();
    let mut generated_paths: Vec<(i64, String)> = Vec::new();

    for (file_id, thumbnail_path, filepath) in paths {
        // Try cached thumbnail first
        if let Some(ref path) = thumbnail_path {
            if !path.is_empty() && std::path::Path::new(path).exists() {
                if let Ok(data) = std::fs::read(path) {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    result.insert(file_id, format!("data:image/png;base64,{b64}"));
                    continue;
                }
            }
        }

        // On-demand generation
        let src_path = std::path::Path::new(&filepath);
        let ext = src_path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
        if ext.is_empty() { continue; }

        if let Ok(raw_data) = std::fs::read(src_path) {
            if let Ok(thumb_path) = thumb_state.0.generate(file_id, &raw_data, &ext) {
                generated_paths.push((file_id, thumb_path.to_string_lossy().to_string()));
                if let Ok(data) = std::fs::read(&thumb_path) {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    result.insert(file_id, format!("data:image/png;base64,{b64}"));
                }
            }
        }
    }

    // Batch-persist generated thumbnail paths in a single lock acquisition
    if !generated_paths.is_empty() {
        if let Ok(conn) = lock_db(&db) {
            for (file_id, path) in &generated_paths {
                let _ = conn.execute(
                    "UPDATE embroidery_files SET thumbnail_path = ?2 WHERE id = ?1",
                    rusqlite::params![file_id, path],
                );
            }
        }
    }

    Ok(result)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub total_files: i64,
    pub total_folders: i64,
    pub total_stitches: i64,
    pub format_counts: std::collections::HashMap<String, i64>,
}

#[tauri::command]
pub fn get_recent_files(
    db: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<Vec<EmbroideryFile>, AppError> {
    let conn = lock_db(&db)?;
    let lim = limit.unwrap_or(20);
    let sql = format!("{FILE_SELECT} WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let files = stmt
        .query_map([lim], |row| row_to_file(row))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(files)
}

#[tauri::command]
pub fn get_favorite_files(
    db: State<'_, DbState>,
) -> Result<Vec<EmbroideryFile>, AppError> {
    let conn = lock_db(&db)?;
    let sql = format!("{FILE_SELECT} WHERE is_favorite = 1 AND deleted_at IS NULL ORDER BY updated_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let files = stmt
        .query_map([], |row| row_to_file(row))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(files)
}

#[tauri::command]
pub fn toggle_favorite(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<bool, AppError> {
    let conn = lock_db(&db)?;
    let current: bool = conn.query_row(
        "SELECT is_favorite FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
        [file_id],
        |row| row.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Datei {file_id} nicht gefunden")),
        other => AppError::Database(other),
    })?;
    let new_val = !current;
    conn.execute(
        "UPDATE embroidery_files SET is_favorite = ?2 WHERE id = ?1 AND deleted_at IS NULL",
        rusqlite::params![file_id, new_val],
    )?;
    Ok(new_val)
}

#[tauri::command]
pub fn get_library_stats(
    db: State<'_, DbState>,
) -> Result<LibraryStats, AppError> {
    let conn = lock_db(&db)?;

    let total_files: i64 = conn.query_row(
        "SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL", [], |r| r.get(0)
    )?;
    let total_folders: i64 = conn.query_row("SELECT COUNT(*) FROM folders", [], |r| r.get(0))?;
    let total_stitches: i64 = conn.query_row(
        "SELECT COALESCE(SUM(stitch_count), 0) FROM embroidery_files WHERE deleted_at IS NULL", [], |r| r.get(0)
    )?;

    let mut stmt = conn.prepare(
        "SELECT ff.format, COUNT(*) FROM file_formats ff \
         JOIN embroidery_files e ON e.id = ff.file_id \
         WHERE e.deleted_at IS NULL \
         GROUP BY ff.format ORDER BY COUNT(*) DESC"
    )?;
    let format_counts: std::collections::HashMap<String, i64> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(LibraryStats { total_files, total_folders, total_stitches, format_counts })
}

#[tauri::command]
pub fn get_file(
    db: State<'_, DbState>,
    file_id: i64,
) -> Result<EmbroideryFile, AppError> {
    let conn = lock_db(&db)?;

    conn.query_row(
        &format!("{FILE_SELECT_LIVE_BY_ID}"),
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

/// Audit Wave 1: per-field length caps applied in `update_file`.
const MAX_TEXT_FIELD: usize = 1024;
const MAX_LINK_FIELD: usize = 2048;

/// Strict YYYY-MM-DD parser used by `update_file` to reject free-form dates.
fn is_valid_iso_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    bytes
        .iter()
        .enumerate()
        .all(|(i, &b)| if matches!(i, 4 | 7) { b == b'-' } else { b.is_ascii_digit() })
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
        && updates.size_range.is_none()
        && updates.skill_level.is_none()
        && updates.language.is_none()
        && updates.format_type.is_none()
        && updates.file_source.is_none()
        && updates.purchase_link.is_none()
        && updates.status.is_none()
        && updates.author.is_none()
        && updates.instructions_html.is_none()
        && updates.pattern_date.is_none()
        && updates.rating.is_none()
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
    if let Some(ref size_range) = updates.size_range {
        set_clauses.push(format!("size_range = ?{idx}"));
        params.push(Box::new(size_range.clone()));
        idx += 1;
    }
    if let Some(ref skill_level) = updates.skill_level {
        if !skill_level.is_empty() {
            let valid = ["beginner", "easy", "intermediate", "advanced", "expert"];
            if !valid.contains(&skill_level.as_str()) {
                return Err(AppError::Validation(format!("Ungültiges Schwierigkeitslevel: {skill_level}")));
            }
        }
        set_clauses.push(format!("skill_level = ?{idx}"));
        params.push(Box::new(skill_level.clone()));
        idx += 1;
    }
    if let Some(ref language) = updates.language {
        if language.len() > MAX_TEXT_FIELD {
            return Err(AppError::Validation("Sprache zu lang".into()));
        }
        set_clauses.push(format!("language = ?{idx}"));
        params.push(Box::new(language.clone()));
        idx += 1;
    }
    if let Some(ref format_type) = updates.format_type {
        if format_type.len() > MAX_TEXT_FIELD {
            return Err(AppError::Validation("Formattyp zu lang".into()));
        }
        set_clauses.push(format!("format_type = ?{idx}"));
        params.push(Box::new(format_type.clone()));
        idx += 1;
    }
    if let Some(ref file_source) = updates.file_source {
        if file_source.len() > MAX_TEXT_FIELD {
            return Err(AppError::Validation("Quelle zu lang".into()));
        }
        set_clauses.push(format!("file_source = ?{idx}"));
        params.push(Box::new(file_source.clone()));
        idx += 1;
    }
    if let Some(ref purchase_link) = updates.purchase_link {
        if !purchase_link.is_empty() {
            if purchase_link.len() > MAX_LINK_FIELD {
                return Err(AppError::Validation("Kaufquelle-URL zu lang".into()));
            }
            let scheme_ok = purchase_link.starts_with("http://") || purchase_link.starts_with("https://");
            if !scheme_ok {
                return Err(AppError::Validation(
                    "Kaufquelle muss mit http:// oder https:// beginnen".into(),
                ));
            }
        }
        set_clauses.push(format!("purchase_link = ?{idx}"));
        params.push(Box::new(purchase_link.clone()));
        idx += 1;
    }
    if let Some(ref status) = updates.status {
        let valid = ["none", "not_started", "planned", "in_progress", "completed", "archived"];
        if !valid.contains(&status.as_str()) {
            return Err(AppError::Validation(format!("Ungültiger Status: {status}")));
        }
        set_clauses.push(format!("status = ?{idx}"));
        params.push(Box::new(status.clone()));
        idx += 1;
    }
    if let Some(ref author) = updates.author {
        set_clauses.push(format!("author = ?{idx}"));
        params.push(Box::new(author.clone()));
        idx += 1;
    }
    if let Some(ref instructions_html) = updates.instructions_html {
        if instructions_html.len() > 100 * 1024 {
            return Err(AppError::Validation("Anleitung ist zu lang (max. 100 KB)".into()));
        }
        set_clauses.push(format!("instructions_html = ?{idx}"));
        params.push(Box::new(sanitize_html(instructions_html)));
        idx += 1;
    }
    if let Some(ref pattern_date) = updates.pattern_date {
        if !pattern_date.is_empty() && !is_valid_iso_date(pattern_date) {
            return Err(AppError::Validation(
                "Musterdatum muss im Format YYYY-MM-DD vorliegen".into(),
            ));
        }
        set_clauses.push(format!("pattern_date = ?{idx}"));
        params.push(Box::new(pattern_date.clone()));
        idx += 1;
    }
    if let Some(rating) = updates.rating {
        if rating == 0 {
            // 0 = clear rating (set to NULL)
            set_clauses.push(format!("rating = NULL"));
        } else if rating >= 1 && rating <= 5 {
            set_clauses.push(format!("rating = ?{idx}"));
            params.push(Box::new(rating));
            idx += 1;
        } else {
            return Err(AppError::Validation(format!("Bewertung muss zwischen 1 und 5 liegen: {rating}")));
        }
    }

    set_clauses.push(format!("updated_at = datetime('now')"));

    let sql = format!(
        "UPDATE embroidery_files SET {} WHERE id = ?{idx} AND deleted_at IS NULL",
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
        &format!("{FILE_SELECT_LIVE_BY_ID}"),
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
            "SELECT thumbnail_path FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
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
pub fn update_file_status(
    db: State<'_, DbState>,
    file_id: i64,
    status: String,
) -> Result<EmbroideryFile, AppError> {
    let valid = ["none", "not_started", "planned", "in_progress", "completed", "archived"];
    if !valid.contains(&status.as_str()) {
        return Err(AppError::Validation(format!("Ungültiger Status: {status}")));
    }
    let conn = lock_db(&db)?;
    let changes = conn.execute(
        "UPDATE embroidery_files SET status = ?2, updated_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
        rusqlite::params![file_id, status],
    )?;
    if changes == 0 {
        return Err(AppError::NotFound(format!("Datei {file_id} nicht gefunden")));
    }
    conn.query_row(
        &format!("{FILE_SELECT_LIVE_BY_ID}"),
        [file_id],
        |row| row_to_file(row),
    )
    .map_err(AppError::Database)
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
        "SELECT COUNT(*) > 0 FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
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
            "SELECT thumbnail_path, filepath FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
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

// ── Pattern Thumbnail from Preview (#120) ────────────────────────────

/// Save a base64-encoded PNG as the thumbnail for a file.
/// Used by the frontend after rendering a PDF first page or loading an image preview.
#[tauri::command]
pub fn save_thumbnail_data(
    db: State<'_, DbState>,
    thumb_state: State<'_, ThumbnailState>,
    file_id: i64,
    png_base64: String,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
        [file_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!("Datei {file_id} nicht gefunden")));
    }

    // Decode base64
    let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &png_base64)
        .map_err(|e| AppError::Validation(format!("Ungueltige Base64-Daten: {e}")))?;

    // Load and resize to 192x192
    let img = image::load_from_memory(&data)
        .map_err(|e| AppError::Internal(format!("Bild konnte nicht geladen werden: {e}")))?;
    let thumb = img.thumbnail(192, 192);

    // Save to thumbnail cache
    let thumb_path = thumb_state.0.thumbnail_path(file_id);
    if let Some(parent) = thumb_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    thumb.save_with_format(&thumb_path, image::ImageFormat::Png)
        .map_err(|e| AppError::Internal(format!("Thumbnail konnte nicht gespeichert werden: {e}")))?;

    let thumb_path_str = thumb_path.to_string_lossy().to_string();

    // Update DB
    conn.execute(
        "UPDATE embroidery_files SET thumbnail_path = ?1, updated_at = datetime('now') WHERE id = ?2 AND deleted_at IS NULL",
        rusqlite::params![thumb_path_str, file_id],
    )?;

    Ok(())
}

// ── Sewing Pattern Upload (#119) ─────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternMetadata {
    pub name: Option<String>,
    pub license: Option<String>,
    pub designer: Option<String>,
    pub source: Option<String>,
    pub description: Option<String>,
    pub instructions_html: Option<String>,
    pub pattern_date: Option<String>,
    pub skill_level: Option<String>,
    pub rating: Option<i32>,
}

const PATTERN_EXTENSIONS: &[&str] = &["pdf", "png", "jpg", "jpeg", "bmp"];

/// HTML sanitization for rich text instructions (#124, audit Wave 1).
/// Backed by the `ammonia` crate to avoid hand-rolled parsing footguns.
/// Strips every tag/attribute outside the allow-list, normalises entity
/// encoding, and rejects URL schemes other than http/https on any surviving
/// hrefs. The result is safe to assign to `innerHTML`.
fn sanitize_html(html: &str) -> String {
    use std::collections::{HashMap, HashSet};
    let mut tags: HashSet<&str> = HashSet::new();
    for t in ["b", "i", "u", "strong", "em", "ul", "ol", "li", "p", "br", "div", "span"] {
        tags.insert(t);
    }
    let attrs: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut url_schemes: HashSet<&str> = HashSet::new();
    url_schemes.insert("http");
    url_schemes.insert("https");

    ammonia::Builder::default()
        .tags(tags)
        .tag_attributes(attrs)
        .url_schemes(url_schemes)
        .clean(html)
        .to_string()
}

#[tauri::command]
pub fn upload_sewing_pattern(
    db: State<'_, DbState>,
    source_path: String,
    collection_id: Option<i64>,
    metadata: PatternMetadata,
) -> Result<EmbroideryFile, AppError> {
    super::validate_no_traversal(&source_path)?;

    let src = std::path::Path::new(&source_path);
    if !src.exists() {
        return Err(AppError::NotFound(format!("Datei nicht gefunden: {source_path}")));
    }

    // Validate extension
    let ext = src.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
    if !PATTERN_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "Nicht unterstuetztes Dateiformat: .{ext}. Erlaubt: PDF, PNG, JPG, BMP"
        )));
    }

    // Validate file size (100 MB limit)
    let file_size = std::fs::metadata(src)?.len();
    if file_size > 100 * 1024 * 1024 {
        return Err(AppError::Validation("Datei ist groesser als 100 MB".into()));
    }

    // Validate instructions_html length (100 KB limit)
    if let Some(ref html) = metadata.instructions_html {
        if html.len() > 100 * 1024 {
            return Err(AppError::Validation("Anleitung ist zu lang (max. 100 KB)".into()));
        }
    }

    // Validate rating
    if let Some(r) = metadata.rating {
        if r < 1 || r > 5 {
            return Err(AppError::Validation(format!("Bewertung muss zwischen 1 und 5 liegen: {r}")));
        }
    }

    // Validate skill_level
    if let Some(ref sl) = metadata.skill_level {
        if !sl.is_empty() {
            let valid = ["beginner", "easy", "intermediate", "advanced", "expert"];
            if !valid.contains(&sl.as_str()) {
                return Err(AppError::Validation(format!("Ungueltiges Schwierigkeitslevel: {sl}")));
            }
        }
    }

    let filename = src.file_name().and_then(|n| n.to_str()).unwrap_or("pattern").to_string();
    let stem = std::path::Path::new(&filename).file_stem().and_then(|s| s.to_str()).unwrap_or("pattern");
    let display_name = metadata.name.as_deref().unwrap_or(stem).to_string();

    let conn = lock_db(&db)?;

    // Resolve library_root
    let library_root: String = conn
        .query_row("SELECT value FROM settings WHERE key = 'library_root'", [], |row| row.get(0))
        .map_err(|_| AppError::Validation("library_root ist nicht konfiguriert".into()))?;

    let base_dir = if library_root.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&library_root[2..])
        } else {
            std::path::PathBuf::from(&library_root)
        }
    } else {
        std::path::PathBuf::from(&library_root)
    };

    let pattern_dir = base_dir.join(".schnittmuster");
    std::fs::create_dir_all(&pattern_dir)?;

    // Ensure folder record exists
    let folder_path = pattern_dir.to_string_lossy().to_string();
    let folder_id: i64 = match conn.query_row(
        "SELECT id FROM folders WHERE path = ?1",
        [&folder_path],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            conn.execute(
                "INSERT INTO folders (name, path) VALUES ('Schnittmuster', ?1)",
                [&folder_path],
            )?;
            conn.last_insert_rowid()
        }
        Err(e) => return Err(AppError::Database(e)),
    };

    // Deduplicate filename
    let mut dest = pattern_dir.join(&filename);
    if dest.exists() {
        let ext_str = src.extension().and_then(|e| e.to_str()).unwrap_or("");
        for i in 1..=100_000 {
            let candidate = if ext_str.is_empty() {
                format!("{stem}_{i}")
            } else {
                format!("{stem}_{i}.{ext_str}")
            };
            dest = pattern_dir.join(&candidate);
            if !dest.exists() { break; }
        }
    }

    // Copy file
    std::fs::copy(src, &dest)?;
    let dest_path = dest.to_string_lossy().to_string();

    // Generate unique_id
    let unique_id = crate::db::migrations::generate_unique_id();

    // Insert DB record
    conn.execute(
        "INSERT INTO embroidery_files (folder_id, filename, filepath, name, file_type, status, unique_id, \
         description, license, author, file_source, skill_level, \
         instructions_html, pattern_date, rating, file_size_bytes) \
         VALUES (?1, ?2, ?3, ?4, 'sewing_pattern', 'none', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            folder_id,
            dest.file_name().and_then(|n| n.to_str()).unwrap_or(&filename),
            dest_path,
            display_name,
            unique_id,
            metadata.description,
            metadata.license,
            metadata.designer,    // -> author column
            metadata.source,      // -> file_source column
            metadata.skill_level,
            metadata.instructions_html.as_deref().map(sanitize_html),
            metadata.pattern_date,
            metadata.rating,
            file_size as i64,
        ],
    )?;
    let file_id = conn.last_insert_rowid();

    // Link to collection if requested
    if let Some(cid) = collection_id {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO collection_items (collection_id, file_id) VALUES (?1, ?2)",
            rusqlite::params![cid, file_id],
        );
    }

    // Return the created record
    let sql = format!("{} WHERE id = ?1 AND deleted_at IS NULL", crate::db::queries::FILE_SELECT);
    conn.query_row(&sql, [file_id], crate::db::queries::row_to_file)
        .map_err(AppError::Database)
}

/// Attach a file to an embroidery file entry.
#[tauri::command]
pub fn attach_file(
    db: State<'_, DbState>,
    file_id: i64,
    source_path: String,
    attachment_type: String,
    display_name: Option<String>,
) -> Result<FileAttachment, AppError> {
    // Reject path traversal
    super::validate_no_traversal(&source_path)?;

    let src = std::path::Path::new(&source_path);
    if !src.exists() {
        return Err(AppError::NotFound(format!("Datei nicht gefunden: {source_path}")));
    }

    let filename = src
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment")
        .to_string();

    // Audit Wave 1: enforce an extension allow-list so attach + open cannot
    // be used to stage executables (.exe, .sh, .scpt, .command, .app …).
    let ext_lower = super::lower_ext(src);
    if !super::ATTACHMENT_EXTENSIONS.contains(&ext_lower.as_str()) {
        return Err(AppError::Validation(format!(
            "Anhang-Format nicht erlaubt: .{ext_lower}. Erlaubt: PDF, PNG, JPG, TXT, MD"
        )));
    }

    let mime_type = match ext_lower.as_str() {
        "pdf" => Some("application/pdf".to_string()),
        "png" => Some("image/png".to_string()),
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "txt" => Some("text/plain".to_string()),
        "md" => Some("text/markdown".to_string()),
        _ => None,
    };

    // Determine attachment storage directory under the configured library root.
    let conn = lock_db(&db)?;

    let base_dir = super::library_root(&conn)
        .ok_or_else(|| AppError::Validation("library_root ist nicht konfiguriert".into()))?;

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
        for counter in 1..=100_000u32 {
            let new_name = if ext.is_empty() {
                format!("{stem}_{counter}")
            } else {
                format!("{stem}_{counter}.{ext}")
            };
            dest = attach_dir.join(&new_name);
            if !dest.exists() {
                break;
            }
            if counter == 100_000 {
                return Err(AppError::Internal(
                    "Dateiname-Deduplizierung: Alle Suffixe erschoepft".into(),
                ));
            }
        }
    }
    std::fs::copy(src, &dest)?;

    let dest_str = dest.to_string_lossy().to_string();

    let actual_filename = dest.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&filename)
        .to_string();

    conn.execute(
        "INSERT INTO file_attachments (file_id, filename, mime_type, file_path, attachment_type, display_name) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![file_id, actual_filename, mime_type, dest_str, attachment_type, display_name],
    )?;

    let id = conn.last_insert_rowid();

    conn.query_row(
        "SELECT id, file_id, filename, mime_type, file_path, attachment_type, display_name, sort_order, created_at \
         FROM file_attachments WHERE id = ?1",
        [id],
        |row| Ok(FileAttachment {
            id: row.get(0)?,
            file_id: row.get(1)?,
            filename: row.get(2)?,
            mime_type: row.get(3)?,
            file_path: row.get(4)?,
            attachment_type: row.get(5)?,
            display_name: row.get(6)?,
            sort_order: row.get(7)?,
            created_at: row.get(8)?,
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
        "SELECT id, file_id, filename, mime_type, file_path, attachment_type, display_name, sort_order, created_at \
         FROM file_attachments WHERE file_id = ?1 ORDER BY sort_order, created_at",
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
                display_name: row.get(6)?,
                sort_order: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(attachments)
}

/// Delete an attachment (DB record + file on disk).
///
/// Audit Wave 1: enforce that the on-disk path lives under the configured
/// attachment directory before unlinking — guards against malicious DB rows
/// (e.g. from a hostile restored backup) pointing at arbitrary files.
#[tauri::command]
pub fn delete_attachment(
    db: State<'_, DbState>,
    attachment_id: i64,
) -> Result<(), AppError> {
    let conn = lock_db(&db)?;

    let (file_id, file_path): (i64, String) = conn
        .query_row(
            "SELECT file_id, file_path FROM file_attachments WHERE id = ?1",
            [attachment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Anhang {attachment_id} nicht gefunden"))
            }
            other => AppError::Database(other),
        })?;

    // Resolve the expected attachment directory under the current library_root.
    // We require the stored path to lie under it before any unlink.
    let containment_ok = match super::library_root(&conn) {
        Some(root) => {
            let expected = root
                .join(".stichman")
                .join("attachments")
                .join(file_id.to_string());
            super::ensure_under(std::path::Path::new(&file_path), &expected).is_ok()
        }
        None => false,
    };

    conn.execute("DELETE FROM file_attachments WHERE id = ?1", [attachment_id])?;

    if containment_ok {
        if let Err(e) = std::fs::remove_file(&file_path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("Failed to remove attachment file {file_path}: {e}");
            }
        }
    } else {
        log::warn!(
            "delete_attachment: refused to unlink path outside attachment dir: {file_path}"
        );
    }

    Ok(())
}

/// Open an attachment with the system default application.
///
/// Audit Wave 1: enforce containment under the per-file attachment directory
/// (`<library_root>/.stichman/attachments/<file_id>/`) **and** require the
/// extension to match the attachment allow-list, so a malicious DB row cannot
/// trick the OS opener into launching arbitrary binaries.
#[tauri::command]
pub fn open_attachment(
    db: State<'_, DbState>,
    attachment_id: i64,
) -> Result<(), AppError> {
    let (file_id, file_path, expected_dir) = {
        let conn = lock_db(&db)?;

        let (file_id, file_path): (i64, String) = conn
            .query_row(
                "SELECT file_id, file_path FROM file_attachments WHERE id = ?1",
                [attachment_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound(format!("Anhang {attachment_id} nicht gefunden"))
                }
                other => AppError::Database(other),
            })?;

        let expected = super::library_root(&conn)
            .ok_or_else(|| AppError::Validation("library_root ist nicht konfiguriert".into()))?
            .join(".stichman")
            .join("attachments")
            .join(file_id.to_string());

        (file_id, file_path, expected)
    };

    super::validate_no_traversal(&file_path)?;
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(AppError::NotFound(format!("Anhang-Datei nicht gefunden: {file_path}")));
    }
    if !path.is_file() {
        return Err(AppError::Validation(format!("Pfad ist keine regulaere Datei: {file_path}")));
    }

    // Enforcing containment check (no longer log-and-continue).
    super::ensure_under(path, &expected_dir)?;

    // Extension allow-list — defends against `.command`/`.scpt`/`.exe` payloads
    // that may have ended up in the attachment dir via a hostile import.
    let ext = super::lower_ext(path);
    if !super::ATTACHMENT_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "Anhang-Format nicht erlaubt zum Oeffnen: .{ext}"
        )));
    }

    let _ = file_id; // currently only used for the containment computation

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
                "SELECT COUNT(*) > 0 FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
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
                "SELECT thumbnail_path FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
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
                "SELECT thumbnail_path FROM embroidery_files WHERE id = ?1 AND deleted_at IS NULL",
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
