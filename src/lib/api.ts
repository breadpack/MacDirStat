import { invoke, type Channel } from "@tauri-apps/api/core";
import type { CleanupAction, CleanupPatternInfo, CleanupRecommendation, CleanupResult, CleanupScanProgress, FileNode, ScanLogEntry, ScanProgress, VolumeInfo } from "./types";

export interface ScanOptions {
  excludePaths?: string[];
  excludeNames?: string[];
  followSymlinks?: boolean;
  maxChildrenPerDir?: number;
}

export async function scanDirectory(
  path: string,
  onProgress: Channel<ScanProgress>,
  onLog: Channel<ScanLogEntry>,
  onTree: Channel<FileNode>,
  options?: ScanOptions,
): Promise<FileNode | null> {
  return invoke<FileNode | null>("scan_directory", {
    path,
    excludePaths: options?.excludePaths,
    excludeNames: options?.excludeNames,
    followSymlinks: options?.followSymlinks,
    maxChildrenPerDir: options?.maxChildrenPerDir,
    onProgress,
    onLog,
    onTree,
  });
}

export async function scanSubdirectory(
  path: string,
  onProgress: Channel<ScanProgress>,
  onLog: Channel<ScanLogEntry>,
  onTree: Channel<FileNode>,
  options?: ScanOptions,
): Promise<FileNode | null> {
  return invoke<FileNode | null>("scan_subdirectory", {
    path,
    excludePaths: options?.excludePaths,
    excludeNames: options?.excludeNames,
    followSymlinks: options?.followSymlinks,
    maxChildrenPerDir: options?.maxChildrenPerDir,
    onProgress,
    onLog,
    onTree,
  });
}

export async function cancelScan(): Promise<void> {
  return invoke("cancel_scan");
}

export async function getSystemVolumes(): Promise<VolumeInfo[]> {
  return invoke<VolumeInfo[]>("get_system_volumes");
}

export async function openInFinder(path: string): Promise<void> {
  return invoke("open_in_finder", { path });
}

export async function moveToTrash(path: string): Promise<void> {
  return invoke("move_to_trash", { path });
}

export async function permanentDelete(path: string): Promise<void> {
  return invoke("permanent_delete", { path });
}

export async function showGetInfo(path: string): Promise<void> {
  return invoke("show_get_info", { path });
}

export async function openInTerminal(path: string): Promise<void> {
  return invoke("open_in_terminal", { path });
}

export async function openFile(path: string): Promise<void> {
  return invoke("open_file", { path });
}

export async function checkFullDiskAccess(): Promise<boolean> {
  return invoke<boolean>("check_full_disk_access");
}

export async function openFullDiskAccessSettings(): Promise<void> {
  return invoke("open_full_disk_access_settings");
}

export async function getCleanupActions(): Promise<CleanupAction[]> {
  return invoke<CleanupAction[]>("get_cleanup_actions");
}

export async function saveCleanupActions(actions: CleanupAction[]): Promise<void> {
  return invoke("save_cleanup_actions", { actions });
}

export async function executeCleanup(
  actionId: number,
  path: string,
  name: string,
): Promise<string> {
  return invoke<string>("execute_cleanup", { actionId, path, name });
}

export async function scanCleanupRecommendations(
  onProgress: Channel<CleanupScanProgress>,
): Promise<CleanupRecommendation[]> {
  return invoke("scan_cleanup_recommendations", { onProgress });
}

export async function executeCleanupRecommendation(
  patternId: string,
  paths: string[],
): Promise<CleanupResult> {
  return invoke("execute_cleanup_recommendation", { patternId, paths });
}

export async function getCleanupPatterns(): Promise<CleanupPatternInfo[]> {
  return invoke("get_cleanup_patterns");
}
