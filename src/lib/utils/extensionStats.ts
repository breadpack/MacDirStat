import type { FileNode } from "../types";

export interface ExtensionStat {
  extension: string;      // lowercase, files without extension use "(no ext)"
  totalBytes: number;
  fileCount: number;
  percentage: number;     // percentage based on totalBytes
  color: string;          // dynamically assigned color
}

/**
 * Walk the file tree and aggregate stats per extension.
 * Returns stats sorted by totalBytes descending.
 */
export function computeExtensionStats(root: FileNode): ExtensionStat[] {
  const map = new Map<string, { bytes: number; count: number }>();

  const stack: FileNode[] = [root];
  while (stack.length > 0) {
    const node = stack.pop()!;
    if (!node.is_dir) {
      const ext = node.extension?.toLowerCase() ?? "(no ext)";
      const entry = map.get(ext);
      if (entry) {
        entry.bytes += node.size;
        entry.count += 1;
      } else {
        map.set(ext, { bytes: node.size, count: 1 });
      }
    }
    for (const child of node.children) {
      stack.push(child);
    }
  }

  // Convert to array and sort by bytes descending
  const totalBytes = Array.from(map.values()).reduce((s, e) => s + e.bytes, 0);
  const stats: ExtensionStat[] = [];

  for (const [ext, { bytes, count }] of map) {
    stats.push({
      extension: ext,
      totalBytes: bytes,
      fileCount: count,
      percentage: totalBytes > 0 ? (bytes / totalBytes) * 100 : 0,
      color: "", // will be assigned by buildColorMap
    });
  }

  stats.sort((a, b) => b.totalBytes - a.totalBytes);
  return stats;
}
