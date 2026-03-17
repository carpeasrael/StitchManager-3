use std::path::Path;
use rusqlite::Connection;
use crate::error::AppError;

const CURRENT_VERSION: i32 = 21;

pub fn init_database(db_path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
    run_migrations(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn init_database_in_memory() -> Result<Connection, AppError> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn get_schema_version(conn: &Connection) -> Result<Option<i32>, AppError> {
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get(0),
    )?;

    if !table_exists {
        return Ok(None);
    }

    let version: Option<i32> = conn.query_row(
        "SELECT MAX(version) FROM schema_version",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}

fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    let current = get_schema_version(conn)?.unwrap_or(0);

    if current >= CURRENT_VERSION {
        return Ok(());
    }

    if current < 1 {
        apply_v1(conn)?;
    }

    if current < 2 {
        apply_v2(conn)?;
    }

    if current < 3 {
        apply_v3(conn)?;
    }

    if current < 4 {
        apply_v4(conn)?;
    }

    if current < 5 {
        apply_v5(conn)?;
    }

    if current < 6 {
        apply_v6(conn)?;
    }

    if current < 7 {
        apply_v7(conn)?;
    }

    if current < 8 {
        apply_v8(conn)?;
    }

    if current < 9 {
        apply_v9(conn)?;
    }

    if current < 10 {
        apply_v10(conn)?;
    }

    if current < 11 {
        apply_v11(conn)?;
    }

    if current < 12 {
        apply_v12(conn)?;
    }

    if current < 13 {
        apply_v13(conn)?;
    }

    if current < 14 {
        apply_v14(conn)?;
    }

    if current < 15 {
        apply_v15(conn)?;
    }

    if current < 16 {
        apply_v16(conn)?;
    }

    if current < 17 {
        apply_v17(conn)?;
    }

    if current < 18 {
        apply_v18(conn)?;
    }

    if current < 19 {
        apply_v19(conn)?;
    }

    if current < 20 {
        apply_v20(conn)?;
    }

    if current < 21 {
        apply_v21(conn)?;
    }

    // Keep query planner statistics up to date
    let _ = conn.execute_batch("ANALYZE;");

    Ok(())
}

fn apply_v1(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        -- Migration tracking
        CREATE TABLE IF NOT EXISTS schema_version (
            version     INTEGER PRIMARY KEY,
            applied_at  TEXT NOT NULL DEFAULT (datetime('now')),
            description TEXT
        );

        -- Folder entries
        CREATE TABLE IF NOT EXISTS folders (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            path        TEXT NOT NULL UNIQUE,
            parent_id   INTEGER REFERENCES folders(id) ON DELETE CASCADE,
            sort_order  INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_folders_parent_id ON folders(parent_id);
        -- idx_folders_path omitted: UNIQUE constraint on path already creates an implicit index

        -- Embroidery files
        CREATE TABLE IF NOT EXISTS embroidery_files (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            folder_id       INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
            filename        TEXT NOT NULL,
            filepath        TEXT NOT NULL UNIQUE,
            name            TEXT,
            theme           TEXT,
            description     TEXT,
            license         TEXT,
            width_mm        REAL,
            height_mm       REAL,
            stitch_count    INTEGER,
            color_count     INTEGER,
            file_size_bytes INTEGER,
            thumbnail_path  TEXT,
            ai_analyzed     INTEGER NOT NULL DEFAULT 0,
            ai_confirmed    INTEGER NOT NULL DEFAULT 0,
            created_at      TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_embroidery_files_folder_id ON embroidery_files(folder_id);
        CREATE INDEX IF NOT EXISTS idx_embroidery_files_name ON embroidery_files(name);
        -- idx_embroidery_files_filepath omitted: UNIQUE constraint on filepath already creates an implicit index
        CREATE INDEX IF NOT EXISTS idx_embroidery_files_ai_analyzed ON embroidery_files(ai_analyzed);

        -- File format variants
        CREATE TABLE IF NOT EXISTS file_formats (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id         INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            format          TEXT NOT NULL,
            format_version  TEXT,
            filepath        TEXT NOT NULL,
            file_size_bytes INTEGER,
            parsed          INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_file_formats_file_id ON file_formats(file_id);
        CREATE INDEX IF NOT EXISTS idx_file_formats_format ON file_formats(format);

        -- Thread colors
        CREATE TABLE IF NOT EXISTS file_thread_colors (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id     INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            sort_order  INTEGER NOT NULL DEFAULT 0,
            color_hex   TEXT NOT NULL,
            color_name  TEXT,
            brand       TEXT,
            brand_code  TEXT,
            is_ai       INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_file_thread_colors_file_id ON file_thread_colors(file_id);

        -- Tags
        CREATE TABLE IF NOT EXISTS tags (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL UNIQUE,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- File-Tag junction
        CREATE TABLE IF NOT EXISTS file_tags (
            file_id INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (file_id, tag_id)
        );
        CREATE INDEX IF NOT EXISTS idx_file_tags_file_id ON file_tags(file_id);
        CREATE INDEX IF NOT EXISTS idx_file_tags_tag_id ON file_tags(tag_id);

        -- AI analysis results
        CREATE TABLE IF NOT EXISTS ai_analysis_results (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id       INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            provider      TEXT NOT NULL,
            model         TEXT NOT NULL,
            prompt_hash   TEXT,
            raw_response  TEXT,
            parsed_name   TEXT,
            parsed_theme  TEXT,
            parsed_desc   TEXT,
            parsed_tags   TEXT,
            parsed_colors TEXT,
            accepted      INTEGER NOT NULL DEFAULT 0,
            analyzed_at   TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_ai_analysis_results_file_id ON ai_analysis_results(file_id);

        -- Settings (key-value)
        CREATE TABLE IF NOT EXISTS settings (
            key        TEXT PRIMARY KEY,
            value      TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Custom field definitions
        CREATE TABLE IF NOT EXISTS custom_field_definitions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL UNIQUE,
            field_type  TEXT NOT NULL DEFAULT 'text',
            options     TEXT,
            required    INTEGER NOT NULL DEFAULT 0,
            sort_order  INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Custom field values
        CREATE TABLE IF NOT EXISTS custom_field_values (
            file_id  INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            field_id INTEGER NOT NULL REFERENCES custom_field_definitions(id) ON DELETE CASCADE,
            value    TEXT,
            PRIMARY KEY (file_id, field_id)
        );
        CREATE INDEX IF NOT EXISTS idx_custom_field_values_file_id ON custom_field_values(file_id);

        -- Default settings (only essential rendering defaults)
        INSERT OR IGNORE INTO settings (key, value) VALUES ('theme_mode', 'hell');

        -- Record migration
        INSERT INTO schema_version (version, description) VALUES (1, 'Initial schema');

        COMMIT;"
    )?;

    Ok(())
}

fn apply_v2(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        ALTER TABLE embroidery_files ADD COLUMN design_name TEXT;
        ALTER TABLE embroidery_files ADD COLUMN jump_count INTEGER;
        ALTER TABLE embroidery_files ADD COLUMN trim_count INTEGER;
        ALTER TABLE embroidery_files ADD COLUMN hoop_width_mm REAL;
        ALTER TABLE embroidery_files ADD COLUMN hoop_height_mm REAL;

        INSERT INTO schema_version (version, description) VALUES (2, 'Add parser metadata fields');

        COMMIT;"
    )?;

    Ok(())
}

fn apply_v3(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        ALTER TABLE embroidery_files ADD COLUMN category TEXT;
        ALTER TABLE embroidery_files ADD COLUMN author TEXT;
        ALTER TABLE embroidery_files ADD COLUMN keywords TEXT;
        ALTER TABLE embroidery_files ADD COLUMN comments TEXT;

        INSERT INTO schema_version (version, description) VALUES (3, 'Add PES extended metadata fields');

        COMMIT;"
    )?;

    Ok(())
}

fn apply_v4(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        CREATE INDEX IF NOT EXISTS idx_file_thread_colors_hex ON file_thread_colors(color_hex);
        CREATE INDEX IF NOT EXISTS idx_ai_analysis_accepted ON ai_analysis_results(accepted);

        INSERT INTO schema_version (version, description) VALUES (4, 'Add indexes for color search and AI status queries');

        COMMIT;"
    )?;

    Ok(())
}

fn apply_v5(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        ALTER TABLE embroidery_files ADD COLUMN unique_id TEXT;
        CREATE UNIQUE INDEX IF NOT EXISTS idx_embroidery_files_unique_id ON embroidery_files(unique_id);

        CREATE TABLE IF NOT EXISTS file_attachments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id INTEGER NOT NULL,
            filename TEXT NOT NULL,
            mime_type TEXT,
            file_path TEXT NOT NULL,
            attachment_type TEXT DEFAULT 'license',
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (file_id) REFERENCES embroidery_files(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_file_attachments_file_id ON file_attachments(file_id);

        INSERT INTO schema_version (version, description) VALUES (5, 'Add unique_id column and file_attachments table');

        COMMIT;"
    )?;

    // Backfill unique IDs for existing records
    backfill_unique_ids(conn)?;

    Ok(())
}

fn apply_v8(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        CREATE TABLE IF NOT EXISTS file_versions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            version_number INTEGER NOT NULL,
            file_data BLOB NOT NULL,
            file_size INTEGER NOT NULL,
            operation TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_file_versions_file_id ON file_versions(file_id);

        CREATE TABLE IF NOT EXISTS machine_profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            machine_type TEXT NOT NULL DEFAULT 'generic',
            transfer_path TEXT NOT NULL,
            target_format TEXT,
            last_used TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        INSERT INTO schema_version (version, description)
        VALUES (8, 'Add file_versions and machine_profiles tables');

        COMMIT;"
    )?;

    Ok(())
}

fn apply_v7(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        ALTER TABLE embroidery_files ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;
        CREATE INDEX IF NOT EXISTS idx_files_favorite ON embroidery_files(is_favorite);
        CREATE INDEX IF NOT EXISTS idx_files_updated_at ON embroidery_files(updated_at);

        INSERT INTO schema_version (version, description)
        VALUES (7, 'Add is_favorite column and dashboard indexes');

        COMMIT;"
    )?;

    Ok(())
}

fn apply_v6(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        -- Composite indexes for common query patterns
        CREATE INDEX IF NOT EXISTS idx_files_folder_filename ON embroidery_files(folder_id, filename);
        CREATE INDEX IF NOT EXISTS idx_files_search_name ON embroidery_files(name, filename);

        -- FTS5 virtual table for full-text search
        CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
            name, filename, theme, description, design_name,
            category, author, keywords, comments, license, unique_id,
            content=embroidery_files, content_rowid=id
        );

        -- Populate FTS index from existing data
        INSERT OR IGNORE INTO files_fts(rowid, name, filename, theme, description, design_name,
            category, author, keywords, comments, license, unique_id)
        SELECT id, COALESCE(name,''), filename, COALESCE(theme,''), COALESCE(description,''),
            COALESCE(design_name,''), COALESCE(category,''), COALESCE(author,''),
            COALESCE(keywords,''), COALESCE(comments,''), COALESCE(license,''),
            COALESCE(unique_id,'')
        FROM embroidery_files;

        -- Triggers to keep FTS index in sync
        CREATE TRIGGER IF NOT EXISTS files_fts_insert AFTER INSERT ON embroidery_files BEGIN
            INSERT INTO files_fts(rowid, name, filename, theme, description, design_name,
                category, author, keywords, comments, license, unique_id)
            VALUES (new.id, COALESCE(new.name,''), new.filename, COALESCE(new.theme,''),
                COALESCE(new.description,''), COALESCE(new.design_name,''),
                COALESCE(new.category,''), COALESCE(new.author,''),
                COALESCE(new.keywords,''), COALESCE(new.comments,''),
                COALESCE(new.license,''), COALESCE(new.unique_id,''));
        END;

        CREATE TRIGGER IF NOT EXISTS files_fts_delete AFTER DELETE ON embroidery_files BEGIN
            INSERT INTO files_fts(files_fts, rowid, name, filename, theme, description,
                design_name, category, author, keywords, comments, license, unique_id)
            VALUES ('delete', old.id, COALESCE(old.name,''), old.filename,
                COALESCE(old.theme,''), COALESCE(old.description,''),
                COALESCE(old.design_name,''), COALESCE(old.category,''),
                COALESCE(old.author,''), COALESCE(old.keywords,''),
                COALESCE(old.comments,''), COALESCE(old.license,''),
                COALESCE(old.unique_id,''));
        END;

        CREATE TRIGGER IF NOT EXISTS files_fts_update AFTER UPDATE ON embroidery_files BEGIN
            INSERT INTO files_fts(files_fts, rowid, name, filename, theme, description,
                design_name, category, author, keywords, comments, license, unique_id)
            VALUES ('delete', old.id, COALESCE(old.name,''), old.filename,
                COALESCE(old.theme,''), COALESCE(old.description,''),
                COALESCE(old.design_name,''), COALESCE(old.category,''),
                COALESCE(old.author,''), COALESCE(old.keywords,''),
                COALESCE(old.comments,''), COALESCE(old.license,''),
                COALESCE(old.unique_id,''));
            INSERT INTO files_fts(rowid, name, filename, theme, description, design_name,
                category, author, keywords, comments, license, unique_id)
            VALUES (new.id, COALESCE(new.name,''), new.filename, COALESCE(new.theme,''),
                COALESCE(new.description,''), COALESCE(new.design_name,''),
                COALESCE(new.category,''), COALESCE(new.author,''),
                COALESCE(new.keywords,''), COALESCE(new.comments,''),
                COALESCE(new.license,''), COALESCE(new.unique_id,''));
        END;

        INSERT INTO schema_version (version, description)
        VALUES (6, 'Add composite indexes, FTS5 full-text search');

        COMMIT;"
    )?;

    Ok(())
}

