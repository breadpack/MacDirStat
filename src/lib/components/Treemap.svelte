<script lang="ts">
  import type { FileNode } from "../types";
  import { tree, dirSizes, currentVolume } from "../stores/scanStore";
  import { selectedPath, hoveredPath, zoomRoot, showFreeSpace, showUnknown } from "../stores/selectionStore";
  import { computeTreemap, type TreemapRect } from "../utils/treemapLayout";
  import { getColor } from "../utils/colorMap";
  import { renderCushionTreemap, type CushionParams } from "../utils/cushionShading";
  import { treemapOptions } from "../stores/treemapOptionsStore";
  import { settings } from "../stores/settingsStore";
  import { highlightedExtension } from "../stores/extensionStore";
  import { formatSize } from "../utils/format";
  import { injectSpecialNodes, isSpecialPath } from "../utils/specialNodes";

  let canvas: HTMLCanvasElement | undefined = $state();
  let containerEl: HTMLDivElement | undefined = $state();
  let width = $state(600);
  let height = $state(400);

  let rects: TreemapRect[] = $state([]);

  // dirSizes의 throttled 스냅샷 (3초마다 갱신)
  // $dirSizes를 $effect/$derived에서 직접 읽으면 매 progress 업데이트마다
  // 트리 전체 복제 + treemap 재계산이 발생하여 OOM 크래시를 유발함
  let throttledSizes = $state<Map<string, { size: number; file_count: number }>>(new Map());
  let sizesVersion = $state(0);

  $effect(() => {
    const timer = setInterval(() => {
      const current = $dirSizes;
      if (current.size > 0) {
        throttledSizes = new Map(current);
        sizesVersion++;
      } else if (throttledSizes.size > 0) {
        throttledSizes = new Map();
        sizesVersion++;
      }
    }, 3000);
    return () => clearInterval(timer);
  });

  let tooltipVisible = $state(false);
  let tooltipX = $state(0);
  let tooltipY = $state(0);
  let tooltipNode: FileNode | null = $state(null);

  function findNodeByPath(node: FileNode, path: string): FileNode | null {
    if (node.path === path) return node;
    for (const child of node.children) {
      const found = findNodeByPath(child, path);
      if (found) return found;
    }
    return null;
  }

  /**
   * dirSizes의 실시간 크기를 트리에 반영한 복사본 생성.
   * 최적화: dirSizes에 해당 경로가 있는 서브트리만 복제하고,
   * 나머지는 원본 참조를 그대로 사용하여 불필요한 딥클론 방지.
   */
  function applyDirSizes(
    node: FileNode,
    sizes: Map<string, { size: number; file_count: number }>,
    affectedPrefixes: Set<string>,
  ): FileNode {
    const info = sizes.get(node.path);
    // 이 노드 하위에 영향받는 경로가 없으면 원본 반환 (클론 없음)
    if (!info && !hasAffectedDescendant(node.path, affectedPrefixes)) {
      return node;
    }
    if (!node.is_dir) {
      return info ? { ...node, size: info.size } : node;
    }
    const children = node.children.map((c) => applyDirSizes(c, sizes, affectedPrefixes));
    const childSum = children.reduce((s, c) => s + c.size, 0);
    const ownSize = info ? info.size : 0;
    const totalSize = ownSize + childSum;
    return { ...node, children, size: totalSize > 0 ? totalSize : node.size };
  }

  /** path 하위에 영향받는 경로가 있는지 빠르게 검사 */
  function hasAffectedDescendant(path: string, prefixes: Set<string>): boolean {
    const pathWithSlash = path.endsWith("/") ? path : path + "/";
    for (const p of prefixes) {
      if (p.startsWith(pathWithSlash) || p === path) return true;
    }
    return false;
  }

  /** dirSizes의 경로들에서 영향받는 prefix 세트 구축 */
  function buildAffectedPrefixes(sizes: Map<string, { size: number; file_count: number }>): Set<string> {
    const prefixes = new Set<string>();
    for (const path of sizes.keys()) {
      prefixes.add(path);
    }
    return prefixes;
  }

  let displayRoot: FileNode | null = $derived.by(() => {
    const t = $tree;
    if (!t) return null;
    const zr = $zoomRoot;
    if (zr) {
      return findNodeByPath(t, zr) ?? t;
    }
    return t;
  });

  $effect(() => {
    // sizesVersion에 의존하여 스캔 중 3초마다만 재계산 (dirSizes 직접 구독 금지)
    void sizesVersion;
    const opts = $treemapOptions;
    if (!displayRoot || width <= 0 || height <= 0) {
      rects = [];
      return;
    }
    // throttled 스냅샷을 사용하여 실시간 크기 반영
    const sizes = throttledSizes;
    let root = displayRoot;
    if (sizes.size > 0) {
      const prefixes = buildAffectedPrefixes(sizes);
      root = applyDirSizes(displayRoot, sizes, prefixes);
    }

    // skeleton 트리(size=0)는 렌더링 스킵
    if (root.size <= 0) {
      rects = [];
      return;
    }

    // Inject free space / unknown nodes only when viewing scan root (not zoomed)
    const zr = $zoomRoot;
    if (!zr) {
      root = injectSpecialNodes(root, $currentVolume, $showFreeSpace, $showUnknown);
    }

    rects = computeTreemap(root, width, height, opts.padding);
  });

  // Cache cushion ImageData to avoid recomputation when only selection/hover changes
  let cachedImageData: ImageData | null = $state(null);
  let cachedRectsRef: TreemapRect[] | null = $state(null);
  let cachedWidth = $state(0);
  let cachedHeight = $state(0);

  /** Build CushionParams from store options */
  function buildCushionParams(): CushionParams {
    const opts = $treemapOptions;
    const lx = opts.lightX;
    const ly = opts.lightY;
    const lz = 1;
    const len = Math.sqrt(lx * lx + ly * ly + lz * lz);
    return {
      brightness: opts.brightness,
      cushionHeight: opts.cushionHeight,
      scaleFactor: opts.scaleFactor,
      ambientLight: opts.ambientLight,
      lightDir: { x: lx / len, y: ly / len, z: lz / len },
    };
  }

  // Recompute cushion ImageData when rects, dimensions, or options change
  $effect(() => {
    const opts = $treemapOptions;
    if (rects.length === 0 || width <= 0 || height <= 0) {
      cachedImageData = null;
      cachedRectsRef = null;
      return;
    }
    if (opts.cushionEnabled) {
      cachedImageData = renderCushionTreemap(rects, width, height, buildCushionParams());
    } else {
      cachedImageData = null;
    }
    cachedRectsRef = rects;
    cachedWidth = width;
    cachedHeight = height;
  });

  // Render: put cached ImageData + overlay grid/borders/labels/selection
  $effect(() => {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, width, height);

    const sp = $selectedPath;
    const hp = $hoveredPath;
    const opts = $treemapOptions;
    const hlExt = $highlightedExtension;

    if (cachedImageData && cachedWidth === width && cachedHeight === height) {
      // Draw cushion-shaded treemap
      ctx.putImageData(cachedImageData, 0, 0);
    } else {
      // Fallback: flat rendering if no cached data (cushion disabled or not ready)
      for (const rect of rects) {
        const w = rect.x1 - rect.x0;
        const h = rect.y1 - rect.y0;
        if (w < 1 || h < 1) continue;
        ctx.fillStyle = getColor(rect.data.extension, rect.data.is_dir);
        ctx.fillRect(rect.x0, rect.y0, w, h);
      }
    }

    // Extension highlight: dim non-matching rects
    if (hlExt) {
      for (const rect of rects) {
        const w = rect.x1 - rect.x0;
        const h = rect.y1 - rect.y0;
        if (w < 1 || h < 1) continue;
        const ext = rect.data.extension?.toLowerCase() ?? "(no ext)";
        if (ext !== hlExt) {
          ctx.fillStyle = "rgba(0, 0, 0, 0.7)";
          ctx.fillRect(rect.x0, rect.y0, w, h);
        } else {
          ctx.strokeStyle = "#fff";
          ctx.lineWidth = 1.5;
          ctx.strokeRect(rect.x0 + 0.5, rect.y0 + 0.5, w - 1, h - 1);
        }
      }
    }

    // Draw grid borders on top
    if (opts.gridEnabled && opts.gridWidth > 0) {
      ctx.strokeStyle = opts.gridColor + "66"; // add alpha
      ctx.lineWidth = opts.gridWidth;
      for (const rect of rects) {
        const w = rect.x1 - rect.x0;
        const h = rect.y1 - rect.y0;
        if (w < 1 || h < 1) continue;
        ctx.strokeRect(rect.x0 + 0.5, rect.y0 + 0.5, w - 1, h - 1);
      }
    }

    // Highlight selected / hovered
    for (const rect of rects) {
      const w = rect.x1 - rect.x0;
      const h = rect.y1 - rect.y0;
      if (w < 1 || h < 1) continue;

      if (sp === rect.data.path) {
        ctx.strokeStyle = "#fff";
        ctx.lineWidth = 2;
        ctx.strokeRect(rect.x0 + 1, rect.y0 + 1, w - 2, h - 2);
      } else if (hp === rect.data.path) {
        ctx.strokeStyle = "rgba(255,255,255,0.5)";
        ctx.lineWidth = 1;
        ctx.strokeRect(rect.x0 + 0.5, rect.y0 + 0.5, w - 1, h - 1);
      }

      // Draw label if rect is large enough and labels are enabled
      if ($settings.treemap.showLabels && w > 40 && h > 14) {
        ctx.fillStyle = "rgba(255,255,255,0.8)";
        ctx.font = "11px sans-serif";
        const label = rect.data.name;
        const textWidth = ctx.measureText(label).width;
        if (textWidth < w - 4) {
          ctx.fillText(label, rect.x0 + 2, rect.y0 + 12);
        }
      }
    }
  });

  function hitTest(x: number, y: number): TreemapRect | null {
    for (let i = rects.length - 1; i >= 0; i--) {
      const r = rects[i];
      if (x >= r.x0 && x <= r.x1 && y >= r.y0 && y <= r.y1) {
        return r;
      }
    }
    return null;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const hit = hitTest(x, y);
    if (hit) {
      hoveredPath.set(hit.data.path);
      tooltipVisible = true;
      tooltipX = e.clientX;
      tooltipY = e.clientY;
      tooltipNode = hit.data;
    } else {
      hoveredPath.set(null);
      tooltipVisible = false;
      tooltipNode = null;
    }
  }

  function handleClick(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const hit = hitTest(x, y);
    if (hit && !isSpecialPath(hit.data.path)) {
      selectedPath.set(hit.data.path);
    }
  }

  function handleDblClick(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const hit = hitTest(x, y);
    if (hit && hit.data.is_dir && !isSpecialPath(hit.data.path)) {
      zoomRoot.set(hit.data.path);
    }
  }

  function handleMouseLeave() {
    hoveredPath.set(null);
    tooltipVisible = false;
    tooltipNode = null;
  }

  function handleResize() {
    if (containerEl) {
      width = containerEl.clientWidth;
      height = containerEl.clientHeight;
    }
  }

  $effect(() => {
    if (containerEl) {
      const ro = new ResizeObserver(() => {
        width = containerEl!.clientWidth;
        height = containerEl!.clientHeight;
      });
      ro.observe(containerEl);
      return () => ro.disconnect();
    }
  });
