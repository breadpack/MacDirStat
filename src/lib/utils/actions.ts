/**
 * Shared action functions that can be invoked from keyboard shortcuts,
 * toolbar buttons, or context menus.
 */

import { open } from "@tauri-apps/plugin-dialog";
import { get } from "svelte/store";
import { tree, scanning, startScan, currentVolume } from "../stores/scanStore";
import { selectedPath, zoomRoot, showFreeSpace, showUnknown } from "../stores/selectionStore";
import { moveToTrash, showGetInfo } from "../api";
import type { FileNode } from "../types";

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
  const ok = window.confirm(`Move "${name}" to Trash?`);
  if (!ok) return;

  try {
    await moveToTrash(sp);
    tree.update((t) => {
      if (!t) return t;
      return removeNode(t, sp) ?? t;
    });
  } catch (e) {
    console.error("Failed to move to trash:", e);
    window.alert(`Failed to move to trash: ${e}`);
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

// ---- internal helpers ----

/**
 * Immutably remove a node from the tree, returning a new tree with updated
 * sizes/counts. Returns null if the target was not found.
 */
function removeNode(node: FileNode, targetPath: string): FileNode | null {
  const idx = node.children.findIndex((c) => c.path === targetPath);
  if (idx >= 0) {
    const removed = node.children[idx];
    const newChildren = node.children.filter((c) => c.path !== targetPath);
    return {
      ...node,
      children: newChildren,
      size: node.size - removed.size,
      file_count: node.file_count - (removed.is_dir ? removed.file_count : 1),
      dir_count: node.dir_count - (removed.is_dir ? 1 + removed.dir_count : 0),
    };
  }
  for (let i = 0; i < node.children.length; i++) {
    const child = node.children[i];
    if (!child.children) continue;
    const updated = removeNode(child, targetPath);
    if (updated) {
      const sizeDiff = child.size - updated.size;
      const fileDiff = child.file_count - updated.file_count;
      const dirDiff = child.dir_count - updated.dir_count;
      const newChildren = [...node.children];
      newChildren[i] = updated;
      return {
        ...node,
        children: newChildren,
        size: node.size - sizeDiff,
        file_count: node.file_count - fileDiff,
        dir_count: node.dir_count - dirDiff,
      };
    }
  }
  return null;
}
