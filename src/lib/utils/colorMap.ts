import type { ExtensionStat } from "./extensionStats";

// 12 high-saturation colors for top extensions (HSL-based, evenly distributed)
export const DYNAMIC_PALETTE = [
  "#4A90D9", "#D94A4A", "#4AD94A", "#D9944A",
  "#944AD9", "#4AD9D9", "#D9D94A", "#D94A94",
  "#4A6BD9", "#D96B4A", "#6BD94A", "#D94AD9",
];

export const GRAY_COLOR = "#666666";

/** Currently active dynamic color map. null = use static EXTENSION_COLORS. */
let activeColorMap: Map<string, string> | null = null;

/** Cache for RGB color conversions, cleared when active color map changes. */
const rgbCache = new Map<string, { r: number; g: number; b: number }>();

/**
 * Build a dynamic color map from extension stats.
 * Top 12 extensions get palette colors, rest get gray.
 */
export function buildColorMap(stats: ExtensionStat[]): Map<string, string> {
  const map = new Map<string, string>();
  for (let i = 0; i < stats.length; i++) {
    map.set(stats[i].extension, i < DYNAMIC_PALETTE.length ? DYNAMIC_PALETTE[i] : GRAY_COLOR);
  }
  return map;
}

/** Set the active dynamic color map (or null to use static colors). */
export function setActiveColorMap(map: Map<string, string> | null) {
  activeColorMap = map;
  rgbCache.clear();
}

/** Get the currently active color map. */
export function getActiveColorMap(): Map<string, string> | null {
  return activeColorMap;
}

const EXTENSION_COLORS: Record<string, string> = {
  // Images - blue
  jpg: "#4A90D9", jpeg: "#4A90D9", png: "#5B9BD5", gif: "#6BA5DA",
  bmp: "#4A90D9", svg: "#5B9BD5", webp: "#6BA5DA", ico: "#4A90D9",
  tiff: "#5B9BD5", raw: "#4A90D9", heic: "#6BA5DA",

  // Video - red
  mp4: "#D94A4A", mkv: "#D95B5B", avi: "#DA6B6B", mov: "#D94A4A",
  wmv: "#D95B5B", flv: "#DA6B6B", webm: "#D94A4A", m4v: "#D95B5B",

  // Audio - orange
  mp3: "#D9944A", wav: "#D9A05B", flac: "#DAA96B", aac: "#D9944A",
  ogg: "#D9A05B", wma: "#DAA96B", m4a: "#D9944A",

  // Code - green
  ts: "#4AD94A", js: "#5BD95B", tsx: "#6BDA6B", jsx: "#4AD94A",
  py: "#5BD95B", rs: "#6BDA6B", go: "#4AD94A", java: "#5BD95B",
  c: "#6BDA6B", cpp: "#4AD94A", h: "#5BD95B", cs: "#6BDA6B",
  rb: "#4AD94A", php: "#5BD95B", swift: "#6BDA6B", kt: "#4AD94A",
  svelte: "#5BD95B", vue: "#6BDA6B", html: "#4AD94A", css: "#5BD95B",
  scss: "#6BDA6B", less: "#4AD94A",

  // Archives - purple
  zip: "#944AD9", tar: "#A05BD9", gz: "#A96BDA", rar: "#944AD9",
  "7z": "#A05BD9", bz2: "#A96BDA", xz: "#944AD9", dmg: "#A05BD9",
  iso: "#A96BDA",

  // Documents - teal
  pdf: "#4AD9D9", doc: "#5BD9D9", docx: "#6BDADA", xls: "#4AD9D9",
  xlsx: "#5BD9D9", ppt: "#6BDADA", pptx: "#4AD9D9", txt: "#5BD9D9",
  md: "#6BDADA", rtf: "#4AD9D9", csv: "#5BD9D9",

  // Data/Config - yellow
  json: "#D9D94A", yaml: "#D9D95B", yml: "#DAD96B", xml: "#D9D94A",
  toml: "#D9D95B", ini: "#DAD96B", env: "#D9D94A",
};

// Special virtual node colors
const SPECIAL_COLORS: Record<string, string> = {
  "__freespace__": "#2a2a2a",   // dark gray (free space)
  "__unknown__":   "#3a2a1a",   // dark brown (unknown space)
};

const DIR_COLOR = "#555555";
const DEFAULT_COLOR = "#888888";

export function getColor(extension: string | null, isDir: boolean): string {
  if (isDir) return DIR_COLOR;
  if (!extension) {
    // Check dynamic map for "(no ext)"
    if (activeColorMap) {
      const dynamicColor = activeColorMap.get("(no ext)");
      if (dynamicColor) return dynamicColor;
    }
    return DEFAULT_COLOR;
  }
  const special = SPECIAL_COLORS[extension];
  if (special) return special;
  // Use dynamic color map if available
  if (activeColorMap) {
    const dynamicColor = activeColorMap.get(extension.toLowerCase());
    if (dynamicColor) return dynamicColor;
    return GRAY_COLOR;
  }
  return EXTENSION_COLORS[extension.toLowerCase()] ?? DEFAULT_COLOR;
}

export interface RGB {
  r: number;
  g: number;
  b: number;
}

export function getColorRGB(extension: string | null, isDir: boolean): RGB {
  const key = `${extension ?? ""}|${isDir ? "1" : "0"}`;
  let cached = rgbCache.get(key);
  if (cached) return cached;
  const hex = getColor(extension, isDir);
  cached = {
    r: parseInt(hex.slice(1, 3), 16),
    g: parseInt(hex.slice(3, 5), 16),
    b: parseInt(hex.slice(5, 7), 16),
  };
  rgbCache.set(key, cached);
  return cached;
}
