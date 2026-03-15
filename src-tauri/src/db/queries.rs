use crate::db::models::EmbroideryFile;

pub const FILE_SELECT: &str =
    "SELECT id, folder_id, filename, filepath, name, theme, description, license, \
     width_mm, height_mm, stitch_count, color_count, file_size_bytes, thumbnail_path, \
     design_name, jump_count, trim_count, hoop_width_mm, hoop_height_mm, \
     category, author, keywords, comments, unique_id, is_favorite, \
     file_type, size_range, skill_level, language, format_type, file_source, purchase_link, status, \
     ai_analyzed, ai_confirmed, created_at, updated_at FROM embroidery_files";

/// Same column list with `e.` alias prefix, for use in JOINs or subquery-filtered queries.
pub const FILE_SELECT_ALIASED: &str =
    "SELECT e.id, e.folder_id, e.filename, e.filepath, e.name, e.theme, e.description, \
     e.license, e.width_mm, e.height_mm, e.stitch_count, e.color_count, \
     e.file_size_bytes, e.thumbnail_path, \
     e.design_name, e.jump_count, e.trim_count, e.hoop_width_mm, e.hoop_height_mm, \
     e.category, e.author, e.keywords, e.comments, e.unique_id, e.is_favorite, \
     e.file_type, e.size_range, e.skill_level, e.language, e.format_type, \
     e.file_source, e.purchase_link, e.status, \
     e.ai_analyzed, e.ai_confirmed, \
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
        design_name: row.get(14)?,
        jump_count: row.get(15)?,
        trim_count: row.get(16)?,
        hoop_width_mm: row.get(17)?,
        hoop_height_mm: row.get(18)?,
        category: row.get(19)?,
        author: row.get(20)?,
        keywords: row.get(21)?,
        comments: row.get(22)?,
        unique_id: row.get(23)?,
        is_favorite: row.get(24)?,
        file_type: row.get(25)?,
        size_range: row.get(26)?,
        skill_level: row.get(27)?,
        language: row.get(28)?,
        format_type: row.get(29)?,
        file_source: row.get(30)?,
        purchase_link: row.get(31)?,
        status: row.get(32)?,
        ai_analyzed: row.get(33)?,
        ai_confirmed: row.get(34)?,
        created_at: row.get(35)?,
        updated_at: row.get(36)?,
    })
}
