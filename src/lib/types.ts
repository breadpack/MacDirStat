export interface FileNode {
  name: string;
  path: string;
  size: number;
  is_dir: boolean;
  children: FileNode[];
  file_count: number;
  extension: string | null;
  dir_count: number;
  modified: number | null;
  is_symlink: boolean;
  is_hidden: boolean;
  is_readonly: boolean;
}

export interface ScanProgress {
  phase: "structure" | "files";
  files_scanned: number;
  current_path: string;
  total_dirs: number;
  completed_dirs: number;
  current_dir_name: string;
  current_dir_files: number;
  current_dir_bytes: number;
  total_bytes: number;
  scanning_dirs: string[];
  dir_sizes: [string, number, number][];
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

export interface FlatTreeRow {
  node: FileNode;
  depth: number;
  expanded: boolean;
  hasChildren: boolean;
  displaySize: number;
}
