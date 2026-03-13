/**
 * Shared action functions that can be invoked from keyboard shortcuts,
 * toolbar buttons, or context menus.
 */

import { open, confirm, message } from "@tauri-apps/plugin-dialog";
import { get } from "svelte/store";
import { tree, scanning, startScan, currentVolume } from "../stores/scanStore";
import { selectedPath, zoomRoot, showFreeSpace, showUnknown } from "../stores/selectionStore";
import { moveToTrash, showGetInfo } from "../api";
import { removeNode } from "./treeUtils";

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
 * Removes the node from the tree on success.
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
    await moveToTrash(sp);
    tree.update((t) => {
      if (!t) return t;
      return removeNode(t, sp) ?? t;
    });
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
