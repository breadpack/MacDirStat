import { writable, derived } from "svelte/store";
import { tree } from "./scanStore";
import { zoomRoot } from "./selectionStore";
import { computeExtensionStats, type ExtensionStat } from "../utils/extensionStats";
import { buildColorMap, setActiveColorMap } from "../utils/colorMap";
import { findNode } from "../utils/treeUtils";

/** Whether the extension panel is visible. */
export const showExtensionPanel = writable<boolean>(false);

/** Currently highlighted extension (clicked in extension list). */
export const highlightedExtension = writable<string | null>(null);

/** Extension stats derived from current tree/zoomRoot. */
export const extensionStats = derived(
  [tree, zoomRoot, showExtensionPanel],
  ([$tree, $zoomRoot, $show]) => {
    if (!$show || !$tree) return [];
    let root = $tree;
    if ($zoomRoot) {
      const found = findNode($tree, $zoomRoot);
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
