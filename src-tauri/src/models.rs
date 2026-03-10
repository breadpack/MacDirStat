use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupTarget {
    Files,
    Dirs,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupAction {
    pub id: u8,
    pub name: String,
    pub command: String,
    pub enabled: bool,
    pub target: CleanupTarget,
    pub confirm: bool,
    pub run_in_terminal: bool,
    pub refresh_after: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
    pub file_count: u64,
    pub extension: Option<String>,
    #[serde(default)]
    pub dir_count: u64,
    #[serde(default)]
    pub modified: Option<u64>,
    #[serde(default)]
    pub is_symlink: bool,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub is_readonly: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_pattern_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub phase: String,
    pub files_scanned: u64,
    pub current_path: String,
    pub total_dirs: u32,
    pub completed_dirs: u32,
    pub current_dir_name: String,
    pub current_dir_files: u64,
    pub current_dir_bytes: u64,
    pub total_bytes: u64,
    pub scanning_dirs: Vec<String>,
    /// 디렉토리별 실시간 크기: [(path, size, file_count), ...]
    pub dir_sizes: Vec<(String, u64, u64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanLogEntry {
    pub level: String,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VolumeInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}
