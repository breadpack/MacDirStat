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

/// 프론트엔드로 전달되는 단일 노드 정보 (자식은 포함하지 않음)
#[derive(Debug, Clone, Serialize)]
pub struct NodeDto {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub file_count: u64,
    pub extension: Option<String>,
    pub dir_count: u64,
    pub modified: Option<u64>,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub is_readonly: bool,
    pub cleanup_pattern_id: Option<String>,
    pub child_count: u32,
}

/// 트리맵 레이아웃용 경량 트리 노드
#[derive(Debug, Clone, Serialize)]
pub struct TreemapNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub extension: Option<String>,
    pub cleanup_pattern_id: Option<String>,
    pub children: Vec<TreemapNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionStat {
    pub extension: String,
    pub total_bytes: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub phase: String,
    pub files_scanned: u64,
    pub current_path: String,
    pub total_dirs: u32,
    pub completed_dirs: u32,
    pub total_bytes: u64,
    /// 백엔드 트리가 갱신될 때마다 증가. 프론트엔드는 값이 바뀌면 필요한 부분을 다시 조회한다.
    pub tree_version: u64,
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
