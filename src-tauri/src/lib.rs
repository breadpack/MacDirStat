mod cleanup_patterns;
mod commands;
mod config;
mod models;
mod platform;
mod scanner;
mod scanner_tree;
mod state;

use state::ScanState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(ScanState::new())
        .invoke_handler(tauri::generate_handler![
            commands::scan_directory,
            commands::scan_subdirectory,
            commands::cancel_scan,
            commands::get_system_volumes,
            commands::open_in_finder,
            commands::move_to_trash,
            commands::permanent_delete,
            commands::show_get_info,
            commands::open_in_terminal,
            commands::open_file,
            commands::check_full_disk_access,
            commands::open_full_disk_access_settings,
            commands::get_cleanup_actions,
            commands::save_cleanup_actions,
            commands::execute_cleanup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
