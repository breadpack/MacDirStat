pub mod dev_tools;
pub mod package_managers;
pub mod containers;
pub mod browsers;
pub mod ides;
pub mod system;
pub mod cloud_storage;
pub mod app_data;
pub mod media;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ── Enums ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CleanupCategory {
    DevTools,
    PackageManagers,
    Containers,
    Browsers,
    Ides,
    System,
    CloudStorage,
    AppData,
    Media,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DetectionMethod {
    PathPattern {
        dir_name: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        path_contains: Option<&'static str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_marker: Option<&'static str>,
    },
    KnownPath {
        path: &'static str,
        expandable: bool,
    },
    Command {
        check_cmd: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        parse_hint: Option<&'static str>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CleanupMethod {
    Delete {
        use_trash: bool,
    },
    ShellCommand {
        command: String,
        run_in_terminal: bool,
        refresh_after: bool,
    },
    OpenInFinder,
}

// ── Pattern struct ──

#[derive(Debug, Clone)]
pub struct CleanupPattern {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: CleanupCategory,
    pub risk: RiskLevel,
    pub detection: DetectionMethod,
    pub cleanup: CleanupMethod,
}

// ── Serializable types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPatternInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: CleanupCategory,
    pub risk: RiskLevel,
}

impl From<&CleanupPattern> for CleanupPatternInfo {
    fn from(p: &CleanupPattern) -> Self {
        CleanupPatternInfo {
            id: p.id.to_string(),
            name: p.name.to_string(),
            description: p.description.to_string(),
            category: p.category.clone(),
            risk: p.risk.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRecommendation {
    pub pattern_id: String,
    pub path: String,
    pub size: u64,
    pub name: String,
    pub description: String,
    pub category: CleanupCategory,
    pub risk: RiskLevel,
    pub cleanup: CleanupMethod,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanupScanProgress {
    pub phase: String,
    pub checked: u32,
    pub total: u32,
    pub current_pattern: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanupResult {
    pub recommendations: Vec<CleanupRecommendation>,
    pub total_reclaimable: u64,
}

// ── Functions ──

pub fn all_patterns() -> Vec<CleanupPattern> {
    let mut patterns = Vec::new();
    patterns.extend(dev_tools::patterns());
    patterns.extend(package_managers::patterns());
    patterns.extend(containers::patterns());
    patterns.extend(browsers::patterns());
    patterns.extend(ides::patterns());
    patterns.extend(system::patterns());
    patterns.extend(cloud_storage::patterns());
    patterns.extend(app_data::patterns());
    patterns.extend(media::patterns());
    patterns
}

/// Build a map from dir_name -> Vec<CleanupPattern> for PathPattern-based detection
pub fn build_scan_pattern_map() -> HashMap<String, Vec<CleanupPattern>> {
    let mut map: HashMap<String, Vec<CleanupPattern>> = HashMap::new();
    for p in all_patterns() {
        if let DetectionMethod::PathPattern { dir_name, .. } = &p.detection {
            map.entry(dir_name.to_string())
                .or_default()
                .push(p);
        }
    }
    map
}

/// Check if a path matches a PathPattern's additional constraints
pub fn matches_path_pattern(path: &Path, pattern: &CleanupPattern) -> bool {
    if let DetectionMethod::PathPattern {
        dir_name: _,
        path_contains,
        parent_marker,
    } = &pattern.detection
    {
        let path_str = path.to_string_lossy();
        if let Some(contains) = path_contains {
            if !path_str.contains(contains) {
                return false;
            }
        }
        if let Some(marker) = parent_marker {
            if let Some(parent) = path.parent() {
                if !parent.join(marker).exists() {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    } else {
        false
    }
}

/// Expand ~ to the user's home directory
pub fn expand_home(path: &str) -> PathBuf {
    if path.starts_with('~') {
        if let Some(home) = dirs_next_home() {
            return PathBuf::from(path.replacen('~', &home, 1));
        }
    }
    PathBuf::from(path)
}

fn dirs_next_home() -> Option<String> {
    std::env::var("HOME").ok()
}

/// Recursively calculate directory size using walkdir
pub fn dir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
