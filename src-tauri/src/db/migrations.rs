use std::path::Path;
use rusqlite::Connection;
use crate::error::AppError;

const CURRENT_VERSION: i32 = 3;

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

        -- Default settings
        INSERT OR IGNORE INTO settings (key, value) VALUES ('library_root', '~/Stickdateien');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('metadata_root', '~/Stickdateien/.stichman');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('theme_mode', 'hell');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('ai_provider', 'ollama');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('ai_url', 'http://localhost:11434');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('ai_model', 'llama3.2-vision');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('ai_temperature', '0.3');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('ai_timeout_ms', '30000');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('rename_pattern', '{name}_{theme}');
        INSERT OR IGNORE INTO settings (key, value) VALUES ('organize_pattern', '{theme}/{name}');

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
            "custom_field_definitions",
            "custom_field_values",
            "embroidery_files",
            "file_formats",
            "file_tags",
            "file_thread_colors",
            "folders",
            "schema_version",
            "settings",
            "tags",
        ];

        assert_eq!(tables, expected, "All 11 tables must exist");
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
        assert_eq!(version, 3, "Schema version must be 3");
    }

    #[test]
    fn test_default_settings_inserted() {
        let conn = init_database_in_memory().unwrap();

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 10, "All 10 default settings must exist");

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
    fn test_schema_version_is_three() {
        let conn = init_database_in_memory().unwrap();

        let version: i32 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);

        let desc: String = conn
            .query_row(
                "SELECT description FROM schema_version WHERE version = 3",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(desc, "Add PES extended metadata fields");
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
