/**
 * Settings type definitions and defaults for MacDirStat.
 */

export interface GeneralSettings {
  followSymlinks: boolean;
  excludePaths: string[];
  excludeNames: string[];
  excludePatterns: string[];
  maxChildrenPerDir: number;
}

export interface TreeViewSettings {
  showColumns: Record<string, boolean>;
  sortBy: string;
  sortDesc: boolean;
  showHiddenFiles: boolean;
}

export interface TreemapSettings {
  style: "squarify";
  padding: number;
  showLabels: boolean;
  colorPalette: string;
}

export interface AppearanceSettings {
  theme: "dark" | "light" | "system";
  treePanelWidth: number;
}

export interface AppSettings {
  general: GeneralSettings;
  treeView: TreeViewSettings;
  treemap: TreemapSettings;
  appearance: AppearanceSettings;
}

export const DEFAULT_EXCLUDE_PATHS = [
  "/dev",
  "/proc",
  "/sys",
  "/Volumes",
  "/.vol",
  "/net",
  "/home",
  "/Network",
  "/System/Volumes",
  "/private/var/folders",
  "/private/var/db",
  "/private/var/protected",
  "/private/var/audit",
  "/.Spotlight-V100",
  "/.fseventsd",
  "/.Trashes",
];

export const DEFAULT_EXCLUDE_NAMES = [
  ".Spotlight-V100",
  ".fseventsd",
  ".Trashes",
  ".DocumentRevisions-V100",
];

export const DEFAULT_SETTINGS: AppSettings = {
  general: {
    followSymlinks: false,
    excludePaths: [...DEFAULT_EXCLUDE_PATHS],
    excludeNames: [...DEFAULT_EXCLUDE_NAMES],
    excludePatterns: [],
    maxChildrenPerDir: 200,
  },
  treeView: {
    showColumns: {
      name: true,
      size: true,
      percent: true,
      files: true,
      subdirs: true,
      modified: true,
      attributes: true,
    },
    sortBy: "size",
    sortDesc: true,
    showHiddenFiles: false,
  },
  treemap: {
    style: "squarify",
    padding: 1,
    showLabels: true,
    colorPalette: "default",
  },
  appearance: {
    theme: "dark",
    treePanelWidth: 35,
  },
};
