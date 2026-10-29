import type { RawExtensionStat } from "../types";

export interface ExtensionStat {
  extension: string;      // lowercase, files without extension use "(no ext)"
  totalBytes: number;
  fileCount: number;
  percentage: number;     // percentage based on totalBytes
  color: string;          // dynamically assigned color
}

/**
 * Convert backend per-extension aggregates (already sorted by bytes desc)
 * into display stats with percentages.
 */
export function toExtensionStats(raw: RawExtensionStat[]): ExtensionStat[] {
  const totalBytes = raw.reduce((s, e) => s + e.total_bytes, 0);
  return raw.map((e) => ({
    extension: e.extension,
    totalBytes: e.total_bytes,
    fileCount: e.file_count,
    percentage: totalBytes > 0 ? (e.total_bytes / totalBytes) * 100 : 0,
    color: "", // will be assigned by buildColorMap
  }));
}
