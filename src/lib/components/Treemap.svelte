<script lang="ts">
  import type { TreemapNode } from "../types";
  import { treeRootPath, slowTreeVersion, currentVolume } from "../stores/scanStore";
  import { selectedPath, hoveredPath, zoomRoot, showFreeSpace, showUnknown } from "../stores/selectionStore";
  import { computeTreemap, type TreemapRect } from "../utils/treemapLayout";
  import { getColor } from "../utils/colorMap";
  import { renderCushionTreemap, type CushionParams } from "../utils/cushionShading";
  import { treemapOptions } from "../stores/treemapOptionsStore";
  import { settings } from "../stores/settingsStore";
  import { highlightedExtension } from "../stores/extensionStore";
  import { formatSize } from "../utils/format";
  import { injectSpecialNodes, isSpecialPath } from "../utils/specialNodes";
  import { getTreemap } from "../api";

  /** 표시 루트 대비 이 비율보다 작은 노드는 부모의 "(N others)"로 합산 */
  const MIN_SIZE_RATIO = 0.0001;
  const MAX_CHILDREN = 200;
  /** 백엔드가 반환할 최대 노드 수 (큰 디렉토리부터 확장) */
  const MAX_NODES = 20000;

  let canvas: HTMLCanvasElement | undefined = $state();
  let overlayCanvas: HTMLCanvasElement | undefined = $state();
  let containerEl: HTMLDivElement | undefined = $state();
  let width = $state(600);
  let height = $state(400);

  // raw: 수천 개의 rect를 deep proxy로 감싸지 않도록
  let rects: TreemapRect[] = $state.raw([]);

  let tooltipVisible = $state(false);
  let tooltipX = $state(0);
  let tooltipY = $state(0);
  let tooltipNode: TreemapNode | null = $state(null);

  let layoutRoot: TreemapNode | null = $state.raw(null);
  let treemapRequest = 0;

  // 백엔드에서 가지치기된 트리맵 트리를 조회 (스캔 중에는 slowTreeVersion 주기로만 갱신)
  $effect(() => {
    void $slowTreeVersion;
    const rootPath = $treeRootPath;
    const zr = $zoomRoot;
    if (!rootPath) {
      layoutRoot = null;
      return;
    }
    const req = ++treemapRequest;
    getTreemap(zr ?? rootPath, MIN_SIZE_RATIO, MAX_CHILDREN, MAX_NODES)
      .then((root) => {
        if (req !== treemapRequest) return;
        if (!root && zr) {
          // zoom 대상이 삭제됨 → 루트로 복귀
          zoomRoot.set(null);
          return;
        }
        layoutRoot = root;
      })
      .catch((e) => console.warn("Failed to load treemap:", e));
  });

  $effect(() => {
    const opts = $treemapOptions;
    let root = layoutRoot;
    if (!root || width <= 0 || height <= 0 || root.size <= 0) {
      rects = [];
      return;
    }
    // Inject free space / unknown nodes only when viewing scan root (not zoomed)
    if (!$zoomRoot) {
      root = injectSpecialNodes(root, $currentVolume, $showFreeSpace, $showUnknown);
    }
    rects = computeTreemap(root, width, height, opts.padding);
  });

  // Cache cushion ImageData to avoid recomputation when only selection/hover changes
  let cachedImageData: ImageData | null = $state.raw(null);
  let cachedRectsRef: TreemapRect[] | null = $state.raw(null);
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

  // Derived key that only tracks cushion-relevant options
  let cushionKey = $derived(`${$treemapOptions.cushionEnabled}|${$treemapOptions.brightness}|${$treemapOptions.cushionHeight}|${$treemapOptions.scaleFactor}|${$treemapOptions.ambientLight}|${$treemapOptions.lightX}|${$treemapOptions.lightY}`);

  // Recompute cushion ImageData only when rects, dimensions, or cushion-relevant options change
  $effect(() => {
    void cushionKey; // track only cushion-relevant options
    if (rects.length === 0 || width <= 0 || height <= 0) {
      cachedImageData = null;
      cachedRectsRef = null;
      return;
    }
    const opts = $treemapOptions;
    if (opts.cushionEnabled) {
      cachedImageData = renderCushionTreemap(rects, width, height, buildCushionParams());
    } else {
      cachedImageData = null;
    }
    cachedRectsRef = rects;
    cachedWidth = width;
    cachedHeight = height;
  });

  // Static layer: cushion/flat rendering + grid + labels
  $effect(() => {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, width, height);

    const opts = $treemapOptions;

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

    // Draw cleanup hatching overlay
    if (opts.showCleanupOverlay) {
      renderCleanupOverlay(ctx, rects);
    }

    // Draw labels (font set once outside the loop)
    if ($settings.treemap.showLabels) {
      ctx.fillStyle = "rgba(255,255,255,0.8)";
      ctx.font = "11px sans-serif";
      for (const rect of rects) {
        const w = rect.x1 - rect.x0;
        const h = rect.y1 - rect.y0;
        if (w > 40 && h > 14) {
          const label = rect.data.name;
          const textWidth = ctx.measureText(label).width;
          if (textWidth < w - 4) {
            ctx.fillText(label, rect.x0 + 2, rect.y0 + 12);
          }
        }
      }
    }
  });

  // Overlay layer: hover/selection borders + extension highlight
  $effect(() => {
    if (!overlayCanvas) return;
    const ctx = overlayCanvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, width, height);

    const sp = $selectedPath;
    const hp = $hoveredPath;
    const hlExt = $highlightedExtension;

    // Extension highlight: dim non-matching rects (batched into single path)
    if (hlExt) {
      ctx.fillStyle = "rgba(0, 0, 0, 0.7)";
      ctx.beginPath();
      for (const rect of rects) {
        const w = rect.x1 - rect.x0;
        const h = rect.y1 - rect.y0;
        if (w < 1 || h < 1) continue;
        const ext = rect.data.extension?.toLowerCase() ?? "(no ext)";
        if (ext !== hlExt) {
          ctx.rect(rect.x0, rect.y0, w, h);
        }
      }
      ctx.fill();
      // Draw borders on matching rects
      ctx.strokeStyle = "#fff";
      ctx.lineWidth = 1.5;
      for (const rect of rects) {
        const w = rect.x1 - rect.x0;
        const h = rect.y1 - rect.y0;
        if (w < 1 || h < 1) continue;
        const ext = rect.data.extension?.toLowerCase() ?? "(no ext)";
        if (ext === hlExt) {
          ctx.strokeRect(rect.x0 + 0.5, rect.y0 + 0.5, w - 1, h - 1);
        }
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
    }
  });

  function renderCleanupOverlay(ctx: CanvasRenderingContext2D, nodes: TreemapRect[]) {
    ctx.save();
    ctx.strokeStyle = "rgba(255, 255, 255, 0.3)";
    ctx.lineWidth = 1;

    for (const node of nodes) {
      if (!node.data.cleanup_pattern_id) continue;
      const x0 = node.x0, y0 = node.y0, x1 = node.x1, y1 = node.y1;
      const w = x1 - x0, h = y1 - y0;
      if (w < 4 || h < 4) continue;

      ctx.beginPath();
      ctx.rect(x0, y0, w, h);
      ctx.clip();

      const step = 8;
      const totalDist = w + h;
      for (let d = step; d < totalDist; d += step) {
        const startX = x0 + Math.min(d, w);
        const startY = y0 + Math.max(0, d - w);
        const endX = x0 + Math.max(0, d - h);
        const endY = y0 + Math.min(d, h);
        ctx.moveTo(startX, startY);
        ctx.lineTo(endX, endY);
      }
      ctx.stroke();
      ctx.restore();
      ctx.save();
    }
    ctx.restore();
  }

  function hitTest(x: number, y: number): TreemapRect | null {
    for (let i = rects.length - 1; i >= 0; i--) {
      const r = rects[i];
      if (x >= r.x0 && x <= r.x1 && y >= r.y0 && y <= r.y1) {
        return r;
      }
    }
    return null;
  }

  let rafPending = false;
  function handleMouseMove(e: MouseEvent) {
    if (rafPending) return;
    rafPending = true;
    requestAnimationFrame(() => {
      rafPending = false;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      const hit = hitTest(x, y);
      if (hit) {
        hoveredPath.set(hit.data.path);
        tooltipVisible = true;
        // Clamp tooltip to viewport
        const tooltipWidth = 400;
        const tooltipHeight = 80;
        let tx = e.clientX + 12;
        let ty = e.clientY + 12;
        if (tx + tooltipWidth > window.innerWidth) {
          tx = e.clientX - tooltipWidth - 12;
        }
        if (ty + tooltipHeight > window.innerHeight) {
          ty = e.clientY - tooltipHeight - 12;
        }
        tooltipX = tx;
        tooltipY = ty;
        tooltipNode = hit.data;
      } else {
        hoveredPath.set(null);
        tooltipVisible = false;
        tooltipNode = null;
      }
    });
  }

  /** special path(__others__ 등)의 경우 부모 디렉토리 경로를 반환 */
  function resolveClickPath(hit: TreemapRect): string {
    if (isSpecialPath(hit.data.path)) {
      // 부모 디렉토리 경로 = __others__ 등의 앞부분
      const idx = hit.data.path.lastIndexOf("/");
      return idx > 0 ? hit.data.path.substring(0, idx) : hit.data.path;
    }
    return hit.data.path;
  }

  /** leaf 노드의 부모 디렉토리 경로를 찾아 반환 (zoom용) */
  function getParentDirPath(hit: TreemapRect): string | null {
    // leaf의 path에서 마지막 '/'까지가 부모 디렉토리
    const path = hit.data.path;
    const idx = path.lastIndexOf("/");
    if (idx <= 0) return null;
    return path.substring(0, idx);
  }

  function handleClick(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const hit = hitTest(x, y);
    if (hit) {
      selectedPath.set(resolveClickPath(hit));
    }
  }

  function handleDblClick(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const hit = hitTest(x, y);
    if (!hit) return;
    // 디렉토리 leaf이면 해당 디렉토리로 zoom, 파일이면 부모 디렉토리로 zoom
    const zoomPath = hit.data.is_dir && !isSpecialPath(hit.data.path)
      ? hit.data.path
      : getParentDirPath(hit);
    if (zoomPath) {
      zoomRoot.set(zoomPath);
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
  ></canvas>
  <canvas
    bind:this={overlayCanvas}
    {width}
    {height}
    class="overlay-canvas"
    onmousemove={handleMouseMove}
    onclick={handleClick}
    ondblclick={handleDblClick}
    onmouseleave={handleMouseLeave}
  ></canvas>

  {#if tooltipVisible && tooltipNode}
    <div
      class="tooltip"
      style="left: {tooltipX}px; top: {tooltipY}px"
    >
      <div class="tooltip-name">{tooltipNode.name}{tooltipNode.cleanup_pattern_id ? ' (cleanable)' : ''}</div>
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
    background: var(--bg-primary);
    overflow: hidden;
  }

  canvas {
    display: block;
  }

  .overlay-canvas {
    position: absolute;
    top: 0;
    left: 0;
  }

  .tooltip {
    position: fixed;
    background: var(--tooltip-bg);
    border: 1px solid var(--border-color-strong);
    border-radius: 4px;
    padding: 6px 10px;
    pointer-events: none;
    z-index: 100;
    max-width: 400px;
    font-size: 12px;
    color: var(--text-primary);
  }

  .tooltip-name {
    font-weight: bold;
    color: var(--text-heading);
  }

  .tooltip-size {
    color: var(--accent-color);
  }

  .tooltip-path {
    color: var(--text-secondary);
    font-size: 11px;
    word-break: break-all;
  }
</style>
