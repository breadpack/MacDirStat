import { writable, derived } from "svelte/store";
import { tree } from "./scanStore";
import { zoomRoot } from "./selectionStore";
import type { FileNode } from "../types";
import { computeExtensionStats, type ExtensionStat } from "../utils/extensionStats";
import { buildColorMap, setActiveColorMap } from "../utils/colorMap";

/** Whether the extension panel is visible. */
export const showExtensionPanel = writable<boolean>(false);

/** Currently highlighted extension (clicked in extension list). */
export const highlightedExtension = writable<string | null>(null);

/** Find a node by path in the tree. */
function findNodeByPath(node: FileNode, path: string): FileNode | null {
  if (node.path === path) return node;
  for (const child of node.children) {
    const found = findNodeByPath(child, path);
    if (found) return found;
  }
  return null;
}

/** Extension stats derived from current tree/zoomRoot. */
export const extensionStats = derived(
  [tree, zoomRoot],
  ([$tree, $zoomRoot]) => {
    if (!$tree) return [];
    let root = $tree;
    if ($zoomRoot) {
      const found = findNodeByPath($tree, $zoomRoot);
      if (found) root = found;
    }
    return computeExtensionStats(root);
  },
);

/** Dynamic color map derived from extension stats. */
export const extensionColorMap = derived(
  extensionStats,
  ($stats) => {
    const map = buildColorMap($stats);
    // Also apply colors to the stats themselves
    setActiveColorMap(map);
    return map;
  },
);