/// Generate unique IDs for all existing records that don't have one.
fn backfill_unique_ids(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare("SELECT id FROM embroidery_files WHERE unique_id IS NULL")?;
    let ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    for id in ids {
        let uid = generate_unique_id();
        conn.execute(
            "UPDATE embroidery_files SET unique_id = ?1 WHERE id = ?2",
            rusqlite::params![uid, id],
        )?;
    }

    Ok(())
}

/// Generate a unique ID in the format SM-XXXXXXXX (8 alphanumeric chars).
pub fn generate_unique_id() -> String {
    let id = uuid::Uuid::new_v4();
    let bytes = id.as_bytes();
    // Take first 5 bytes → 8 base32 characters (without padding)
    let encoded = base32_encode(&bytes[..5]);
    format!("SM-{encoded}")
}

/// Simple base32 encoding (RFC 4648 without padding) for 5 bytes → 8 chars.
fn base32_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::with_capacity(8);
    let mut buffer: u64 = 0;
    for &b in data {
        buffer = (buffer << 8) | b as u64;
    }
    // 5 bytes = 40 bits → 8 × 5-bit groups
    for i in (0..8).rev() {
        let idx = ((buffer >> (i * 5)) & 0x1F) as usize;
        result.push(ALPHABET[idx] as char);
    }
    result
}

