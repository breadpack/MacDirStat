/** 백엔드 트리의 단일 노드 요약. 자식은 getChildren()으로 별도 조회한다. */
export interface FileNode {
  name: string;
  path: string;
  size: number;
  is_dir: boolean;
  file_count: number;
  extension: string | null;
  dir_count: number;
  modified: number | null;
  is_symlink: boolean;
  is_hidden: boolean;
  is_readonly: boolean;
  cleanup_pattern_id: string | null;
  child_count: number;
}

/** 트리맵 레이아웃용 경량 트리 (백엔드에서 크기 기준으로 가지치기됨) */
export interface TreemapNode {
  name: string;
  path: string;
  size: number;
  is_dir: boolean;
  extension: string | null;
  cleanup_pattern_id: string | null;
  children: TreemapNode[];
}

export interface RawExtensionStat {
  extension: string;
  total_bytes: number;
  file_count: number;
}

export interface ScanProgress {
  phase: "structure" | "files";
  files_scanned: number;
  current_path: string;
  total_dirs: number;
  completed_dirs: number;
  total_bytes: number;
  /** 백엔드 트리가 갱신될 때마다 증가 */
  tree_version: number;
}

export interface VolumeInfo {
  name: string;
  mount_point: string;
  total_bytes: number;
  available_bytes: number;
}

export interface ScanLogEntry {
  level: "error" | "warn" | "info" | "skip";
  message: string;
  path: string;
}

export type CleanupTarget = "Files" | "Dirs" | "Both";

export interface CleanupAction {
  id: number;
  name: string;
  command: string;
  enabled: boolean;
  target: CleanupTarget;
  confirm: boolean;
  run_in_terminal: boolean;
  refresh_after: boolean;
}

export type CleanupCategory =
  | "DevTools" | "PackageManager" | "Container" | "Browser"
  | "IDE" | "System" | "CloudStorage" | "AppData" | "Media";

export type RiskLevel = "Safe" | "Caution" | "Warning";

export type CleanupMethod =
  | { type: "Delete"; use_trash: boolean }
  | { type: "ShellCommand"; command: string; run_in_terminal: boolean; refresh_after: boolean }
  | { type: "OpenInFinder" };

export interface CleanupRecommendation {
  pattern_id: string;
  pattern_name: string;
  category: CleanupCategory;
  risk_level: RiskLevel;
  description: string;
  paths: string[];
  total_size: number;
  cleanup_method: CleanupMethod;
}

export interface CleanupScanProgress {
  current_pattern: string;
  checked: number;
  total: number;
}

export interface CleanupResult {
  success: boolean;
  freed_bytes: number;
  message: string;
}

export interface CleanupPatternInfo {
  id: string;
  category: CleanupCategory;
  name: string;
  description: string;
  risk_level: RiskLevel;
  cleanup_method: CleanupMethod;
}

export interface FlatTreeRow {
  node: FileNode;
  depth: number;
  expanded: boolean;
  hasChildren: boolean;
  displaySize: number;
}
