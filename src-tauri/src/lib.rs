use std::sync::Mutex;
use tauri::Manager;

mod commands;
mod db;
mod error;
mod parsers;
mod services;

/// Wrapper for the rusqlite connection stored in Tauri managed state.
/// Used by Rust-side commands for write operations, complex queries, and batch ops.
/// The frontend uses `tauri-plugin-sql` for lightweight read queries independently.
#[allow(dead_code)] // Will be accessed by commands in Sprint 2+
pub struct DbState(pub(crate) Mutex<rusqlite::Connection>);

pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("stitch_manager.db");
            let conn = db::init_database(&db_path)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        });

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_log::Builder::new().build());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
