use std::collections::HashMap;
use serde::Serialize;
use tauri::State;
use crate::DbState;
use crate::error::{lock_db, AppError};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatusCounts {
    pub none: i64,
    pub analyzed: i64,
    pub confirmed: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStat {
    pub folder_name: String,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingMetadata {
    pub no_tags: i64,
    pub no_rating: i64,
    pub no_description: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub files_by_type: HashMap<String, i64>,
    pub ai_status: AiStatusCounts,
    pub top_folders: Vec<FolderStat>,
    pub missing_metadata: MissingMetadata,
    pub storage_by_folder: Vec<FolderStat>,
    pub recent_imports: i64,
}

#[tauri::command]
pub fn get_dashboard_stats(db: State<'_, DbState>) -> Result<DashboardStats, AppError> {
    let conn = lock_db(&db)?;

    // Files by type
    let mut files_by_type = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT file_type, COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL GROUP BY file_type",
        )?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?
            .filter_map(|r| r.ok());
        for (ft, count) in rows {
            files_by_type.insert(ft, count);
        }
    }

    // Audit Wave 2 perf: collapse 3 separate full-table scans into one
    // conditional-aggregate query.
    let (ai_none, ai_analyzed, ai_confirmed): (i64, i64, i64) = conn.query_row(
        "SELECT \
         SUM(CASE WHEN ai_analyzed = 0 THEN 1 ELSE 0 END), \
         SUM(CASE WHEN ai_analyzed = 1 AND ai_confirmed = 0 THEN 1 ELSE 0 END), \
         SUM(CASE WHEN ai_analyzed = 1 AND ai_confirmed = 1 THEN 1 ELSE 0 END) \
         FROM embroidery_files WHERE deleted_at IS NULL",
        [],
        |row| Ok((row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                  row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                  row.get::<_, Option<i64>>(2)?.unwrap_or(0))),
    )?;

    // Top 10 folders by file count
    let top_folders = {
        let mut stmt = conn.prepare(
            "SELECT f.name, COUNT(e.id) AS cnt FROM folders f \
             LEFT JOIN embroidery_files e ON e.folder_id = f.id AND e.deleted_at IS NULL \
             GROUP BY f.id ORDER BY cnt DESC LIMIT 10",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(FolderStat {
                    folder_name: row.get(0)?,
                    value: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        rows
    };

    // Audit Wave 2 perf: combine three missing-metadata counts into one query.
    let (no_tags, no_rating, no_description): (i64, i64, i64) = conn.query_row(
        "SELECT \
         SUM(CASE WHEN NOT EXISTS (SELECT 1 FROM file_tags WHERE file_id = e.id) THEN 1 ELSE 0 END), \
         SUM(CASE WHEN rating IS NULL THEN 1 ELSE 0 END), \
         SUM(CASE WHEN description IS NULL OR description = '' THEN 1 ELSE 0 END) \
         FROM embroidery_files e WHERE deleted_at IS NULL",
        [],
        |row| Ok((row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                  row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                  row.get::<_, Option<i64>>(2)?.unwrap_or(0))),
    )?;

    // Storage by folder
    let storage_by_folder = {
        let mut stmt = conn.prepare(
            "SELECT f.name, COALESCE(SUM(e.file_size_bytes), 0) AS total \
             FROM folders f \
             LEFT JOIN embroidery_files e ON e.folder_id = f.id AND e.deleted_at IS NULL \
             GROUP BY f.id ORDER BY total DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(FolderStat {
                    folder_name: row.get(0)?,
                    value: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        rows
    };

    // Recent imports (last 7 days)
    let recent_imports: i64 = conn.query_row(
        "SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL AND created_at >= datetime('now', '-7 days')",
        [],
        |row| row.get(0),
    )?;

    Ok(DashboardStats {
        files_by_type,
        ai_status: AiStatusCounts {
            none: ai_none,
            analyzed: ai_analyzed,
            confirmed: ai_confirmed,
        },
        top_folders,
        missing_metadata: MissingMetadata {
            no_tags,
            no_rating,
            no_description,
        },
        storage_by_folder,
        recent_imports,
    })
}

#[cfg(test)]
mod tests {
    use crate::db::migrations::init_database_in_memory;

    #[test]
    fn test_dashboard_stats_empty_db() {
        let conn = init_database_in_memory().unwrap();

        // AI status counts on empty DB
        let ai_none: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL AND ai_analyzed = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ai_none, 0);

        let recent: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM embroidery_files WHERE deleted_at IS NULL AND created_at >= datetime('now', '-7 days')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(recent, 0);
    }
}
