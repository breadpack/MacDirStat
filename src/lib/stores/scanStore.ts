import { writable, get } from "svelte/store";
import { Channel } from "@tauri-apps/api/core";
import type { FileNode, ScanLogEntry, ScanProgress, VolumeInfo } from "../types";
import { scanDirectory, scanSubdirectory, cancelScan, getSystemVolumes, type ScanOptions } from "../api";
import { replaceSubtree } from "../utils/treeUtils";
import { clearHistory } from "./navigationStore";
import { settings } from "./settingsStore";

const INITIAL_PROGRESS: ScanProgress = {
  phase: "structure",
  files_scanned: 0,
  current_path: "",
  total_dirs: 0,
  completed_dirs: 0,
  current_dir_name: "",
  current_dir_files: 0,
  current_dir_bytes: 0,
  total_bytes: 0,
  scanning_dirs: [],
  dir_sizes: [],
};

export const tree = writable<FileNode | null>(null);
export const scanning = writable(false);
export const progress = writable<ScanProgress>(INITIAL_PROGRESS);
export const scanError = writable<string | null>(null);
export const scanCancelled = writable(false);
export const scanLogs = writable<ScanLogEntry[]>([]);
export const scanningDirs = writable<Set<string>>(new Set());
export const scanWarnErrorCount = writable(0);

/** 스캔 중 디렉토리별 실시간 크기: path → { size, file_count } */
export const dirSizes = writable<Map<string, { size: number; file_count: number }>>(new Map());

/** Module-level mutable map for dirSizes to avoid copying on every tick */
let _dirSizesMap = new Map<string, { size: number; file_count: number }>();

/** True while a partial subtree refresh is in progress (does not block full UI). */
export const partialScanning = writable(false);

/** Volume info for the currently scanned path. */
export const currentVolume = writable<VolumeInfo | null>(null);

export async function startScan(path: string) {
  tree.set(null);
  _dirSizesMap = new Map();
  dirSizes.set(_dirSizesMap);
  clearHistory();
  scanning.set(true);
  scanError.set(null);
  scanCancelled.set(false);
  scanLogs.set([]);
  scanWarnErrorCount.set(0);
  progress.set(INITIAL_PROGRESS);
  scanningDirs.set(new Set());
  currentVolume.set(null);

  // Detect volume for scan path
  try {
    const volumes = await getSystemVolumes();
    const vol = volumes
      .filter((v) => path.startsWith(v.mount_point))
      .sort((a, b) => b.mount_point.length - a.mount_point.length)[0];
    currentVolume.set(vol ?? null);
  } catch {
    // Volume detection is best-effort; proceed without it
  }

  const progressChannel = new Channel<ScanProgress>();
  progressChannel.onmessage = (msg) => {
    progress.set(msg);

    if (msg.scanning_dirs) {
      const newDirs = msg.scanning_dirs;
      scanningDirs.update((prev) => {
        if (prev.size === newDirs.length && newDirs.every(d => prev.has(d))) return prev;
        return new Set(newDirs);
      });
    }

    // 디렉토리별 실시간 크기 머지 (부분 업데이트 지원, in-place mutation)
    if (msg.dir_sizes && msg.dir_sizes.length > 0) {
      for (const [path, size, fc] of msg.dir_sizes) {
        _dirSizesMap.set(path, { size, file_count: fc });
      }
      dirSizes.set(_dirSizesMap);
    }
  };

  const logChannel = new Channel<ScanLogEntry>();
  logChannel.onmessage = (entry) => {
    if (entry.level === "error" || entry.level === "warn") {
      scanWarnErrorCount.update(n => n + 1);
    }
    scanLogs.update((logs) => {
      const next = [...logs, entry];
      return next.length > 5000 ? next.slice(-5000) : next;
    });
  };

  const treeChannel = new Channel<FileNode>();
  treeChannel.onmessage = (partialTree) => {
    tree.set(partialTree);
  };

  // Build scan options from settings store
  const currentSettings = get(settings);
  const scanOptions: ScanOptions = {
    excludePaths: currentSettings.general.excludePaths,
    excludeNames: currentSettings.general.excludeNames,
    followSymlinks: currentSettings.general.followSymlinks,
    maxChildrenPerDir: currentSettings.general.maxChildrenPerDir,
  };

  try {
    const result = await scanDirectory(path, progressChannel, logChannel, treeChannel, scanOptions);
    _dirSizesMap = new Map();
    dirSizes.set(_dirSizesMap); // 최종 트리는 정확한 크기를 가짐
    tree.set(result);
  } catch (e) {
    if (!get(scanCancelled)) {
      scanError.set(String(e));
    }
  } finally {
    scanning.set(false);
    scanningDirs.set(new Set());
  }
}

export async function stopScan() {
  scanCancelled.set(true);
  await cancelScan();
}

/**
 * Re-scan only the subtree at `path` and merge the result back into the
 * existing tree.  Uses `partialScanning` so the main UI stays interactive.
 */
export async function refreshSubtree(path: string) {
  let currentTree: FileNode | null = null;
  tree.subscribe((t) => (currentTree = t))();
  if (!currentTree) return;

  partialScanning.set(true);

  const progressChannel = new Channel<ScanProgress>();
  progressChannel.onmessage = () => {};          // ignore progress for partial scan

  const logChannel = new Channel<ScanLogEntry>();
  logChannel.onmessage = (entry) => {
    scanLogs.update((logs) => {
      const next = [...logs, entry];
      return next.length > 5000 ? next.slice(-5000) : next;
    });
  };

  const treeChannel = new Channel<FileNode>();
  treeChannel.onmessage = () => {};               // ignore intermediate trees

  const subSettings = get(settings);
  const subScanOptions: ScanOptions = {
    excludePaths: subSettings.general.excludePaths,
    excludeNames: subSettings.general.excludeNames,
    followSymlinks: subSettings.general.followSymlinks,
    maxChildrenPerDir: subSettings.general.maxChildrenPerDir,
  };

  try {
    const subtreeResult = await scanSubdirectory(path, progressChannel, logChannel, treeChannel, subScanOptions);
    if (subtreeResult) {
      tree.update((t) => {
        if (!t) return t;
        return replaceSubtree(t, path, subtreeResult);
      });
    }
  } catch (e) {
    scanError.set(String(e));
  } finally {
    partialScanning.set(false);
  }
}
