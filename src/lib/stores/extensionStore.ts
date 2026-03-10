import { writable, derived } from "svelte/store";
import { tree } from "./scanStore";
import { zoomRoot } from "./selectionStore";
import type { FileNode } from "../types";
import { computeExtensionStats, type ExtensionStat } from "../utils/extensionStats";
import { buildColorMap } from "../utils/colorMap";

/** Whether the extension panel is visible. */
export const showExtensionPanel = writable<boolean>(false);

/** Currently highlighted extension (clicked in extension list). */
export const highlightedExtension = writable<string | null>(null);

/** Find a node by path in the tree with prefix pruning. */
function findNodeByPath(node: FileNode, targetPath: string): FileNode | null {
  if (node.path === targetPath) return node;
  if (!node.is_dir) return null;
  const prefix = node.path.endsWith("/") ? node.path : node.path + "/";
  if (!targetPath.startsWith(prefix)) return null;
  for (const child of node.children) {
    const found = findNodeByPath(child, targetPath);
    if (found) return found;
  }
  return null;
}

/** Extension stats derived from current tree/zoomRoot, gated on panel visibility. */
export const extensionStats = derived(
  [tree, zoomRoot, showExtensionPanel],
  ([$tree, $zoomRoot, $show]) => {
    if (!$show || !$tree) return [];
    let root = $tree;
    if ($zoomRoot) {
      const found = findNodeByPath($tree, $zoomRoot);
      if (found) root = found;
    }
    return computeExtensionStats(root);
  },
);

/** Dynamic color map derived from extension stats (pure, no side-effects). */
export const extensionColorMap = derived(
  extensionStats,
  ($stats) => {
    return buildColorMap($stats);
  },
);
