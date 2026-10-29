import { writable, derived, get } from "svelte/store";
import { Channel } from "@tauri-apps/api/core";
import type { CleanupMethod, FileNode, ScanLogEntry, ScanProgress, VolumeInfo } from "../types";
import { scanDirectory, refreshSubtree as refreshSubtreeApi, cancelScan, getSystemVolumes, getNode, type ScanOptions } from "../api";
import { clearHistory } from "./navigationStore";
import { settings } from "./settingsStore";

const INITIAL_PROGRESS: ScanProgress = {
  phase: "structure",
  files_scanned: 0,
  current_path: "",
  total_dirs: 0,
  completed_dirs: 0,
  total_bytes: 0,
  tree_version: 0,
};

/** 쿼리 결과가 무거운 뷰(트리맵, 확장자 통계)의 스캔 중 갱신 주기 */
const SLOW_REFRESH_MS = 3000;

/**
 * 스캔 루트 노드 요약. 전체 트리는 백엔드에 있으며 컴포넌트는 필요한 부분만 조회한다.
 */
export const tree = writable<FileNode | null>(null);
/** 루트 경로 (값이 바뀔 때만 알림) */
export const treeRootPath = derived(tree, (t) => t?.path ?? null);
/** 백엔드 트리가 바뀔 때마다 증가. 컴포넌트는 이 값이 바뀌면 자신이 보여주는 부분을 다시 조회한다. */
export const treeVersion = writable(0);
/** 스캔 중에는 SLOW_REFRESH_MS 간격으로만 증가하는 treeVersion */
export const slowTreeVersion = writable(0);

export const scanning = writable(false);
export const progress = writable<ScanProgress>(INITIAL_PROGRESS);
export const scanError = writable<string | null>(null);
export const scanCancelled = writable(false);
export const scanLogs = writable<ScanLogEntry[]>([]);
export const scanningDirs = writable<Set<string>>(new Set());
export const scanWarnErrorCount = writable(0);

/** True while a partial subtree refresh is in progress (does not block full UI). */
export const partialScanning = writable(false);

/** Volume info for the currently scanned path. */
export const currentVolume = writable<VolumeInfo | null>(null);

let lastSlowUpdate = 0;
let slowTimer: ReturnType<typeof setTimeout> | null = null;

treeVersion.subscribe((v) => {
  if (!get(scanning)) {
    if (slowTimer) { clearTimeout(slowTimer); slowTimer = null; }
    lastSlowUpdate = Date.now();
    slowTreeVersion.set(v);
    return;
  }
  if (slowTimer) return;
  const wait = Math.max(0, SLOW_REFRESH_MS - (Date.now() - lastSlowUpdate));
  slowTimer = setTimeout(() => {
    slowTimer = null;
    lastSlowUpdate = Date.now();
    slowTreeVersion.set(get(treeVersion));
  }, wait);
});

/** 현재 스캔 세션 식별자. 늦게 도착한 이전 스캔의 응답을 무시하는 데 사용한다. */
let scanGeneration = 0;
let rootPath: string | null = null;
let reloading = false;
let reloadPending = false;

function bumpVersion() {
  treeVersion.update((v) => v + 1);
}

/**
 * 백엔드 트리 변경 후 루트 요약을 다시 읽고 뷰들에 갱신을 알린다.
 * 동시에 여러 번 호출되면 하나로 합친다.
 */
export async function reloadTree(): Promise<void> {
  if (!rootPath) return;
  if (reloading) {
    reloadPending = true;
    return;
  }
  reloading = true;
  const gen = scanGeneration;
  try {
    const root = await getNode(rootPath);
    if (gen !== scanGeneration) return;
    tree.set(root);
    bumpVersion();
  } catch (e) {
    console.warn("Failed to reload tree:", e);
  } finally {
    reloading = false;
    if (reloadPending) {
      reloadPending = false;
      void reloadTree();
    }
  }
}

function currentScanOptions(): ScanOptions {
  const general = get(settings).general;
  return {
    excludePaths: general.excludePaths,
    excludeNames: general.excludeNames,
    followSymlinks: general.followSymlinks,
    maxChildrenPerDir: general.maxChildrenPerDir,
  };
}

function appendLog(entry: ScanLogEntry) {
  scanLogs.update((logs) => {
    const next = [...logs, entry];
    return next.length > 5000 ? next.slice(-5000) : next;
  });
}

export async function startScan(path: string) {
  const gen = ++scanGeneration;
  rootPath = path;
  tree.set(null);
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

  let lastTreeVersion = 0;
  const progressChannel = new Channel<ScanProgress>();
  progressChannel.onmessage = (msg) => {
    if (gen !== scanGeneration) return;
    progress.set(msg);
    if (msg.tree_version > lastTreeVersion) {
      lastTreeVersion = msg.tree_version;
      void reloadTree();
    }
  };

  const logChannel = new Channel<ScanLogEntry>();
  logChannel.onmessage = (entry) => {
    if (entry.level === "error" || entry.level === "warn") {
      scanWarnErrorCount.update(n => n + 1);
    }
    appendLog(entry);
  };

  let result: FileNode | null = null;
  try {
    result = await scanDirectory(path, progressChannel, logChannel, currentScanOptions());
  } catch (e) {
    if (!get(scanCancelled)) {
      scanError.set(String(e));
    }
  } finally {
    if (gen === scanGeneration) {
      scanning.set(false);
      scanningDirs.set(new Set());
      tree.set(result);
      bumpVersion();
    }
  }
}

export async function stopScan() {
  scanCancelled.set(true);
  await cancelScan();
}

/**
 * Re-scan only the subtree at `path` and merge the result back into the
 * backend tree.  Uses `partialScanning` so the main UI stays interactive.
 */
export async function refreshSubtree(path: string) {
  if (!get(tree)) return;

  partialScanning.set(true);

  const progressChannel = new Channel<ScanProgress>();
  progressChannel.onmessage = () => {};          // ignore progress for partial scan

  const logChannel = new Channel<ScanLogEntry>();
  logChannel.onmessage = appendLog;

  const gen = scanGeneration;
  try {
    const root = await refreshSubtreeApi(path, progressChannel, logChannel, currentScanOptions());
    if (root && gen === scanGeneration) {
      tree.set(root);
      bumpVersion();
    }
  } catch (e) {
    scanError.set(String(e));
  } finally {
    partialScanning.set(false);
  }
}

/** Reflect a finished cleanup recommendation in the tree. */
export async function syncTreeAfterCleanup(paths: string[], method: CleanupMethod) {
  if (paths.length === 0) return;

  if (method.type === "Delete") {
    // Backend already removed the deleted paths from its tree
    await reloadTree();
  } else if (method.type === "ShellCommand") {
    // Re-scan affected paths' parents for accurate sizes
    for (const p of paths) {
      const parentPath = p.substring(0, p.lastIndexOf("/")) || "/";
      if (get(tree) && (await getNode(parentPath))) {
        void refreshSubtree(parentPath);
      }
    }
  }
}
