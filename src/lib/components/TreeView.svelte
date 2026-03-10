<script lang="ts">
  import { untrack } from "svelte";
  import type { FileNode, FlatTreeRow } from "../types";
  import { tree, progress, dirSizes, scanning } from "../stores/scanStore";
  import { selectedPath } from "../stores/selectionStore";
  import { sortState, type SortState } from "../stores/columnStore";
  import { getParentPath, recordChildVisit, getLastChild } from "../stores/navigationStore";
  import { isSpecialPath } from "../utils/specialNodes";
  import TreeRow from "./TreeRow.svelte";
  import TreeColumnHeader from "./TreeColumnHeader.svelte";
  import ContextMenu from "./ContextMenu.svelte";

  // dirSizes를 3초마다 스냅샷 → 정렬용 (너무 자주 재정렬하면 스크롤이 깨짐)
  let sortSizes = $state<Map<string, { size: number; file_count: number }>>(new Map());

  $effect(() => {
    if (!$scanning) {
      // When not scanning, sync once and stop
      const current = $dirSizes;
      if (current.size > 0) {
        sortSizes = new Map(current);
      } else if (sortSizes.size > 0) {
        sortSizes = new Map();
      }
      return;
    }
    const timer = setInterval(() => {
      const current = $dirSizes;
      if (current.size > 0) {
        sortSizes = new Map(current);
      }
    }, 3000);
    return () => clearInterval(timer);
  });

  const ROW_HEIGHT = 24;
  const OVERSCAN = 10;

  let containerEl: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let containerHeight = $state(400);

  let expandedPaths = $state(new Set<string>());
  let lastRootPath = $state<string | null>(null);

  // Auto-expand root only on initial load or when scanning a new path
  $effect(() => {
    const t = $tree;
    if (t && t.path !== lastRootPath) {
      lastRootPath = t.path;
      expandedPaths = new Set([t.path]);
    }
  });

  let currentSortState = $state<SortState>({ columnId: "size", direction: "desc" });

  $effect(() => {
    const unsub = sortState.subscribe((v) => {
      currentSortState = v;
    });
    return unsub;
  });

  let flatRows: FlatTreeRow[] = $derived.by(() => {
    const t = $tree;
    if (!t) return [];
    return flatten(t, sortSizes, currentSortState);
  });

  const MAX_VISIBLE_CHILDREN = 500;

  function getEffectiveSize(node: FileNode, sizes: Map<string, { size: number; file_count: number }>): number {
    const info = sizes.get(node.path);
    return info ? info.size : node.size;
  }

  function compareNodes(a: FileNode, b: FileNode, sort: SortState, sizes: Map<string, { size: number; file_count: number }>): number {
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
        result = getEffectiveSize(a, sizes) - getEffectiveSize(b, sizes);
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
        result = getEffectiveSize(a, sizes) - getEffectiveSize(b, sizes);
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
  function topNChildren(
    children: FileNode[],
    n: number,
    sort: SortState,
    sizes: Map<string, { size: number; file_count: number }>,
  ): FileNode[] {
    if (children.length <= n) {
      return [...children].sort((a, b) => compareNodes(a, b, sort, sizes));
    }
    // 디렉토리 우선 분리 (소수이므로 빠름)
    const dirs: FileNode[] = [];
    const files: FileNode[] = [];
    for (const c of children) {
      (c.is_dir ? dirs : files).push(c);
    }
    // 각 그룹을 정렬 (디렉토리는 보통 소수)
    dirs.sort((a, b) => compareNodes(a, b, sort, sizes));
    const remaining = n - dirs.length;
    if (remaining <= 0) {
      return dirs.slice(0, n);
    }
    if (files.length <= remaining) {
      // All files fit, no truncation needed
      files.sort((a, b) => compareNodes(a, b, sort, sizes));
      return [...dirs, ...files];
    }
    if (files.length <= remaining * 3) {
      // Small enough, full sort is fine
      files.sort((a, b) => compareNodes(a, b, sort, sizes));
      return [...dirs, ...files.slice(0, remaining)];
    }
    // For large arrays, use partial sort via quickselect
    const cmp = (a: FileNode, b: FileNode) => compareNodes(a, b, sort, sizes);
    const topFiles = quickSelectTop(files, remaining, cmp);
    topFiles.sort(cmp); // sort only the top N
    return [...dirs, ...topFiles];
  }

  /**
   * Iterative flatten: 재귀 대신 명시적 스택 사용으로 깊은 트리에서의
   * 스택 오버플로를 방지하고, 대량 children에 partial sort 적용.
   */
  function flatten(root: FileNode, sizes: Map<string, { size: number; file_count: number }>, sort: SortState): FlatTreeRow[] {
    const rows: FlatTreeRow[] = [];
    // stack: [node, depth]
    const stack: [FileNode, number][] = [[root, 0]];

    while (stack.length > 0) {
      const [node, depth] = stack.pop()!;
      const hasChildren = node.is_dir && node.children.length > 0;
      const expanded = expandedPaths.has(node.path);
      const displaySize = getEffectiveSize(node, sizes);
      rows.push({ node, depth, expanded, hasChildren, displaySize });

      if (expanded && hasChildren) {
        const total = node.children.length;
        const sorted = topNChildren(node.children, MAX_VISIBLE_CHILDREN, sort, sizes);

        // 스택은 LIFO이므로 역순으로 push해야 순서대로 출력됨
        if (total > MAX_VISIBLE_CHILDREN) {
          // truncated 노드를 먼저 push (마지막에 출력)
          stack.push([{
            name: `... and ${total - MAX_VISIBLE_CHILDREN} more items`,
            path: node.path + "/__truncated__",
            size: 0,
            is_dir: false,
            children: [],
            file_count: 0,
            extension: null,
            dir_count: 0,
            modified: null,
            is_symlink: false,
            is_hidden: false,
            is_readonly: false,
          }, depth + 1]);
        }

        for (let i = sorted.length - 1; i >= 0; i--) {
          stack.push([sorted[i], depth + 1]);
        }
      }
    }
    return rows;
  }

  function toggleExpand(path: string) {
    const next = new Set(expandedPaths);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    expandedPaths = next;
  }

  let totalHeight = $derived(flatRows.length * ROW_HEIGHT);

  let startIndex = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN));
  let endIndex = $derived(
    Math.min(flatRows.length, Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + OVERSCAN)
  );
  let visibleRows = $derived(flatRows.slice(startIndex, endIndex));

  // 스캔 중이면 progress.total_bytes, 아니면 트리 크기
  // sortSizes (throttled)를 사용하여 $dirSizes 직접 구독을 회피 (OOM 방지)
  let rootSize = $derived(
    sortSizes.size > 0 ? $progress.total_bytes : ($tree?.size ?? 0)
  );

  function handleScroll() {
    if (containerEl) {
      scrollTop = containerEl.scrollTop;
    }
  }

  // Scroll to selected path (auto-expand ancestors if needed)
  // 핵심: $selectedPath 변경에만 반응. flatRows/expandedPaths는 untrack하여 반응 루프 차단.
  $effect(() => {
    const sp = $selectedPath;  // 이것만 tracked dependency
    if (!sp || !containerEl) return;
    if (isSpecialPath(sp)) return;

    // untrack: flatRows, expandedPaths, tree 읽기는 dependency로 추적하지 않음
    untrack(() => {
      const t = $tree;
      if (!t) return;

      // 이미 보이는 경로인지 확인
      const idx = getPathIndex(sp);
      if (idx >= 0) {
        // 보이는 경로 → 스크롤만
        scrollToIdx(idx);
        return;
      }

      // 트리에 존재하는 경로인지 확인 후 조상 expand
      if (!sp.startsWith(t.path)) return;

      const next = new Set(expandedPaths);
      let changed = false;
      const parts = sp.slice(t.path.length).split("/").filter(Boolean);
      let current = t.path;
      if (!next.has(current)) { next.add(current); changed = true; }
      for (const part of parts) {
        current = current.endsWith("/") ? current + part : current + "/" + part;
        if (!next.has(current)) { next.add(current); changed = true; }
      }

      if (changed) {
        expandedPaths = next;
        // expand 후 다음 프레임에서 스크롤 (flatRows가 재계산된 후)
        requestAnimationFrame(() => {
          if (!containerEl) return;
          const newIdx = getPathIndex(sp);
          if (newIdx < 0) return;
          scrollToIdx(newIdx);
        });
      }
    });
  });

  function scrollToIdx(idx: number) {
    if (!containerEl) return;
    const top = idx * ROW_HEIGHT;
    const bottom = top + ROW_HEIGHT;
    if (top < containerEl.scrollTop) {
      containerEl.scrollTop = top;
    } else if (bottom > containerEl.scrollTop + containerHeight) {
      containerEl.scrollTop = bottom - containerHeight;
    }
  }

  function handleResize() {
    if (containerEl) {
      containerHeight = containerEl.clientHeight;
    }
  }

  // Context menu state
  let ctxMenu = $state<{ x: number; y: number; path: string; name: string; isDir: boolean; size: number; childCount: number; cleanupPatternId: string | null } | null>(null);

  function handleContextMenu(e: MouseEvent, row: FlatTreeRow) {
    e.preventDefault();
    selectedPath.set(row.node.path);
    ctxMenu = {
      x: e.clientX,
      y: e.clientY,
      path: row.node.path,
      name: row.node.name,
      isDir: row.node.is_dir,
      size: row.node.size,
      childCount: row.node.file_count + row.node.dir_count,
      cleanupPatternId: row.node.cleanup_pattern_id,
    };
  }

  // --- Keyboard navigation ---

  // Path -> index lookup: $derived rebuilds when flatRows changes
  let pathIndexMap = $derived.by(() => {
    const m = new Map<string, number>();
    for (let i = 0; i < flatRows.length; i++) {
      m.set(flatRows[i].node.path, i);
    }
    return m;
  });

  function getPathIndex(path: string): number {
    return pathIndexMap.get(path) ?? -1;
  }

  let selectedIndex = $derived($selectedPath ? (pathIndexMap.get($selectedPath) ?? -1) : -1);

  // Record child visits when selection changes
  $effect(() => {
    const sp = $selectedPath;
    if (!sp) return;
    const parentPath = getParentPath(sp);
    if (parentPath) recordChildVisit(parentPath, sp);
  });

  function selectByIndex(idx: number) {
    if (idx >= 0 && idx < flatRows.length) {
      selectedPath.set(flatRows[idx].node.path);
    }
  }

  function scrollToIndex(idx: number) {
    if (!containerEl) return;
    const top = idx * ROW_HEIGHT;
    const bottom = top + ROW_HEIGHT;
    if (top < containerEl.scrollTop) {
      containerEl.scrollTop = top;
    } else if (bottom > containerEl.scrollTop + containerHeight) {
      containerEl.scrollTop = bottom - containerHeight;
    }
  }

  function selectAndScroll(idx: number) {
    selectByIndex(idx);
    scrollToIndex(idx);
  }

  export function selectParent() {
    const sp = $selectedPath;
    if (!sp) return;
    const t = $tree;
    if (!t) return;
    const parentPath = getParentPath(sp);
    if (!parentPath || !sp.startsWith(t.path)) return;
    // Don't go above root
    if (parentPath.length < t.path.length) return;
    selectedPath.set(parentPath);
  }

  export function reselectChild() {
    const sp = $selectedPath;
    if (!sp) return;
    const lastChild = getLastChild(sp);
    if (lastChild) {
      // Ensure parent is expanded so child is visible
      if (!expandedPaths.has(sp)) {
        const next = new Set(expandedPaths);
        next.add(sp);
        expandedPaths = next;
      }
      selectedPath.set(lastChild);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    const len = flatRows.length;
    if (len === 0) return;

    switch (e.key) {
      case "ArrowUp": {
        e.preventDefault();
        if (e.altKey) {
          selectParent();
        } else {
          const target = selectedIndex > 0 ? selectedIndex - 1 : 0;
          selectAndScroll(target);
        }
        break;
      }
      case "ArrowDown": {
        e.preventDefault();
        if (e.altKey) {
          reselectChild();
        } else {
          const target = selectedIndex < len - 1 ? selectedIndex + 1 : len - 1;
          selectAndScroll(target);
        }
        break;
      }
      case "ArrowLeft": {
        e.preventDefault();
        if (selectedIndex < 0) break;
        const row = flatRows[selectedIndex];
        if (row.expanded && row.hasChildren) {
          // Collapse
          toggleExpand(row.node.path);
        } else {
          // Move to parent
          selectParent();
        }
        break;
      }
      case "ArrowRight": {
        e.preventDefault();
        if (selectedIndex < 0) break;
        const row = flatRows[selectedIndex];
        if (row.hasChildren && !row.expanded) {
          // Expand
          toggleExpand(row.node.path);
        } else if (row.hasChildren && row.expanded) {
          // Move to first child (next row in flatRows)
          selectAndScroll(selectedIndex + 1);
        }
        break;
      }
      case "Enter": {
        e.preventDefault();
        if (selectedIndex < 0) break;
        const row = flatRows[selectedIndex];
        if (row.hasChildren) {
          toggleExpand(row.node.path);
        }
        break;
      }
      case "Home": {
        e.preventDefault();
        selectAndScroll(0);
        break;
      }
      case "End": {
        e.preventDefault();
        selectAndScroll(len - 1);
        break;
      }
      default:
        return; // Don't prevent default for unhandled keys
    }
  }
</script>

<svelte:window onresize={handleResize} />

<div class="tree-container">
  <TreeColumnHeader />
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="tree-view"
    role="tree"
    tabindex="0"
    bind:this={containerEl}
    onscroll={handleScroll}
    onkeydown={handleKeydown}
  >
    <div class="tree-scroll-content" style="height: {totalHeight}px">
      <div class="tree-visible" style="transform: translateY({startIndex * ROW_HEIGHT}px)">
        {#each visibleRows as row (row.node.path)}
          <TreeRow {row} parentSize={rootSize} onToggle={toggleExpand} onContextMenu={(e) => handleContextMenu(e, row)} />
        {/each}
      </div>
    </div>
  </div>
</div>

{#if ctxMenu}
  <ContextMenu
    x={ctxMenu.x}
    y={ctxMenu.y}
    path={ctxMenu.path}
    name={ctxMenu.name}
    isDir={ctxMenu.isDir}
    size={ctxMenu.size}
    childCount={ctxMenu.childCount}
    cleanupPatternId={ctxMenu.cleanupPatternId}
    onClose={() => ctxMenu = null}
  />
{/if}

<style>
  .tree-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1a1a1a;
  }

  .tree-view {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    outline: none;
  }

  .tree-view:focus-visible {
    outline: 1px solid #4A90D9;
    outline-offset: -1px;
  }

  .tree-scroll-content {
    position: relative;
  }

  .tree-visible {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
  }
</style>
