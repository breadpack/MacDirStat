use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;

use crate::cleanup_patterns::{
    self, CleanupMethod, CleanupPatternInfo, CleanupRecommendation, CleanupResult,
    CleanupScanProgress, DetectionMethod,
};
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
    scan_directory(path, exclude_paths, exclude_names, follow_symlinks, max_children_per_dir, on_progress, on_log, on_tree, state).await
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
    trash::delete(&canonical).map_err(|e| e.to_string())
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
        if canonical_str == *root || canonical_str.starts_with(&format!("{}/", root)) {
            return Err(format!("Cannot delete protected system path: {}", root));
        }
    }

    if !canonical.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if canonical.is_dir() {
        std::fs::remove_dir_all(&canonical).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&canonical).map_err(|e| e.to_string())
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
            return Err(format!("Command contains dangerous pattern '{}'. Refusing to execute.", pattern));
        }
    }

    if action.run_in_terminal {
        execute_in_terminal(&command)
    } else {
        execute_background(&command)
    }
}

fn execute_in_terminal(command: &str) -> Result<String, String> {
    // Wrap the command so the terminal tab closes automatically after completion.
    // "exit" causes the shell to exit; Terminal preference "close if shell exited cleanly"
    // handles the rest. We also add explicit tab-close via AppleScript after a delay.
    let wrapped = format!(
        "{}; echo ''; echo 'Press Enter to close...'; read; exit",
        command
    );
    let escaped = wrapped.replace('\\', "\\\\").replace('"', "\\\"");

    let script = format!(
        r#"tell application "Terminal"
    activate
    do script "{}"
end tell"#,
        escaped
    );

    std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| e.to_string())?;

    Ok("Command sent to Terminal".to_string())
}

/// Detect the user's login shell (defaults to /bin/zsh on macOS)
fn user_shell() -> String {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    const ALLOWED_SHELLS: &[&str] = &["/bin/sh", "/bin/bash", "/bin/zsh", "/usr/local/bin/fish", "/opt/homebrew/bin/fish", "/bin/dash", "/usr/local/bin/bash"];
    if ALLOWED_SHELLS.contains(&shell.as_str()) {
        shell
    } else {
        "/bin/zsh".to_string()
    }
}

