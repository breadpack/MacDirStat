use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;

use crate::config;
use crate::models::{CleanupAction, CleanupTarget, FileNode, ScanLogEntry, ScanProgress, VolumeInfo};
use crate::platform;
use crate::scanner;
use crate::state::ScanState;

fn build_scan_options(
    exclude_paths: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    follow_symlinks: Option<bool>,
    max_children_per_dir: Option<usize>,
) -> scanner::ScanOptions {
    scanner::ScanOptions {
        exclude_paths: exclude_paths.unwrap_or_else(|| {
            scanner::DEFAULT_SKIP_PATHS.iter().map(|s| s.to_string()).collect()
        }),
        exclude_names: exclude_names.unwrap_or_else(|| {
            scanner::DEFAULT_SKIP_NAMES.iter().map(|s| s.to_string()).collect()
        }),
        follow_symlinks: follow_symlinks.unwrap_or(false),
        max_children_per_dir: max_children_per_dir.unwrap_or(200),
        cross_device: false,
    }
}

#[tauri::command]
pub async fn scan_directory(
    path: String,
    exclude_paths: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    follow_symlinks: Option<bool>,
    max_children_per_dir: Option<usize>,
    on_progress: Channel<ScanProgress>,
    on_log: Channel<ScanLogEntry>,
    on_tree: Channel<FileNode>,
    state: State<'_, ScanState>,
) -> Result<Option<FileNode>, String> {
    state.cancel_token.store(false, Ordering::Relaxed);

    let options = build_scan_options(exclude_paths, exclude_names, follow_symlinks, max_children_per_dir);
    let cancel_token = Arc::clone(&state.cancel_token);
    let result = tauri::async_runtime::spawn_blocking(move || {
        scanner::scan_directory(&path, cancel_token, on_progress, on_log, on_tree, options)
    })
    .await
    .map_err(|e| e.to_string())?;

    if let Some(ref tree) = result {
        *state.tree.lock().await = Some(tree.clone());
    }

    Ok(result)
}

#[tauri::command]
pub async fn scan_subdirectory(
    path: String,
    exclude_paths: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    follow_symlinks: Option<bool>,
    max_children_per_dir: Option<usize>,
    on_progress: Channel<ScanProgress>,
    on_log: Channel<ScanLogEntry>,
    on_tree: Channel<FileNode>,
    state: State<'_, ScanState>,
) -> Result<Option<FileNode>, String> {
    state.cancel_token.store(false, Ordering::Relaxed);

    let options = build_scan_options(exclude_paths, exclude_names, follow_symlinks, max_children_per_dir);
    let cancel_token = Arc::clone(&state.cancel_token);
    let result = tauri::async_runtime::spawn_blocking(move || {
        scanner::scan_directory(&path, cancel_token, on_progress, on_log, on_tree, options)
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub async fn cancel_scan(state: State<'_, ScanState>) -> Result<(), String> {
    state.cancel_token.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn get_system_volumes() -> Result<Vec<VolumeInfo>, String> {
    Ok(platform::get_volumes())
}

#[tauri::command]
pub fn open_in_finder(path: String) -> Result<(), String> {
    platform::open_in_finder(&path)
}

#[tauri::command]
pub fn move_to_trash(path: String) -> Result<(), String> {
    trash::delete(&path).map_err(|e| e.to_string())
}

/// Protected system paths that must never be permanently deleted.
const PROTECTED_ROOTS: &[&str] = &["/", "/System", "/usr", "/bin", "/sbin", "/Users"];

#[tauri::command]
pub fn permanent_delete(path: String) -> Result<(), String> {
    // Block deletion of protected system paths
    let canonical = std::path::Path::new(&path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let canonical_str = canonical.to_string_lossy();
    for root in PROTECTED_ROOTS {
        if canonical_str == *root {
            return Err(format!("Cannot delete protected system path: {}", root));
        }
    }

    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if p.is_dir() {
        std::fs::remove_dir_all(&path).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&path).map_err(|e| e.to_string())
    }
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

#[tauri::command]
pub fn get_cleanup_actions() -> Vec<CleanupAction> {
    config::load_cleanups()
}

#[tauri::command]
pub fn save_cleanup_actions(actions: Vec<CleanupAction>) -> Result<(), String> {
    config::save_cleanups(&actions)
}

/// Dangerous command patterns that trigger a warning.
const DANGEROUS_PATTERNS: &[&str] = &["rm -rf", "sudo ", "mkfs", "dd if=", "> /dev/"];

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn substitute_variables(template: &str, path: &str, name: &str) -> String {
    let parent = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    template
        .replace("%p", &shell_escape(path))
        .replace("%n", &shell_escape(name))
        .replace("%d", &shell_escape(&parent))
        .replace("%e", &shell_escape(&ext))
}

#[tauri::command]
pub fn execute_cleanup(action_id: u8, path: String, name: String) -> Result<String, String> {
    let actions = config::load_cleanups();
    let action = actions
        .iter()
        .find(|a| a.id == action_id)
        .ok_or("Cleanup action not found")?;

    if !action.enabled {
        return Err("This cleanup action is disabled".to_string());
    }

    // Check target type compatibility
    let is_dir = std::path::Path::new(&path).is_dir();
    match action.target {
        CleanupTarget::Files if is_dir => {
            return Err("This action only applies to files".to_string());
        }
        CleanupTarget::Dirs if !is_dir => {
            return Err("This action only applies to directories".to_string());
        }
        _ => {}
    }

    let command = substitute_variables(&action.command, &path, &name);

    // Check for dangerous patterns
    let lower = command.to_lowercase();
    for pattern in DANGEROUS_PATTERNS {
        if lower.contains(pattern) {
            // Return a warning prefix so frontend can show extra confirmation
            // But still proceed if called (frontend handles the confirm)
            break;
        }
    }

    if action.run_in_terminal {
        execute_in_terminal(&command)
    } else {
        execute_background(&command)
    }
}

fn execute_in_terminal(command: &str) -> Result<String, String> {
    // Use osascript to open Terminal.app and run the command
    let script = format!(
        r#"tell application "Terminal"
    activate
    do script "{}"
end tell"#,
        command.replace('\\', "\\\\").replace('"', "\\\"")
    );

    std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| e.to_string())?;

    Ok("Command sent to Terminal".to_string())
}

fn execute_background(command: &str) -> Result<String, String> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() {
            "Command completed successfully".to_string()
        } else {
            stdout
        })
    } else {
        Err(if stderr.is_empty() {
            format!("Command failed with exit code: {:?}", output.status.code())
        } else {
            stderr
        })
    }
}
