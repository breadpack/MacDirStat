use tauri::State;

use crate::models::VolumeInfo;
use crate::platform;
use crate::state::ScanState;

#[tauri::command]
pub fn get_system_volumes() -> Result<Vec<VolumeInfo>, String> {
    Ok(platform::get_volumes())
}

#[tauri::command]
pub fn open_in_finder(path: String) -> Result<(), String> {
    platform::open_in_finder(&path)
}

#[tauri::command]
pub fn move_to_trash(path: String, state: State<'_, ScanState>) -> Result<(), String> {
    let canonical = std::path::Path::new(&path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let canonical_str = canonical.to_string_lossy();
    for root in PROTECTED_ROOTS {
        if canonical_str == *root || canonical_str.starts_with(&format!("{}/", root)) {
            return Err(format!("Cannot trash protected system path: {}", root));
        }
    }
    if !canonical.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    trash::delete(&canonical).map_err(|e| e.to_string())?;
    state.remove_path(&path);
    Ok(())
}

/// Protected system paths that must never be permanently deleted.
pub(crate) const PROTECTED_ROOTS: &[&str] = &["/", "/System", "/usr", "/bin", "/sbin", "/Users"];

#[tauri::command]
pub fn permanent_delete(path: String, state: State<'_, ScanState>) -> Result<(), String> {
    // Block deletion of protected system paths
    let canonical = std::path::Path::new(&path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let canonical_str = canonical.to_string_lossy();
    for root in PROTECTED_ROOTS {
        if canonical_str == *root || canonical_str.starts_with(&format!("{}/", root)) {
            return Err(format!("Cannot delete protected system path: {}", root));
        }
    }

    if !canonical.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if canonical.is_dir() {
        std::fs::remove_dir_all(&canonical).map_err(|e| e.to_string())?;
    } else {
        std::fs::remove_file(&canonical).map_err(|e| e.to_string())?;
    }
    state.remove_path(&path);
    Ok(())
}

#[tauri::command]
pub fn show_get_info(path: String) -> Result<(), String> {
    platform::show_get_info(&path)
}

#[tauri::command]
pub fn open_in_terminal(path: String) -> Result<(), String> {
    platform::open_in_terminal(&path)
}

#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    platform::open_file(&path)
}

#[tauri::command]
pub fn check_full_disk_access() -> bool {
    platform::check_full_disk_access()
}

#[tauri::command]
pub fn open_full_disk_access_settings() -> Result<(), String> {
    platform::open_full_disk_access_settings()
}
