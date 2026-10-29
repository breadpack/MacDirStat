import type { FileNode } from "../types";
import type { SortState } from "../stores/columnStore";

export function compareNodes(a: FileNode, b: FileNode, sort: SortState): number {
  // 디렉토리를 항상 파일보다 먼저 표시
  if (a.is_dir !== b.is_dir) {
    return a.is_dir ? -1 : 1;
  }

  let result = 0;
  switch (sort.columnId) {
    case "name":
      result = a.name.toLowerCase().localeCompare(b.name.toLowerCase());
      break;
    case "size":
      result = a.size - b.size;
      break;
    case "files":
      result = a.file_count - b.file_count;
      break;
    case "subdirs":
      result = a.dir_count - b.dir_count;
      break;
    case "modified":
      result = (a.modified ?? 0) - (b.modified ?? 0);
      break;
    default:
      result = a.size - b.size;
  }
  return sort.direction === "desc" ? -result : result;
}

function quickSelectInPlace<T>(arr: T[], lo: number, hi: number, k: number, compare: (a: T, b: T) => number): void {
  while (lo < hi) {
    const pivotIdx = lo + ((hi - lo) >> 1);
    const pivot = arr[pivotIdx];
    // Move pivot to end
    [arr[pivotIdx], arr[hi]] = [arr[hi], arr[pivotIdx]];
    let storeIdx = lo;
    for (let i = lo; i < hi; i++) {
      if (compare(arr[i], pivot) < 0) {
        [arr[storeIdx], arr[i]] = [arr[i], arr[storeIdx]];
        storeIdx++;
      }
    }
    [arr[storeIdx], arr[hi]] = [arr[hi], arr[storeIdx]];
    if (storeIdx === k) return;
    if (storeIdx < k) lo = storeIdx + 1;
    else hi = storeIdx - 1;
  }
}

function quickSelectTop<T>(arr: T[], k: number, compare: (a: T, b: T) => number): T[] {
  if (k >= arr.length) return arr;
  const work = arr.slice();
  quickSelectInPlace(work, 0, work.length - 1, k, compare);
  return work.slice(0, k);
}

/**
 * Partial sort: 대량의 children에서 상위 N개만 정렬하여 반환.
 * 전체 정렬(O(n log n)) 대신 부분 선택(O(n + k log k)) 사용.
 */
export function topNChildren(
  children: FileNode[],
  n: number,
  sort: SortState,
): FileNode[] {
  if (children.length <= n) {
    return [...children].sort((a, b) => compareNodes(a, b, sort));
  }
  // 디렉토리 우선 분리 (소수이므로 빠름)
  const dirs: FileNode[] = [];
  const files: FileNode[] = [];
  for (const c of children) {
    (c.is_dir ? dirs : files).push(c);
  }
  // 각 그룹을 정렬 (디렉토리는 보통 소수)
  dirs.sort((a, b) => compareNodes(a, b, sort));
  const remaining = n - dirs.length;
  if (remaining <= 0) {
    return dirs.slice(0, n);
  }
  if (files.length <= remaining) {
    // All files fit, no truncation needed
    files.sort((a, b) => compareNodes(a, b, sort));
    return [...dirs, ...files];
  }
  if (files.length <= remaining * 3) {
    // Small enough, full sort is fine
    files.sort((a, b) => compareNodes(a, b, sort));
    return [...dirs, ...files.slice(0, remaining)];
  }
  // For large arrays, use partial sort via quickselect
  const cmp = (a: FileNode, b: FileNode) => compareNodes(a, b, sort);
  const topFiles = quickSelectTop(files, remaining, cmp);
  topFiles.sort(cmp); // sort only the top N
  return [...dirs, ...topFiles];
}
