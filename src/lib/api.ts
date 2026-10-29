import { invoke, type Channel } from "@tauri-apps/api/core";
import type { CleanupAction, CleanupMethod, CleanupPatternInfo, CleanupRecommendation, CleanupResult, CleanupScanProgress, FileNode, RawExtensionStat, ScanLogEntry, ScanProgress, TreemapNode, VolumeInfo } from "./types";

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
  });
}

/** Re-scan a subtree and merge it into the backend tree. Returns the updated root. */
export async function refreshSubtree(
  path: string,
  onProgress: Channel<ScanProgress>,
  onLog: Channel<ScanLogEntry>,
  options?: ScanOptions,
): Promise<FileNode | null> {
  return invoke<FileNode | null>("refresh_subtree", {
    path,
    excludePaths: options?.excludePaths,
    excludeNames: options?.excludeNames,
    followSymlinks: options?.followSymlinks,
    maxChildrenPerDir: options?.maxChildrenPerDir,
    onProgress,
    onLog,
  });
}

export async function getNode(path: string): Promise<FileNode | null> {
  return invoke<FileNode | null>("get_node", { path });
}

/** Children of each path, in input order (null for unknown paths). */
export async function getChildren(paths: string[]): Promise<(FileNode[] | null)[]> {
  return invoke<(FileNode[] | null)[]>("get_children", { paths });
}

export async function getTreemap(
  path: string,
  minRatio: number,
  maxChildren: number,
  maxNodes: number,
): Promise<TreemapNode | null> {
  return invoke<TreemapNode | null>("get_treemap", { path, minRatio, maxChildren, maxNodes });
}

export async function getExtensionStats(path: string): Promise<RawExtensionStat[]> {
  return invoke<RawExtensionStat[]>("get_extension_stats", { path });
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
  cleanupMethod?: CleanupMethod,
): Promise<CleanupResult> {
  return invoke("execute_cleanup_recommendation", { patternId, paths, cleanupMethod });
}

export async function getCleanupPatterns(): Promise<CleanupPatternInfo[]> {
  return invoke("get_cleanup_patterns");
}