fn execute_background(command: &str) -> Result<String, String> {
    let shell = user_shell();
    let output = std::process::Command::new(&shell)
        .arg("-l") // login shell: loads ~/.zprofile, ~/.zshrc etc. for full PATH
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

// ── Cleanup recommendation commands ──

#[tauri::command]
pub async fn scan_cleanup_recommendations(
    on_progress: Channel<CleanupScanProgress>,
) -> Result<Vec<CleanupRecommendation>, String> {
    tokio::task::spawn_blocking(move || {
        let patterns = cleanup_patterns::all_patterns();
        let total = patterns.len();
        let mut recommendations = Vec::new();

        for (i, pattern) in patterns.iter().enumerate() {
            let _ = on_progress.send(CleanupScanProgress {
                current_pattern: pattern.name.to_string(),
                checked: i + 1,
                total,
            });

            match &pattern.detection {
                DetectionMethod::KnownPath { path, .. } => {
                    let expanded = cleanup_patterns::expand_home(path);
                    if expanded.exists() {
                        let size = cleanup_patterns::dir_size(&expanded);
                        recommendations.push(CleanupRecommendation {
                            pattern_id: pattern.id.to_string(),
                            pattern_name: pattern.name.to_string(),
                            category: pattern.category.clone(),
                            risk_level: pattern.risk_level.clone(),
                            description: pattern.description.to_string(),
                            paths: vec![expanded.to_string_lossy().to_string()],
                            total_size: size,
                            cleanup_method: pattern.cleanup.clone(),
                        });
                    }
                }
                DetectionMethod::Command { check_cmd, .. } => {
                    // Special handling: expand simulator runtimes into individual items
                    if pattern.id == "xcode-simulator-runtimes" {
                        recommendations.extend(scan_simulator_runtimes(pattern));
                        continue;
                    }

                    let shell = user_shell();
                    let output = std::process::Command::new(&shell)
                        .arg("-l")
                        .arg("-c")
                        .arg(check_cmd)
                        .output();

                    if let Ok(output) = output {
                        let stdout =
                            String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !stdout.is_empty() && stdout != "0" {
                            recommendations.push(CleanupRecommendation {
                                pattern_id: pattern.id.to_string(),
                                pattern_name: pattern.name.to_string(),
                                category: pattern.category.clone(),
                                risk_level: pattern.risk_level.clone(),
                                description: pattern.description.to_string(),
                                paths: vec![],
                                total_size: 0,
                                cleanup_method: pattern.cleanup.clone(),
                            });
                        }
                    }
                }
                DetectionMethod::PathPattern { .. } => {
                    // PathPattern is handled during scan, skip here
                }
            }
        }

        recommendations.sort_by(|a, b| b.total_size.cmp(&a.total_size));
        Ok(recommendations)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn execute_cleanup_recommendation(
    pattern_id: String,
    paths: Vec<String>,
    cleanup_method: Option<CleanupMethod>,
) -> Result<CleanupResult, String> {
    let patterns = cleanup_patterns::all_patterns();
    let pattern = patterns.iter().find(|p| p.id == pattern_id);

    // For dynamically generated patterns (e.g., xcode-simulator-runtimes-*),
    // use the cleanup_method passed from the frontend recommendation.
    let cleanup = match (&pattern, &cleanup_method) {
        (Some(p), _) => p.cleanup.clone(),
        (None, Some(m)) => {
            if !pattern_id.starts_with("xcode-simulator-runtimes-") {
                return Err(format!("Untrusted cleanup method for unknown pattern: {}", pattern_id));
            }
            m.clone()
        },
        (None, None) => return Err(format!("Pattern not found: {}", pattern_id)),
    };

    match &cleanup {
        CleanupMethod::Delete { use_trash } => {
            let use_trash = *use_trash;
            let mut total_freed: u64 = 0;
            let mut deleted_count: usize = 0;

            for path_str in &paths {
                let p = std::path::Path::new(path_str);
                if !p.exists() {
                    continue;
                }

                // Validate not a protected path
                let canonical = p.canonicalize().map_err(|e| {
                    format!("Cannot resolve path {}: {}", path_str, e)
                })?;
                let canonical_str = canonical.to_string_lossy();
                for root in PROTECTED_ROOTS {
                    if canonical_str.as_ref() == *root || canonical_str.starts_with(&format!("{}/", root)) {
                        return Err(format!(
                            "Cannot delete protected system path: {}",
                            root
                        ));
                    }
                }

                let size = cleanup_patterns::dir_size(&canonical);

                if use_trash {
                    trash::delete(&canonical).map_err(|e| e.to_string())?;
                } else if canonical.is_dir() {
                    std::fs::remove_dir_all(&canonical).map_err(|e| e.to_string())?;
                } else {
                    std::fs::remove_file(&canonical).map_err(|e| e.to_string())?;
                }

                total_freed += size;
                deleted_count += 1;
            }

            Ok(CleanupResult {
                success: true,
                freed_bytes: total_freed,
                message: format!(
                    "Deleted {} path(s), freed {} bytes",
                    deleted_count,
                    total_freed
                ),
            })
        }
        CleanupMethod::ShellCommand {
            command,
            run_in_terminal,
            ..
        } => {
            let message = if *run_in_terminal {
                execute_in_terminal(command)?
            } else {
                execute_background(command)?
            };

            Ok(CleanupResult {
                success: true,
                freed_bytes: 0,
                message,
            })
        }
        CleanupMethod::OpenInFinder => {
            if let Some(first_path) = paths.first() {
                platform::open_in_finder(first_path)?;
            }

            Ok(CleanupResult {
                success: true,
                freed_bytes: 0,
                message: "Opened in Finder".to_string(),
            })
        }
    }
}

/// Parse `xcrun simctl runtime list -j` and return individual recommendations per runtime.
fn scan_simulator_runtimes(pattern: &cleanup_patterns::CleanupPattern) -> Vec<CleanupRecommendation> {
    let shell = user_shell();
    let output = std::process::Command::new(&shell)
        .arg("-l")
        .arg("-c")
        .arg("xcrun simctl runtime list -j 2>/dev/null")
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let obj = match json.as_object() {
        Some(o) => o,
        None => return vec![],
    };

    let mut results = Vec::new();
    for (_uuid, info) in obj {
        let version = info.get("version").and_then(|v| v.as_str()).unwrap_or("?");
        let build = info.get("build").and_then(|v| v.as_str()).unwrap_or("?");
        let size_bytes = info.get("sizeBytes").and_then(|v| v.as_u64()).unwrap_or(0);
        let identifier = info.get("identifier").and_then(|v| v.as_str()).unwrap_or("");
        let platform = info.get("platformIdentifier").and_then(|v| v.as_str()).unwrap_or("");
        let deletable = info.get("deletable").and_then(|v| v.as_bool()).unwrap_or(false);
        let path = info.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let last_used = info.get("lastUsedAt").and_then(|v| v.as_str()).unwrap_or("unknown");

        if !deletable || identifier.is_empty() {
            continue;
        }

        // Extract platform name (e.g., "iphonesimulator" -> "iOS")
        let platform_name = if platform.contains("iphone") {
            "iOS"
        } else if platform.contains("appletv") {
            "tvOS"
        } else if platform.contains("watch") {
            "watchOS"
        } else if platform.contains("xros") {
            "visionOS"
        } else {
            "Simulator"
        };

        let display_name = format!("{} {} ({})", platform_name, version, build);
        let description = format!(
            "{} simulator runtime. Last used: {}.",
            display_name,
            &last_used[..10.min(last_used.len())]
        );

        let delete_cmd = format!("xcrun simctl runtime delete {}", shell_escape(identifier));

        results.push(CleanupRecommendation {
            pattern_id: format!("xcode-simulator-runtimes-{}", identifier),
            pattern_name: display_name,
            category: pattern.category.clone(),
            risk_level: pattern.risk_level.clone(),
            description,
            paths: if path.is_empty() { vec![] } else { vec![path.to_string()] },
            total_size: size_bytes,
            cleanup_method: CleanupMethod::ShellCommand {
                command: delete_cmd,
                run_in_terminal: false,
                refresh_after: false,
            },
        });
    }

    results.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    results
}

#[tauri::command]
pub fn get_cleanup_patterns() -> Vec<CleanupPatternInfo> {
    cleanup_patterns::all_patterns()
        .iter()
        .map(CleanupPatternInfo::from)
        .collect()
}
