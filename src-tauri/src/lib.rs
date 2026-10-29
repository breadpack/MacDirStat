mod cleanup_patterns;
mod commands;
mod config;
mod models;
mod platform;
mod scanner;
mod scanner_files;
mod skeleton;
mod state;
mod tree;

use state::ScanState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(ScanState::new())
        .invoke_handler(tauri::generate_handler![
            commands::scan::scan_directory,
            commands::scan::refresh_subtree,
            commands::scan::cancel_scan,
            commands::scan::get_node,
            commands::scan::get_children,
            commands::scan::get_treemap,
            commands::scan::get_extension_stats,
            commands::fs_ops::get_system_volumes,
            commands::fs_ops::open_in_finder,
            commands::fs_ops::move_to_trash,
            commands::fs_ops::permanent_delete,
            commands::fs_ops::show_get_info,
            commands::fs_ops::open_in_terminal,
            commands::fs_ops::open_file,
            commands::fs_ops::check_full_disk_access,
            commands::fs_ops::open_full_disk_access_settings,
            commands::cleanup::get_cleanup_actions,
            commands::cleanup::save_cleanup_actions,
            commands::cleanup::execute_cleanup,
            commands::cleanup::scan_cleanup_recommendations,
            commands::cleanup::execute_cleanup_recommendation,
            commands::cleanup::get_cleanup_patterns,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
