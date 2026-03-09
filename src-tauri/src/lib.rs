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
pub struct DbState(pub(crate) Mutex<rusqlite::Connection>);

pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:stitch_manager.db", vec![])
                .build(),
        )
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
        .invoke_handler(tauri::generate_handler![
            commands::folders::get_folders,
            commands::folders::create_folder,
            commands::folders::update_folder,
            commands::folders::delete_folder,
            commands::folders::get_folder_file_count,
            commands::scanner::scan_directory,
            commands::scanner::import_files,
            commands::files::get_files,
            commands::files::get_file,
            commands::files::get_file_formats,
            commands::files::get_file_colors,
            commands::files::get_file_tags,
            commands::files::update_file,
            commands::files::delete_file,
            commands::files::set_file_tags,
            commands::files::get_all_tags,
            commands::files::get_thumbnail,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            commands::settings::get_custom_fields,
            commands::settings::create_custom_field,
            commands::settings::delete_custom_field,
            commands::scanner::parse_embroidery_file,
            commands::batch::batch_rename,
            commands::batch::batch_organize,
            commands::batch::batch_export_usb,
            commands::ai::ai_build_prompt,
            commands::ai::ai_analyze_file,
            commands::ai::ai_accept_result,
            commands::ai::ai_reject_result,
            commands::ai::ai_test_connection,
            commands::ai::ai_analyze_batch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
