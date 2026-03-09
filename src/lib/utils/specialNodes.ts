import type { FileNode, VolumeInfo } from "../types";

/** Special path constants for virtual nodes. */
export const FREESPACE_PATH = "/__freespace__";
export const UNKNOWN_PATH = "/__unknown__";

/** Synthetic path suffixes used by treemap pruning and tree truncation. */
export const OTHERS_SUFFIX = "/__others__";
export const TRUNCATED_SUFFIX = "/__truncated__";

/** Check whether a path belongs to a virtual (special) node. */
export function isSpecialPath(path: string): boolean {
  return path === FREESPACE_PATH || path === UNKNOWN_PATH
    || path.endsWith(OTHERS_SUFFIX) || path.endsWith(TRUNCATED_SUFFIX);
}

/**
 * Inject virtual Free Space and/or Unknown Space nodes into the root.
 * Returns a shallow copy of root with adjusted size and extra children.
 * Only meaningful when viewing the scan root (not a zoomed subdirectory).
 */
export function injectSpecialNodes(
  root: FileNode,
  volume: VolumeInfo | null,
  showFree: boolean,
  showUnk: boolean,
): FileNode {
  if (!volume) return root;
  if (!showFree && !showUnk) return root;

  const scannedSize = root.size;
  const totalSize = volume.total_bytes;
  const freeSize = volume.available_bytes;
  const unknownSize = Math.max(0, totalSize - scannedSize - freeSize);

  const extraChildren: FileNode[] = [];

  if (showFree && freeSize > 0) {
    extraChildren.push({
      name: "<Free Space>",
      path: FREESPACE_PATH,
      size: freeSize,
      is_dir: false,
      children: [],
      file_count: 0,
      dir_count: 0,
      modified: null,
      is_symlink: false,
      is_hidden: false,
      is_readonly: false,
      extension: "__freespace__",
    });
  }

  if (showUnk && unknownSize > 0) {
    extraChildren.push({
      name: "<Unknown>",
      path: UNKNOWN_PATH,
      size: unknownSize,
      is_dir: false,
      children: [],
      file_count: 0,
      dir_count: 0,
      modified: null,
      is_symlink: false,
      is_hidden: false,
      is_readonly: false,
      extension: "__unknown__",
    });
  }

  if (extraChildren.length === 0) return root;

  return {
    ...root,
    size: scannedSize + extraChildren.reduce((s, c) => s + c.size, 0),
    children: [...root.children, ...extraChildren],
  };
}
