import { writable, derived } from "svelte/store";
import { treeRootPath, slowTreeVersion } from "./scanStore";
import { zoomRoot } from "./selectionStore";
import { toExtensionStats, type ExtensionStat } from "../utils/extensionStats";
import { buildColorMap } from "../utils/colorMap";
import { getExtensionStats } from "../api";

/** Whether the extension panel is visible. */
export const showExtensionPanel = writable<boolean>(false);

/** Currently highlighted extension (clicked in extension list). */
export const highlightedExtension = writable<string | null>(null);

let statsRequest = 0;

/** Extension stats for the current tree/zoomRoot, queried from the backend. */
export const extensionStats = derived<[typeof treeRootPath, typeof zoomRoot, typeof showExtensionPanel, typeof slowTreeVersion], ExtensionStat[]>(
  [treeRootPath, zoomRoot, showExtensionPanel, slowTreeVersion],
  ([$rootPath, $zoomRoot, $show], set) => {
    if (!$show || !$rootPath) {
      set([]);
      return;
    }
    const req = ++statsRequest;
    getExtensionStats($zoomRoot ?? $rootPath)
      .then((raw) => {
        if (req === statsRequest) set(toExtensionStats(raw));
      })
      .catch((e) => console.warn("Failed to load extension stats:", e));
  },
  [],
);

/** Dynamic color map derived from extension stats (pure, no side-effects). */
export const extensionColorMap = derived(
  extensionStats,
  ($stats) => {
    return buildColorMap($stats);
  },
);
