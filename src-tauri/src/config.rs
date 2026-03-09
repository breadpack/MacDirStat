use std::fs;
use std::path::PathBuf;

use crate::models::{CleanupAction, CleanupTarget};

fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.macdirstat.app");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

fn cleanups_path() -> PathBuf {
    config_dir().join("cleanups.json")
}

pub fn default_cleanups() -> Vec<CleanupAction> {
    vec![
        CleanupAction {
            id: 0,
            name: "Compress (zip)".to_string(),
            command: "zip -r %p.zip %p".to_string(),
            enabled: true,
            target: CleanupTarget::Both,
            confirm: true,
            run_in_terminal: false,
            refresh_after: true,
        },
        CleanupAction {
            id: 1,
            name: "Calculate checksum".to_string(),
            command: "shasum -a 256 %p".to_string(),
            enabled: true,
            target: CleanupTarget::Files,
            confirm: false,
            run_in_terminal: true,
            refresh_after: false,
        },
        CleanupAction {
            id: 2,
            name: "List contents".to_string(),
            command: "ls -lahR %p".to_string(),
            enabled: true,
            target: CleanupTarget::Dirs,
            confirm: false,
            run_in_terminal: true,
            refresh_after: false,
        },
        CleanupAction {
            id: 3,
            name: "Disk usage".to_string(),
            command: "du -sh %p".to_string(),
            enabled: true,
            target: CleanupTarget::Both,
            confirm: false,
            run_in_terminal: true,
            refresh_after: false,
        },
    ]
}

pub fn load_cleanups() -> Vec<CleanupAction> {
    let path = cleanups_path();
    if !path.exists() {
        let defaults = default_cleanups();
        let _ = save_cleanups(&defaults);
        return defaults;
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| default_cleanups()),
        Err(_) => default_cleanups(),
    }
}

pub fn save_cleanups(actions: &[CleanupAction]) -> Result<(), String> {
    let path = cleanups_path();
    let json = serde_json::to_string_pretty(actions).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}
