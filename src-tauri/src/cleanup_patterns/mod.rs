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
pub enum CleanupCategory {
    DevTools,
    PackageManager,
    Container,
    Browser,
    IDE,
    System,
    CloudStorage,
    AppData,
    Media,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Safe,
    Caution,
    Warning,
}

#[derive(Debug, Clone)]
pub enum DetectionMethod {
    PathPattern {
        dir_name: Option<&'static str>,
        path_contains: Option<&'static str>,
        parent_marker: Option<&'static str>,
    },
    KnownPath {
        path: &'static str,
        expandable: bool,
    },
    Command {
        check_cmd: &'static str,
        parse_hint: &'static str,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
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
    pub category: CleanupCategory,
    pub name: &'static str,
    pub description: &'static str,
    pub risk_level: RiskLevel,
    pub detection: DetectionMethod,
    pub cleanup: CleanupMethod,
}

// ── Serializable types for frontend ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPatternInfo {
    pub id: String,
    pub category: CleanupCategory,
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub cleanup_method: CleanupMethod,
}

impl From<&CleanupPattern> for CleanupPatternInfo {
    fn from(p: &CleanupPattern) -> Self {
        CleanupPatternInfo {
            id: p.id.to_string(),
            category: p.category.clone(),
            name: p.name.to_string(),
            description: p.description.to_string(),
            risk_level: p.risk_level.clone(),
            cleanup_method: p.cleanup.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRecommendation {
    pub pattern_id: String,
    pub pattern_name: String,
    pub category: CleanupCategory,
    pub risk_level: RiskLevel,
    pub description: String,
    pub paths: Vec<String>,
    pub total_size: u64,
    pub cleanup_method: CleanupMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupScanProgress {
    pub current_pattern: String,
    pub checked: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub success: bool,
    pub freed_bytes: u64,
    pub message: String,
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

/// Build a map from dir_name -> Vec<usize> (pattern indices) for scan-time matching
pub fn build_scan_pattern_map(patterns: &[CleanupPattern]) -> HashMap<String, Vec<usize>> {
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, p) in patterns.iter().enumerate() {
        if let DetectionMethod::PathPattern { dir_name: Some(name), .. } = &p.detection {
            map.entry(name.to_string()).or_default().push(i);
        }
    }
    map
}

/// Check if a path matches a PathPattern's additional constraints
pub fn matches_path_pattern(pattern: &CleanupPattern, path: &Path) -> bool {
    if let DetectionMethod::PathPattern {
        dir_name,
        path_contains,
        parent_marker,
    } = &pattern.detection
    {
        if let Some(expected_name) = dir_name {
            let actual_name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if actual_name != *expected_name {
                return false;
            }
        }
        if let Some(contains) = path_contains {
            if !path.to_string_lossy().contains(contains) {
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
    if path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/var/empty".to_string());
        PathBuf::from(home).join(&path[2..])
    } else {
        PathBuf::from(path)
    }
}

/// Get the physical size of a file from its metadata
fn file_physical_size(m: &std::fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        m.blocks() * 512
    }
    #[cfg(not(unix))]
    {
        m.len()
    }
}

/// Recursively calculate directory size using walkdir
pub fn dir_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| file_physical_size(&m)).unwrap_or(0);
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| file_physical_size(&m))
        .sum()
}
