import { writable, get } from "svelte/store";

// Each directory path -> last visited child path
const childHistory = writable<Map<string, string>>(new Map());

export function getParentPath(path: string): string | null {
  const lastSlash = path.lastIndexOf("/");
  if (lastSlash <= 0) return null;
  return path.substring(0, lastSlash);
}

export function recordChildVisit(parentPath: string, childPath: string): void {
  childHistory.update((map) => {
    if (map.get(parentPath) === childPath) return map; // same value, no notification
    const next = new Map(map);
    next.set(parentPath, childPath);
    return next;
  });
}

export function getLastChild(parentPath: string): string | null {
  const map = get(childHistory);
  return map.get(parentPath) ?? null;
}

export function clearHistory(): void {
  childHistory.set(new Map());
}
