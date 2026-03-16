use std::sync::Mutex;
use tauri::{Emitter, Manager};

mod commands;
mod db;
mod error;
mod parsers;
mod services;

/// Wrapper for the rusqlite connection stored in Tauri managed state.
/// Used by Rust-side commands for write operations, complex queries, and batch ops.
/// The frontend uses `tauri-plugin-sql` for lightweight read queries independently.
pub struct DbState(pub(crate) Mutex<rusqlite::Connection>);

/// Wrapper for the ThumbnailGenerator stored in Tauri managed state.
pub struct ThumbnailState(pub(crate) services::thumbnail::ThumbnailGenerator);

pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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

            // Try to auto-start watcher if library_root is set
            let library_root: Option<String> = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'library_root'",
                    [],
                    |row| row.get(0),
                )
                .ok();

            app.manage(DbState(Mutex::new(conn)));

            // Initialize thumbnail generator with cache dir
            let thumb_cache_dir = app_data_dir.join("thumbnails");
            let thumbnail_gen = services::thumbnail::ThumbnailGenerator::new(thumb_cache_dir);
            app.manage(ThumbnailState(thumbnail_gen));

            // Initialize watcher holder
            let watcher_holder =
                services::file_watcher::WatcherHolder(Mutex::new(None));

            if let Some(root) = library_root {
                // Expand ~ to home directory
                let expanded = if root.starts_with("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&root[2..]).to_string_lossy().to_string()
                    } else {
                        root.clone()
                    }
                } else {
                    root.clone()
                };

                if std::path::Path::new(&expanded).is_dir() {
                    match services::file_watcher::start_watcher(
                        &expanded,
                        app.handle(),
                    ) {
                        Ok(state) => {
                            match watcher_holder.0.lock() {
                                Ok(mut guard) => { *guard = Some(state); }
                                Err(e) => {
                                    log::error!("Watcher mutex poisoned: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to start file watcher: {e}");
                            let _ = app.handle().emit("watcher:status", serde_json::json!({
                                "active": false,
                                "error": format!("{e}")
                            }));
                        }
                    }
                }
            }

            app.manage(watcher_holder);

            // Initialize USB monitor
            let usb_holder = services::usb_monitor::UsbMonitorHolder(Mutex::new(None));
            match services::usb_monitor::start_usb_monitor(app.handle()) {
                Ok(state) => {
                    match usb_holder.0.lock() {
                        Ok(mut guard) => { *guard = Some(state); }
                        Err(e) => {
                            log::error!("USB monitor mutex poisoned: {e}");
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to start USB monitor: {e}");
                }
            }
            app.manage(usb_holder);

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
            commands::folders::get_all_folder_file_counts,
            commands::scanner::scan_directory,
            commands::scanner::import_files,
            commands::files::get_files,
            commands::files::get_files_by_ids,
            commands::files::get_files_paginated,
            commands::files::get_thumbnails_batch,
            commands::files::get_recent_files,
            commands::files::get_favorite_files,
            commands::files::toggle_favorite,
            commands::files::get_library_stats,
            commands::files::get_file,
            commands::files::get_file_formats,
            commands::files::get_file_colors,
            commands::files::get_file_tags,
            commands::files::update_file,
            commands::files::update_file_status,
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
            commands::settings::get_custom_field_values,
            commands::settings::set_custom_field_values,
            commands::scanner::mass_import,
            commands::scanner::parse_embroidery_file,
            commands::scanner::get_stitch_segments,
            commands::batch::batch_rename,
            commands::batch::batch_organize,
            commands::batch::batch_export_usb,
            commands::ai::ai_build_prompt,
            commands::ai::ai_analyze_file,
            commands::ai::ai_accept_result,
            commands::ai::ai_reject_result,
            commands::ai::ai_test_connection,
            commands::ai::ai_analyze_batch,
            commands::scanner::watcher_auto_import,
            commands::scanner::watcher_remove_by_paths,
            commands::migration::migrate_from_2stitch,
            commands::files::generate_qr_code,
            commands::files::attach_file,
            commands::files::get_attachments,
            commands::files::delete_attachment,
            commands::files::open_attachment,
            commands::files::get_attachment_count,
            commands::files::get_attachment_counts,
            commands::batch::generate_pdf_report,
            commands::convert::get_supported_formats,
            commands::convert::convert_file,
            commands::convert::convert_files_batch,
            commands::edit::preview_transform,
            commands::edit::save_transformed,
            commands::edit::get_stitch_dimensions,
            commands::templates::list_templates,
            commands::templates::instantiate_template,
            commands::versions::get_file_versions,
            commands::versions::restore_version,
            commands::versions::delete_version,
            commands::versions::export_version,
            commands::transfer::list_machines,
            commands::transfer::add_machine,
            commands::transfer::delete_machine,
            commands::transfer::test_machine_connection,
            commands::transfer::transfer_files,
            services::file_watcher::watcher_start,
            services::file_watcher::watcher_stop,
            services::file_watcher::watcher_get_status,
            services::usb_monitor::get_usb_devices,
            services::usb_monitor::usb_monitor_start,
            services::usb_monitor::usb_monitor_stop,
            commands::settings::copy_background_image,
            commands::settings::remove_background_image,
            commands::settings::get_background_image,
            commands::thread_colors::get_thread_matches,
            commands::thread_colors::get_available_brands,
            commands::thread_colors::get_brand_colors,
            commands::backup::create_backup,
            commands::backup::restore_backup,
            commands::backup::check_missing_files,
            commands::backup::relink_file,
            commands::backup::relink_batch,
            commands::backup::export_metadata_json,
            commands::backup::export_metadata_csv,
            commands::backup::soft_delete_file,
            commands::backup::restore_file,
            commands::backup::get_trash,
            commands::backup::purge_file,
            commands::backup::auto_purge_trash,
            commands::backup::archive_file,
            commands::backup::unarchive_file,
            commands::backup::import_metadata_json,
            commands::backup::archive_files_batch,
            commands::backup::unarchive_files_batch,
            commands::backup::export_library,
            commands::backup::import_library,
            commands::projects::create_project,
            commands::projects::get_projects,
            commands::projects::get_project,
            commands::projects::update_project,
            commands::projects::delete_project,
            commands::projects::duplicate_project,
            commands::projects::set_project_details,
            commands::projects::get_project_details,
            commands::projects::create_collection,
            commands::projects::get_collections,
            commands::projects::delete_collection,
            commands::projects::add_to_collection,
            commands::projects::remove_from_collection,
            commands::projects::get_collection_files,
            commands::manufacturing::create_supplier,
            commands::manufacturing::get_suppliers,
            commands::manufacturing::get_supplier,
            commands::manufacturing::update_supplier,
            commands::manufacturing::delete_supplier,
            commands::manufacturing::create_material,
            commands::manufacturing::get_materials,
            commands::manufacturing::get_material,
            commands::manufacturing::update_material,
            commands::manufacturing::delete_material,
            commands::manufacturing::get_inventory,
            commands::manufacturing::update_inventory,
            commands::manufacturing::get_low_stock_materials,
            commands::manufacturing::create_product,
            commands::manufacturing::get_products,
            commands::manufacturing::get_product,
            commands::manufacturing::update_product,
            commands::manufacturing::delete_product,
            commands::manufacturing::add_bom_entry,
            commands::manufacturing::get_bom_entries,
            commands::manufacturing::update_bom_entry,
            commands::manufacturing::delete_bom_entry,
            commands::manufacturing::create_time_entry,
            commands::manufacturing::get_time_entries,
            commands::manufacturing::update_time_entry,
            commands::manufacturing::delete_time_entry,
            commands::print::get_printers,
            commands::print::print_pdf,
            commands::print::compute_tiles,
            commands::print::mark_as_printed,
            commands::print::get_recently_printed,
            commands::print::save_print_settings,
            commands::print::load_print_settings,
            commands::viewer::read_file_bytes,
            commands::viewer::toggle_bookmark,
            commands::viewer::get_bookmarks,
            commands::viewer::update_bookmark_label,
            commands::viewer::add_note,
            commands::viewer::update_note,
            commands::viewer::delete_note,
            commands::viewer::get_notes,
            commands::viewer::set_last_viewed_page,
            commands::viewer::get_last_viewed_page,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
