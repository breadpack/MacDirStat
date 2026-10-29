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
    check_deletable(&canonical)?;
    if !canonical.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    trash::delete(&canonical).map_err(|e| e.to_string())?;
    state.remove_path(&path);
    Ok(())
}

/// 하위 전체가 보호되는 시스템 경로
const PROTECTED_TREES: &[&str] = &["/System", "/usr", "/bin", "/sbin"];
/// 경로 자체만 보호되는 경로 (하위 항목은 삭제 가능)
const PROTECTED_EXACT: &[&str] = &["/", "/Users"];

/// 삭제/휴지통 이동이 금지된 경로면 사유를 반환한다.
/// home: 현재 사용자 홈 디렉토리 (홈 자체와 ~/Library를 보호)
fn protected_reason(path: &str, home: Option<&str>) -> Option<String> {
    let path = if path.len() > 1 { path.trim_end_matches('/') } else { path };
    if let Some(root) = PROTECTED_TREES.iter().find(|r| path == **r || path.starts_with(&format!("{r}/"))) {
        return Some(format!("Cannot delete protected system path: {root}"));
    }
    if PROTECTED_EXACT.contains(&path) {
        return Some(format!("Cannot delete protected system path: {path}"));
    }
    // /Users/<name>: 사용자 홈 폴더(및 /Users/Shared) 자체
    if path.strip_prefix("/Users/").is_some_and(|rest| !rest.is_empty() && !rest.contains('/')) {
        return Some(format!("Cannot delete user home folder: {path}"));
    }
    if let Some(home) = home.map(|h| h.trim_end_matches('/')) {
        if path == home || path == format!("{home}/Library") {
            return Some(format!("Cannot delete protected home path: {path}"));
        }
    }
    None
}

/// 정규화된 경로가 삭제 가능한지 검사한다.
pub(crate) fn check_deletable(canonical: &std::path::Path) -> Result<(), String> {
    let home = std::env::var("HOME").ok();
    match protected_reason(&canonical.to_string_lossy(), home.as_deref()) {
        Some(reason) => Err(reason),
        None => Ok(()),
    }
}

#[tauri::command]
pub fn permanent_delete(path: String, state: State<'_, ScanState>) -> Result<(), String> {
    let canonical = std::path::Path::new(&path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    check_deletable(&canonical)?;

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

#[cfg(test)]
mod tests {
    use super::protected_reason;

    const HOME: Option<&str> = Some("/Users/me");

    #[test]
    fn allows_paths_inside_home() {
        for p in ["/Users/me/Projects/build", "/Users/me/.codex/worktrees/x", "/Users/me/Library/Caches", "/Users/other/tmp", "/Applications/Foo.app"] {
            assert_eq!(protected_reason(p, HOME), None, "{p}");
        }
    }

    #[test]
    fn blocks_system_trees_and_top_level_roots() {
        for p in ["/", "/Users", "/Users/", "/System", "/System/Library/x", "/usr/local/bin", "/bin", "/sbin/x"] {
            assert!(protected_reason(p, HOME).is_some(), "{p}");
        }
    }

    #[test]
    fn blocks_home_folders_and_library() {
        for p in ["/Users/me", "/Users/me/", "/Users/Shared", "/Users/other", "/Users/me/Library"] {
            assert!(protected_reason(p, HOME).is_some(), "{p}");
        }
    }
}
