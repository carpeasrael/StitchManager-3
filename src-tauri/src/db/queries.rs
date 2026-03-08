use crate::db::models::EmbroideryFile;

pub const FILE_SELECT: &str =
    "SELECT id, folder_id, filename, filepath, name, theme, description, license, \
     width_mm, height_mm, stitch_count, color_count, file_size_bytes, thumbnail_path, \
     ai_analyzed, ai_confirmed, created_at, updated_at FROM embroidery_files";

/// Same column list with `e.` alias prefix, for use in JOINs or subquery-filtered queries.
pub const FILE_SELECT_ALIASED: &str =
    "SELECT e.id, e.folder_id, e.filename, e.filepath, e.name, e.theme, e.description, \
     e.license, e.width_mm, e.height_mm, e.stitch_count, e.color_count, \
     e.file_size_bytes, e.thumbnail_path, e.ai_analyzed, e.ai_confirmed, \
     e.created_at, e.updated_at FROM embroidery_files e";

pub fn row_to_file(row: &rusqlite::Row) -> rusqlite::Result<EmbroideryFile> {
    Ok(EmbroideryFile {
        id: row.get(0)?,
        folder_id: row.get(1)?,
        filename: row.get(2)?,
        filepath: row.get(3)?,
        name: row.get(4)?,
        theme: row.get(5)?,
        description: row.get(6)?,
        license: row.get(7)?,
        width_mm: row.get(8)?,
        height_mm: row.get(9)?,
        stitch_count: row.get(10)?,
        color_count: row.get(11)?,
        file_size_bytes: row.get(12)?,
        thumbnail_path: row.get(13)?,
        ai_analyzed: row.get(14)?,
        ai_confirmed: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}