</script>

<svelte:window onresize={handleResize} />

<div class="treemap-container" bind:this={containerEl}>
  <canvas
    bind:this={canvas}
    {width}
    {height}
    onmousemove={handleMouseMove}
    onclick={handleClick}
    ondblclick={handleDblClick}
    onmouseleave={handleMouseLeave}
  ></canvas>

  {#if tooltipVisible && tooltipNode}
    <div
      class="tooltip"
      style="left: {tooltipX + 12}px; top: {tooltipY + 12}px"
    >
      <div class="tooltip-name">{tooltipNode.name}</div>
      <div class="tooltip-size">{formatSize(tooltipNode.size)}</div>
      <div class="tooltip-path">{tooltipNode.path}</div>
    </div>
  {/if}
</div>

<style>
  .treemap-container {
    position: relative;
    width: 100%;
    height: 100%;
    background: #111;
    overflow: hidden;
  }

  canvas {
    display: block;
  }

  .tooltip {
    position: fixed;
    background: rgba(30, 30, 30, 0.95);
    border: 1px solid #555;
    border-radius: 4px;
    padding: 6px 10px;
    pointer-events: none;
    z-index: 100;
    max-width: 400px;
    font-size: 12px;
    color: #ccc;
  }

  .tooltip-name {
    font-weight: bold;
    color: #fff;
  }

  .tooltip-size {
    color: #4A90D9;
  }

  .tooltip-path {
    color: #888;
    font-size: 11px;
    word-break: break-all;
  }
</style>
