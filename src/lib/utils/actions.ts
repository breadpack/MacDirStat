/**
 * Shared action functions that can be invoked from keyboard shortcuts,
 * toolbar buttons, or context menus.
 */

import { open, confirm, message } from "@tauri-apps/plugin-dialog";
import { get } from "svelte/store";
import { tree, scanning, startScan, currentVolume, reloadTree, refreshSubtree } from "../stores/scanStore";
import { cleanupActions, runCleanup } from "../stores/cleanupStore";
import { selectedPath, zoomRoot, showFreeSpace, showUnknown } from "../stores/selectionStore";
import { moveToTrash, showGetInfo, getNode } from "../api";

/**
 * Open a folder dialog and start scanning.
 */
export async function actionOpenFolder(): Promise<void> {
  if (get(scanning)) return;
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    zoomRoot.set(null);
    await startScan(selected as string);
  }
}

/**
 * Refresh (re-scan) the current tree root.
 */
export function actionRefresh(): void {
  if (get(scanning)) return;
  const rootPath = get(tree)?.path;
  if (rootPath) {
    zoomRoot.set(null);
    startScan(rootPath);
  }
}

/**
 * Copy the currently selected path to the clipboard.
 */
export async function actionCopyPath(): Promise<void> {
  const sp = get(selectedPath);
  if (!sp) return;
  try {
    await navigator.clipboard.writeText(sp);
  } catch {
    // Clipboard write may fail in some contexts
  }
}

/**
 * Move the currently selected item to Trash.
 * The backend removes the node from the scan tree on success.
 */
export async function actionMoveToTrash(): Promise<void> {
  const sp = get(selectedPath);
  if (!sp) return;
  const t = get(tree);
  if (!t) return;
  // Don't allow deleting root
  if (sp === t.path) return;

  const name = sp.split("/").pop() ?? sp;
  const ok = await confirm(`Move "${name}" to Trash?`);
  if (!ok) return;

  try {
    await moveToTrash(sp); // backend also removes the node from its tree
    await reloadTree();
  } catch (e) {
    console.error("Failed to move to trash:", e);
    await message(`Failed to move to trash: ${e}`, { kind: "error" });
  }
}

/**
 * Show macOS Get Info for the selected path.
 */
export async function actionShowInfo(): Promise<void> {
  const sp = get(selectedPath);
  if (!sp) return;
  try {
    await showGetInfo(sp);
  } catch (e) {
    console.error("Failed to show Get Info:", e);
  }
}

/**
 * Toggle free space display (F6).
 */
export function actionToggleFreeSpace(): void {
  const t = get(tree);
  if (!t) return;
  if (!get(currentVolume) || get(zoomRoot)) return;
  showFreeSpace.update((v) => !v);
}

/**
 * Toggle unknown space display (F7).
 */
export function actionToggleUnknown(): void {
  const t = get(tree);
  if (!t) return;
  if (!get(currentVolume) || get(zoomRoot)) return;
  showUnknown.update((v) => !v);
}

/**
 * Run the user-defined cleanup action in `slotId` on the selected item (Cmd+Shift+0~9).
 */
export async function actionRunCleanupSlot(slotId: number): Promise<void> {
  const action = get(cleanupActions).find((a) => a.id === slotId);
  if (!action || !action.enabled || !action.name.trim() || !action.command.trim()) return;

  const currentPath = get(selectedPath);
  if (!currentPath) return;

  if (!get(tree)) return;

  const node = await getNode(currentPath);
  if (!node) return;

  // Check target compatibility
  if (action.target === "Files" && node.is_dir) return;
  if (action.target === "Dirs" && !node.is_dir) return;

  if (action.confirm) {
    const ok = await confirm(`Run "${action.name}" on "${node.name}"?`, {
      title: "Run Cleanup",
      kind: "info",
      okLabel: "Run",
      cancelLabel: "Cancel",
    });
    if (!ok) return;
  }

  try {
    const result = await runCleanup(action.id, node.path, node.name);
    if (!action.run_in_terminal && result) {
      console.log(`Cleanup "${action.name}" result:`, result);
    }
    if (action.refresh_after && node.is_dir) {
      await refreshSubtree(node.path);
    }
  } catch (e) {
    console.error(`Cleanup "${action.name}" failed:`, e);
    await message(`Cleanup failed: ${e}`, { kind: "error" });
  }
}