fn apply_v9(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        -- S1-01: file_type discriminator
        ALTER TABLE embroidery_files ADD COLUMN file_type TEXT NOT NULL DEFAULT 'embroidery';
        CREATE INDEX IF NOT EXISTS idx_files_file_type ON embroidery_files(file_type);

        -- S1-02: sewing pattern metadata fields
        ALTER TABLE embroidery_files ADD COLUMN size_range TEXT;
        ALTER TABLE embroidery_files ADD COLUMN skill_level TEXT;
        ALTER TABLE embroidery_files ADD COLUMN language TEXT;
        ALTER TABLE embroidery_files ADD COLUMN format_type TEXT;
        ALTER TABLE embroidery_files ADD COLUMN file_source TEXT;
        ALTER TABLE embroidery_files ADD COLUMN purchase_link TEXT;

        -- S1-03: status tracking
        ALTER TABLE embroidery_files ADD COLUMN status TEXT NOT NULL DEFAULT 'none';
        CREATE INDEX IF NOT EXISTS idx_files_status ON embroidery_files(status);

        -- S1-05: Rebuild FTS5 with new searchable columns
        DROP TRIGGER IF EXISTS files_fts_insert;
        DROP TRIGGER IF EXISTS files_fts_delete;
        DROP TRIGGER IF EXISTS files_fts_update;
        DROP TABLE IF EXISTS files_fts;

        CREATE VIRTUAL TABLE files_fts USING fts5(
            name, filename, theme, description, design_name,
            category, author, keywords, comments, license, unique_id,
            language, file_source, size_range,
            content=embroidery_files, content_rowid=id
        );

        -- Repopulate FTS index
        INSERT INTO files_fts(rowid, name, filename, theme, description, design_name,
            category, author, keywords, comments, license, unique_id,
            language, file_source, size_range)
        SELECT id, COALESCE(name,''), filename, COALESCE(theme,''), COALESCE(description,''),
            COALESCE(design_name,''), COALESCE(category,''), COALESCE(author,''),
            COALESCE(keywords,''), COALESCE(comments,''), COALESCE(license,''),
            COALESCE(unique_id,''),
            COALESCE(language,''), COALESCE(file_source,''), COALESCE(size_range,'')
        FROM embroidery_files;

        -- Recreate triggers with all 14 indexed columns
        CREATE TRIGGER files_fts_insert AFTER INSERT ON embroidery_files BEGIN
            INSERT INTO files_fts(rowid, name, filename, theme, description, design_name,
                category, author, keywords, comments, license, unique_id,
                language, file_source, size_range)
            VALUES (new.id, COALESCE(new.name,''), new.filename, COALESCE(new.theme,''),
                COALESCE(new.description,''), COALESCE(new.design_name,''),
                COALESCE(new.category,''), COALESCE(new.author,''),
                COALESCE(new.keywords,''), COALESCE(new.comments,''),
                COALESCE(new.license,''), COALESCE(new.unique_id,''),
                COALESCE(new.language,''), COALESCE(new.file_source,''),
                COALESCE(new.size_range,''));
        END;

        CREATE TRIGGER files_fts_delete AFTER DELETE ON embroidery_files BEGIN
            INSERT INTO files_fts(files_fts, rowid, name, filename, theme, description,
                design_name, category, author, keywords, comments, license, unique_id,
                language, file_source, size_range)
            VALUES ('delete', old.id, COALESCE(old.name,''), old.filename,
                COALESCE(old.theme,''), COALESCE(old.description,''),
                COALESCE(old.design_name,''), COALESCE(old.category,''),
                COALESCE(old.author,''), COALESCE(old.keywords,''),
                COALESCE(old.comments,''), COALESCE(old.license,''),
                COALESCE(old.unique_id,''),
                COALESCE(old.language,''), COALESCE(old.file_source,''),
                COALESCE(old.size_range,''));
        END;

        CREATE TRIGGER files_fts_update AFTER UPDATE ON embroidery_files BEGIN
            INSERT INTO files_fts(files_fts, rowid, name, filename, theme, description,
                design_name, category, author, keywords, comments, license, unique_id,
                language, file_source, size_range)
            VALUES ('delete', old.id, COALESCE(old.name,''), old.filename,
                COALESCE(old.theme,''), COALESCE(old.description,''),
                COALESCE(old.design_name,''), COALESCE(old.category,''),
                COALESCE(old.author,''), COALESCE(old.keywords,''),
                COALESCE(old.comments,''), COALESCE(old.license,''),
                COALESCE(old.unique_id,''),
                COALESCE(old.language,''), COALESCE(old.file_source,''),
                COALESCE(old.size_range,''));
            INSERT INTO files_fts(rowid, name, filename, theme, description, design_name,
                category, author, keywords, comments, license, unique_id,
                language, file_source, size_range)
            VALUES (new.id, COALESCE(new.name,''), new.filename, COALESCE(new.theme,''),
                COALESCE(new.description,''), COALESCE(new.design_name,''),
                COALESCE(new.category,''), COALESCE(new.author,''),
                COALESCE(new.keywords,''), COALESCE(new.comments,''),
                COALESCE(new.license,''), COALESCE(new.unique_id,''),
                COALESCE(new.language,''), COALESCE(new.file_source,''),
                COALESCE(new.size_range,''));
        END;

        INSERT INTO schema_version (version, description)
        VALUES (9, 'Add file_type, sewing pattern metadata, status tracking, rebuild FTS5');

        COMMIT;"
    )?;

    Ok(())
}

