<script lang="ts">
  import { untrack } from "svelte";
  import type { FileNode, FlatTreeRow } from "../types";
  import { tree, treeRootPath, treeVersion } from "../stores/scanStore";
  import { selectedPath } from "../stores/selectionStore";
  import { sortState, type SortState } from "../stores/columnStore";
  import { getParentPath, recordChildVisit, getLastChild } from "../stores/navigationStore";
  import { isSpecialPath } from "../utils/specialNodes";
  import { topNChildren } from "../utils/treeSort";
  import { getChildren } from "../api";
  import TreeRow from "./TreeRow.svelte";
  import TreeColumnHeader from "./TreeColumnHeader.svelte";
  import ContextMenu from "./ContextMenu.svelte";

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

  // 펼쳐진 디렉토리의 자식 목록 캐시 (백엔드 조회 결과). 트리 버전이 바뀌면 전부 다시 조회한다.
  let childrenCache = $state.raw(new Map<string, FileNode[]>());
  let cacheVersion = -1;
  let childrenRequest = 0;
  let appliedRequest = 0;

  $effect(() => {
    const version = $treeVersion;
    const rootPath = $treeRootPath;
    const expanded = expandedPaths;
    untrack(() => loadChildren(rootPath, expanded, version));
  });

  async function loadChildren(rootPath: string | null, expanded: Set<string>, version: number) {
    if (!rootPath) {
      childrenCache = new Map();
      cacheVersion = -1;
      return;
    }
    const stale = version !== cacheVersion;
    const paths = stale ? [...expanded] : [...expanded].filter((p) => !childrenCache.has(p));
    if (paths.length === 0) return;
    const req = ++childrenRequest;
    try {
      const results = await getChildren(paths);
      // 더 최신 요청이 이미 반영됐으면 무시 (요청이 겹쳐도 갱신이 멈추지 않도록 최신만 버리지 않음)
      if (req < appliedRequest) return;
      appliedRequest = req;
      const next = stale ? new Map<string, FileNode[]>() : new Map(childrenCache);
      paths.forEach((p, i) => {
        const children = results[i];
        if (children) next.set(p, children);
      });
      cacheVersion = version;
      childrenCache = next;
    } catch (e) {
      console.warn("Failed to load children:", e);
    }
  }

  let flatRows: FlatTreeRow[] = $derived.by(() => {
    const t = $tree;
    if (!t) return [];
    return flatten(t, childrenCache, currentSortState);
  });

  const MAX_VISIBLE_CHILDREN = 500;

  /**
   * Iterative flatten: 재귀 대신 명시적 스택 사용으로 깊은 트리에서의
   * 스택 오버플로를 방지하고, 대량 children에 partial sort 적용.
   */
  function flatten(root: FileNode, cache: Map<string, FileNode[]>, sort: SortState): FlatTreeRow[] {
    const rows: FlatTreeRow[] = [];
    // stack: [node, depth]
    const stack: [FileNode, number][] = [[root, 0]];

    while (stack.length > 0) {
      const [node, depth] = stack.pop()!;
      const hasChildren = node.is_dir && node.child_count > 0;
      const expanded = expandedPaths.has(node.path);
      rows.push({ node, depth, expanded, hasChildren, displaySize: node.size });

      const children = expanded && hasChildren ? cache.get(node.path) : undefined;
      if (children) {
        const total = children.length;
        const sorted = topNChildren(children, MAX_VISIBLE_CHILDREN, sort);

        // 스택은 LIFO이므로 역순으로 push해야 순서대로 출력됨
        if (total > MAX_VISIBLE_CHILDREN) {
          // truncated 노드를 먼저 push (마지막에 출력)
          stack.push([{
            name: `... and ${total - MAX_VISIBLE_CHILDREN} more items`,
            path: node.path + "/__truncated__",
            size: 0,
            is_dir: false,
            child_count: 0,
            cleanup_pattern_id: null,
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

  // 백엔드 트리는 스캔 중에도 실시간으로 크기가 채워지므로 루트 크기를 그대로 사용
  let rootSize = $derived($tree?.size ?? 0);

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
        // 자식 목록은 비동기로 로드되므로, 행이 나타나면 스크롤한다
        pendingScrollPath = sp;
        expandedPaths = next;
      }
    });
  });

  let pendingScrollPath: string | null = null;

  $effect(() => {
    void flatRows;
    untrack(() => {
      if (!pendingScrollPath || !containerEl) return;
      const idx = getPathIndex(pendingScrollPath);
      if (idx < 0) return;
      pendingScrollPath = null;
      scrollToIdx(idx);
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
    background: var(--bg-primary);
  }

  .tree-view {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    outline: none;
  }

  .tree-view:focus-visible {
    outline: 1px solid var(--accent-color);
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