fn apply_v10(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        -- S2-01: PDF metadata columns
        ALTER TABLE embroidery_files ADD COLUMN page_count INTEGER;
        ALTER TABLE embroidery_files ADD COLUMN paper_size TEXT;

        -- S2-03: Enhanced file attachments
        ALTER TABLE file_attachments ADD COLUMN display_name TEXT;
        ALTER TABLE file_attachments ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

        INSERT INTO schema_version (version, description)
        VALUES (10, 'Add page_count, paper_size columns and enhanced file_attachments');

        COMMIT;"
    )?;

    Ok(())
}

fn apply_v11(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        CREATE TABLE IF NOT EXISTS instruction_bookmarks (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id     INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            page_number INTEGER NOT NULL,
            label       TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(file_id, page_number)
        );
        CREATE INDEX IF NOT EXISTS idx_bookmarks_file_id ON instruction_bookmarks(file_id);

        CREATE TABLE IF NOT EXISTS instruction_notes (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id     INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            page_number INTEGER NOT NULL,
            note_text   TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_notes_file_id ON instruction_notes(file_id);
        CREATE INDEX IF NOT EXISTS idx_notes_file_page ON instruction_notes(file_id, page_number);

        INSERT INTO schema_version (version, description)
        VALUES (11, 'Add instruction_bookmarks and instruction_notes tables');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v13(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        ALTER TABLE embroidery_files ADD COLUMN deleted_at TEXT;
        CREATE INDEX IF NOT EXISTS idx_files_deleted_at ON embroidery_files(deleted_at);

        ALTER TABLE projects ADD COLUMN deleted_at TEXT;
        CREATE INDEX IF NOT EXISTS idx_projects_deleted_at ON projects(deleted_at);

        INSERT INTO schema_version (version, description)
        VALUES (13, 'Add deleted_at column for soft delete / recycle bin');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v12(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        CREATE TABLE IF NOT EXISTS projects (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            pattern_file_id INTEGER REFERENCES embroidery_files(id) ON DELETE SET NULL,
            status          TEXT NOT NULL DEFAULT 'not_started',
            notes           TEXT,
            created_at      TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_projects_pattern_file_id ON projects(pattern_file_id);
        CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);

        CREATE TABLE IF NOT EXISTS project_details (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            key        TEXT NOT NULL,
            value      TEXT,
            UNIQUE(project_id, key)
        );
        CREATE INDEX IF NOT EXISTS idx_project_details_project_id ON project_details(project_id);

        CREATE TABLE IF NOT EXISTS collections (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            description TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS collection_items (
            collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
            file_id       INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            PRIMARY KEY (collection_id, file_id)
        );
        CREATE INDEX IF NOT EXISTS idx_collection_items_file_id ON collection_items(file_id);

        INSERT INTO schema_version (version, description)
        VALUES (12, 'Add projects, project_details, collections, collection_items tables');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v14(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        -- Extend projects table with manufacturing fields
        ALTER TABLE projects ADD COLUMN order_number TEXT;
        ALTER TABLE projects ADD COLUMN customer TEXT;
        ALTER TABLE projects ADD COLUMN priority TEXT DEFAULT 'normal';
        ALTER TABLE projects ADD COLUMN deadline TEXT;
        ALTER TABLE projects ADD COLUMN responsible_person TEXT;
        ALTER TABLE projects ADD COLUMN approval_status TEXT DEFAULT 'draft';

        CREATE INDEX IF NOT EXISTS idx_projects_priority ON projects(priority);
        CREATE INDEX IF NOT EXISTS idx_projects_deadline ON projects(deadline);
        CREATE INDEX IF NOT EXISTS idx_projects_approval_status ON projects(approval_status);

        -- Suppliers
        CREATE TABLE IF NOT EXISTS suppliers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            contact TEXT,
            website TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_suppliers_deleted_at ON suppliers(deleted_at);

        -- Materials
        CREATE TABLE IF NOT EXISTS materials (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            material_number TEXT UNIQUE,
            name TEXT NOT NULL,
            material_type TEXT,
            unit TEXT DEFAULT 'Stk',
            supplier_id INTEGER REFERENCES suppliers(id) ON DELETE SET NULL,
            net_price REAL,
            waste_factor REAL DEFAULT 0.0,
            min_stock REAL DEFAULT 0,
            reorder_time_days INTEGER,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_materials_supplier_id ON materials(supplier_id);
        CREATE INDEX IF NOT EXISTS idx_materials_material_type ON materials(material_type);
        CREATE INDEX IF NOT EXISTS idx_materials_deleted_at ON materials(deleted_at);

        -- Material inventory
        CREATE TABLE IF NOT EXISTS material_inventory (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            material_id INTEGER NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
            total_stock REAL DEFAULT 0,
            reserved_stock REAL DEFAULT 0,
            location TEXT,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_material_inventory_material_id ON material_inventory(material_id);

        -- Products
        CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_number TEXT UNIQUE,
            name TEXT NOT NULL,
            category TEXT,
            description TEXT,
            product_type TEXT,
            status TEXT DEFAULT 'active',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_products_product_type ON products(product_type);
        CREATE INDEX IF NOT EXISTS idx_products_status ON products(status);
        CREATE INDEX IF NOT EXISTS idx_products_deleted_at ON products(deleted_at);

        -- Bill of materials
        CREATE TABLE IF NOT EXISTS bill_of_materials (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
            material_id INTEGER NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
            quantity REAL NOT NULL,
            unit TEXT,
            notes TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_bom_product_id ON bill_of_materials(product_id);
        CREATE INDEX IF NOT EXISTS idx_bom_material_id ON bill_of_materials(material_id);

        -- Time entries
        CREATE TABLE IF NOT EXISTS time_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            step_name TEXT NOT NULL,
            planned_minutes REAL,
            actual_minutes REAL,
            worker TEXT,
            machine TEXT,
            recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_time_entries_project_id ON time_entries(project_id);

        INSERT INTO schema_version (version, description)
        VALUES (14, 'Add manufacturing tables: suppliers, materials, inventory, products, BOM, time_entries; extend projects');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v15(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        -- Sprint D: Production Workflow
        CREATE TABLE IF NOT EXISTS step_definitions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            default_duration_minutes REAL,
            sort_order INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS product_steps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
            step_definition_id INTEGER NOT NULL REFERENCES step_definitions(id) ON DELETE CASCADE,
            sort_order INTEGER DEFAULT 0,
            UNIQUE(product_id, step_definition_id)
        );
        CREATE INDEX IF NOT EXISTS idx_product_steps_product_id ON product_steps(product_id);

        CREATE TABLE IF NOT EXISTS workflow_steps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            step_definition_id INTEGER NOT NULL REFERENCES step_definitions(id),
            status TEXT NOT NULL DEFAULT 'pending',
            responsible TEXT,
            started_at TEXT,
            completed_at TEXT,
            notes TEXT,
            sort_order INTEGER DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_workflow_steps_project_id ON workflow_steps(project_id);
        CREATE INDEX IF NOT EXISTS idx_workflow_steps_status ON workflow_steps(status);

        -- Sprint E: Procurement
        CREATE TABLE IF NOT EXISTS purchase_orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            order_number TEXT UNIQUE,
            supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
            status TEXT NOT NULL DEFAULT 'draft',
            order_date TEXT,
            expected_delivery TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_purchase_orders_supplier_id ON purchase_orders(supplier_id);
        CREATE INDEX IF NOT EXISTS idx_purchase_orders_status ON purchase_orders(status);
        CREATE INDEX IF NOT EXISTS idx_purchase_orders_deleted_at ON purchase_orders(deleted_at);

        CREATE TABLE IF NOT EXISTS order_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            order_id INTEGER NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
            material_id INTEGER NOT NULL REFERENCES materials(id),
            quantity_ordered REAL NOT NULL,
            quantity_delivered REAL DEFAULT 0,
            unit_price REAL,
            notes TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_order_items_order_id ON order_items(order_id);
        CREATE INDEX IF NOT EXISTS idx_order_items_material_id ON order_items(material_id);

        CREATE TABLE IF NOT EXISTS deliveries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            order_id INTEGER NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
            delivery_date TEXT NOT NULL DEFAULT (datetime('now')),
            delivery_note TEXT,
            notes TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_deliveries_order_id ON deliveries(order_id);

        CREATE TABLE IF NOT EXISTS delivery_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            delivery_id INTEGER NOT NULL REFERENCES deliveries(id) ON DELETE CASCADE,
            order_item_id INTEGER NOT NULL REFERENCES order_items(id),
            quantity_received REAL NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_delivery_items_delivery_id ON delivery_items(delivery_id);

        -- Sprint F: License Management
        CREATE TABLE IF NOT EXISTS license_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            license_type TEXT DEFAULT 'personal',
            valid_from TEXT,
            valid_until TEXT,
            max_uses INTEGER,
            current_uses INTEGER DEFAULT 0,
            commercial_allowed INTEGER DEFAULT 0,
            source TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_license_records_deleted_at ON license_records(deleted_at);

        CREATE TABLE IF NOT EXISTS license_file_links (
            license_id INTEGER NOT NULL REFERENCES license_records(id) ON DELETE CASCADE,
            file_id INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
            PRIMARY KEY (license_id, file_id)
        );
        CREATE INDEX IF NOT EXISTS idx_license_file_links_file_id ON license_file_links(file_id);

        INSERT INTO schema_version (version, description)
        VALUES (15, 'Add workflow, procurement, and license management tables');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v16(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        CREATE TABLE IF NOT EXISTS quality_inspections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            workflow_step_id INTEGER REFERENCES workflow_steps(id) ON DELETE SET NULL,
            inspector TEXT,
            inspection_date TEXT NOT NULL DEFAULT (datetime('now')),
            result TEXT NOT NULL DEFAULT 'pending',
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_quality_inspections_project_id ON quality_inspections(project_id);
        CREATE INDEX IF NOT EXISTS idx_quality_inspections_result ON quality_inspections(result);

        CREATE TABLE IF NOT EXISTS defect_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            inspection_id INTEGER NOT NULL REFERENCES quality_inspections(id) ON DELETE CASCADE,
            description TEXT NOT NULL,
            severity TEXT DEFAULT 'minor',
            status TEXT DEFAULT 'open',
            resolved_at TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_defect_records_inspection_id ON defect_records(inspection_id);
        CREATE INDEX IF NOT EXISTS idx_defect_records_status ON defect_records(status);

        INSERT INTO schema_version (version, description)
        VALUES (16, 'Add quality inspections and defect tracking tables');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v17(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        -- Cost rates: configurable labor, machine, overhead, profit rates
        CREATE TABLE IF NOT EXISTS cost_rates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rate_type TEXT NOT NULL,
            name TEXT NOT NULL,
            rate_value REAL NOT NULL,
            unit TEXT,
            setup_cost REAL DEFAULT 0,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_cost_rates_rate_type ON cost_rates(rate_type);
        CREATE INDEX IF NOT EXISTS idx_cost_rates_deleted_at ON cost_rates(deleted_at);

        -- Project cost items: persisted cost breakdown snapshots
        CREATE TABLE IF NOT EXISTS project_cost_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            cost_type TEXT NOT NULL,
            description TEXT,
            amount REAL NOT NULL,
            calculated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_project_cost_items_project_id ON project_cost_items(project_id);

        -- Add cost fields to license_records
        ALTER TABLE license_records ADD COLUMN cost_per_piece REAL DEFAULT 0;
        ALTER TABLE license_records ADD COLUMN cost_per_series REAL DEFAULT 0;
        ALTER TABLE license_records ADD COLUMN cost_flat REAL DEFAULT 0;

        -- Link time entries to cost rates for per-resource costing
        ALTER TABLE time_entries ADD COLUMN cost_rate_id INTEGER REFERENCES cost_rates(id) ON DELETE SET NULL;

        -- Add shipping cost to purchase orders for procurement cost aggregation
        ALTER TABLE purchase_orders ADD COLUMN shipping_cost REAL DEFAULT 0;

        -- Add production quantity to projects for per-piece calculations
        ALTER TABLE projects ADD COLUMN quantity INTEGER DEFAULT 1;

        -- Link licenses to projects
        CREATE TABLE IF NOT EXISTS project_license_links (
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            license_id INTEGER NOT NULL REFERENCES license_records(id) ON DELETE CASCADE,
            PRIMARY KEY (project_id, license_id)
        );
        CREATE INDEX IF NOT EXISTS idx_project_license_links_license_id ON project_license_links(license_id);

        INSERT INTO schema_version (version, description)
        VALUES (17, 'Add cost calculation: cost_rates, project_cost_items, license costs, project quantity');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v18(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        -- Material consumptions: actual usage per project/material/step
        CREATE TABLE IF NOT EXISTS material_consumptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            material_id INTEGER NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
            quantity REAL NOT NULL,
            unit TEXT,
            step_name TEXT,
            recorded_by TEXT,
            notes TEXT,
            recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_material_consumptions_project_id ON material_consumptions(project_id);
        CREATE INDEX IF NOT EXISTS idx_material_consumptions_material_id ON material_consumptions(material_id);

        -- Inventory transactions: audit log for all automated stock changes
        CREATE TABLE IF NOT EXISTS inventory_transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            material_id INTEGER NOT NULL REFERENCES materials(id) ON DELETE CASCADE,
            project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            transaction_type TEXT NOT NULL,
            quantity REAL NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_inventory_transactions_material_id ON inventory_transactions(material_id);
        CREATE INDEX IF NOT EXISTS idx_inventory_transactions_project_id ON inventory_transactions(project_id);
        CREATE INDEX IF NOT EXISTS idx_inventory_transactions_type ON inventory_transactions(transaction_type);

        INSERT INTO schema_version (version, description)
        VALUES (18, 'Add material consumption tracking and inventory transaction audit log');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v19(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        -- Add project linkage to purchase orders
        ALTER TABLE purchase_orders ADD COLUMN project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL;
        CREATE INDEX IF NOT EXISTS idx_purchase_orders_project_id ON purchase_orders(project_id);

        INSERT INTO schema_version (version, description)
        VALUES (19, 'Add project_id to purchase_orders for project-order linkage');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v20(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        CREATE TABLE IF NOT EXISTS product_variants (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
            sku TEXT,
            variant_name TEXT,
            size TEXT,
            color TEXT,
            additional_cost REAL DEFAULT 0,
            notes TEXT,
            status TEXT DEFAULT 'active',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_product_variants_product_id ON product_variants(product_id);
        CREATE INDEX IF NOT EXISTS idx_product_variants_deleted_at ON product_variants(deleted_at);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_product_variants_sku_active ON product_variants(sku) WHERE deleted_at IS NULL;

        INSERT INTO schema_version (version, description)
        VALUES (20, 'Add product_variants table for sizes, colors, and customization');

        COMMIT;"
    )?;
    Ok(())
}

fn apply_v21(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "BEGIN TRANSACTION;

        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_type TEXT NOT NULL,
            entity_id INTEGER NOT NULL,
            field_name TEXT NOT NULL,
            old_value TEXT,
            new_value TEXT,
            changed_by TEXT,
            changed_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_audit_log_entity ON audit_log(entity_type, entity_id);
        CREATE INDEX IF NOT EXISTS idx_audit_log_changed_at ON audit_log(changed_at);

        INSERT INTO schema_version (version, description)
        VALUES (21, 'Add audit_log table for change history traceability');

        COMMIT;"
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_database_creates_tables() {
        let conn = init_database_in_memory().unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        let expected = vec![
            "ai_analysis_results",
            "audit_log",
            "bill_of_materials",
            "collection_items",
            "collections",
            "cost_rates",
            "custom_field_definitions",
            "custom_field_values",
            "defect_records",
            "deliveries",
            "delivery_items",
            "embroidery_files",
            "file_attachments",
            "file_formats",
            "file_tags",
            "file_thread_colors",
            "file_versions",
            "files_fts",
            "files_fts_config",
            "files_fts_data",
            "files_fts_docsize",
            "files_fts_idx",
            "folders",
            "instruction_bookmarks",
            "instruction_notes",
            "inventory_transactions",
            "license_file_links",
            "license_records",
            "machine_profiles",
            "material_consumptions",
            "material_inventory",
            "materials",
            "order_items",
            "product_steps",
            "product_variants",
            "products",
            "project_cost_items",
            "project_details",
            "project_license_links",
            "projects",
            "purchase_orders",
            "quality_inspections",
            "schema_version",
            "settings",
            "step_definitions",
            "suppliers",
            "tags",
            "time_entries",
            "workflow_steps",
        ];

        assert_eq!(tables, expected, "All tables must exist (including FTS5)");
    }

    #[test]
    fn test_init_database_is_idempotent() {
        let conn = init_database_in_memory().unwrap();

        // Run migrations again — should be a no-op
        let result = run_migrations(&conn);
        assert!(result.is_ok(), "Second migration run must succeed");

        let version: i32 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 21, "Schema version must be 17");
    }

    #[test]
    fn test_default_settings_inserted() {
        let conn = init_database_in_memory().unwrap();

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "Only essential default settings must exist");

        let theme: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme_mode'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(theme, "hell");
    }

    #[test]
    fn test_schema_version_is_twentyone() {
        let conn = init_database_in_memory().unwrap();

        let version: i32 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 21);

        let desc: String = conn
            .query_row(
                "SELECT description FROM schema_version WHERE version = 21",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(desc.contains("audit"), "v21 description should mention audit");
    }

    #[test]
    fn test_cascade_delete_folder_removes_files() {
        let conn = init_database_in_memory().unwrap();

        conn.execute(
            "INSERT INTO folders (name, path) VALUES ('Test', '/tmp/test')",
            [],
        )
        .unwrap();
        let folder_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO embroidery_files (folder_id, filename, filepath) VALUES (?1, 'a.pes', '/tmp/test/a.pes')",
            [folder_id],
        )
        .unwrap();

        let file_count: i32 = conn
            .query_row("SELECT COUNT(*) FROM embroidery_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(file_count, 1);

        conn.execute("DELETE FROM folders WHERE id = ?1", [folder_id])
            .unwrap();

        let file_count: i32 = conn
            .query_row("SELECT COUNT(*) FROM embroidery_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(file_count, 0, "Cascade delete must remove child files");
    }
}
