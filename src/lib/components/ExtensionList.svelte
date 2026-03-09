<script lang="ts">
  import { extensionStats, extensionColorMap, highlightedExtension } from "../stores/extensionStore";
  import { formatSize, formatNumber } from "../utils/format";
  import type { ExtensionStat } from "../utils/extensionStats";

  const ROW_HEIGHT = 24;
  const OVERSCAN = 10;

  let containerEl: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let containerHeight = $state(300);

  // Sort state
  type SortColumn = "extension" | "bytes" | "percentage" | "files";
  type SortDir = "asc" | "desc";
  let sortColumn = $state<SortColumn>("bytes");
  let sortDir = $state<SortDir>("desc");

  // Subscribe to stores
  let stats = $state<ExtensionStat[]>([]);
  let colorMap = $state<Map<string, string>>(new Map());
  let highlighted = $state<string | null>(null);

  $effect(() => {
    const unsub = extensionStats.subscribe((v) => { stats = v; });
    return unsub;
  });

  $effect(() => {
    const unsub = extensionColorMap.subscribe((v) => { colorMap = v; });
    return unsub;
  });

  $effect(() => {
    const unsub = highlightedExtension.subscribe((v) => { highlighted = v; });
    return unsub;
  });

  // Sorted stats
  let sortedStats = $derived.by(() => {
    const sorted = [...stats];
    sorted.sort((a, b) => {
      let result = 0;
      switch (sortColumn) {
        case "extension":
          result = a.extension.localeCompare(b.extension);
          break;
        case "bytes":
          result = a.totalBytes - b.totalBytes;
          break;
        case "percentage":
          result = a.percentage - b.percentage;
          break;
        case "files":
          result = a.fileCount - b.fileCount;
          break;
      }
      return sortDir === "desc" ? -result : result;
    });
    // Assign colors from the color map
    for (const s of sorted) {
      s.color = colorMap.get(s.extension) ?? "#666";
    }
    return sorted;
  });

  // Virtual scrolling
  let totalHeight = $derived(sortedStats.length * ROW_HEIGHT);
  let startIndex = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN));
  let endIndex = $derived(
    Math.min(sortedStats.length, Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + OVERSCAN)
  );
  let visibleRows = $derived(sortedStats.slice(startIndex, endIndex));

  function handleScroll() {
    if (containerEl) {
      scrollTop = containerEl.scrollTop;
    }
  }

  function handleResize() {
    if (containerEl) {
      containerHeight = containerEl.clientHeight;
    }
  }

  function toggleSort(col: SortColumn) {
    if (sortColumn === col) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
    } else {
      sortColumn = col;
      sortDir = col === "extension" ? "asc" : "desc";
    }
  }

  function handleRowClick(ext: string) {
    if (highlighted === ext) {
      highlightedExtension.set(null);
    } else {
      highlightedExtension.set(ext);
    }
  }

  function sortIndicator(col: SortColumn): string {
    if (sortColumn !== col) return "";
    return sortDir === "asc" ? " \u25B2" : " \u25BC";
  }

  $effect(() => {
    if (containerEl) {
      const ro = new ResizeObserver(() => {
        containerHeight = containerEl!.clientHeight;
      });
      ro.observe(containerEl);
      return () => ro.disconnect();
    }
  });
</script>

<svelte:window onresize={handleResize} />

<div class="ext-panel">
  <div class="ext-header">
    <button class="col-ext" onclick={() => toggleSort("extension")}>
      Ext{sortIndicator("extension")}
    </button>
    <span class="col-color"></span>
    <button class="col-bytes" onclick={() => toggleSort("bytes")}>
      Bytes{sortIndicator("bytes")}
    </button>
    <button class="col-pct" onclick={() => toggleSort("percentage")}>
      %{sortIndicator("percentage")}
    </button>
    <button class="col-files" onclick={() => toggleSort("files")}>
      Files{sortIndicator("files")}
    </button>
  </div>
  <div
    class="ext-list"
    bind:this={containerEl}
    onscroll={handleScroll}
  >
    <div class="ext-scroll-content" style="height: {totalHeight}px">
      <div class="ext-visible" style="transform: translateY({startIndex * ROW_HEIGHT}px)">
        {#each visibleRows as stat (stat.extension)}
          <button
            class="ext-row"
            class:highlighted={highlighted === stat.extension}
            onclick={() => handleRowClick(stat.extension)}
          >
            <span class="col-ext" title={stat.extension}>
              {stat.extension}
            </span>
            <span class="col-color">
              <span class="color-swatch" style="background: {stat.color}"></span>
            </span>
            <span class="col-bytes">{formatSize(stat.totalBytes)}</span>
            <span class="col-pct">
              <span class="pct-bar-bg">
                <span class="pct-bar-fill" style="width: {Math.min(stat.percentage, 100)}%"></span>
              </span>
              <span class="pct-text">{stat.percentage.toFixed(1)}</span>
            </span>
            <span class="col-files">{formatNumber(stat.fileCount)}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>
  <div class="ext-footer">
    {sortedStats.length} extensions
  </div>
</div>

<style>
  .ext-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1a1a1a;
    font-size: 11px;
    color: #ccc;
    min-width: 0;
  }

  .ext-header {
    display: flex;
    align-items: center;
    height: 24px;
    background: #252525;
    border-bottom: 1px solid #333;
    flex-shrink: 0;
    padding: 0 2px;
  }

  .ext-header button {
    background: none;
    border: none;
    color: #aaa;
    font-size: 10px;
    font-weight: 600;
    cursor: pointer;
    padding: 0 2px;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ext-header button:hover {
    color: #fff;
  }

  .ext-list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .ext-scroll-content {
    position: relative;
  }

  .ext-visible {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
  }

  .ext-row {
    display: flex;
    align-items: center;
    height: 24px;
    padding: 0 2px;
    cursor: pointer;
    border: none;
    background: none;
    color: #ccc;
    font-size: 11px;
    width: 100%;
    text-align: left;
  }

  .ext-row:hover {
    background: #2a2a2a;
  }

  .ext-row.highlighted {
    background: #333d4a;
  }

  .col-ext {
    width: 52px;
    min-width: 52px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-color {
    width: 16px;
    min-width: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .color-swatch {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .col-bytes {
    width: 52px;
    min-width: 52px;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-pct {
    width: 52px;
    min-width: 52px;
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .pct-bar-bg {
    flex: 1;
    height: 8px;
    background: #2a2a2a;
    border-radius: 2px;
    overflow: hidden;
  }

  .pct-bar-fill {
    display: block;
    height: 100%;
    background: #4A90D9;
    border-radius: 2px;
  }

  .pct-text {
    width: 24px;
    text-align: right;
    font-size: 10px;
    color: #999;
  }

  .col-files {
    width: 36px;
    min-width: 36px;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ext-footer {
    height: 20px;
    display: flex;
    align-items: center;
    padding: 0 4px;
    background: #252525;
    border-top: 1px solid #333;
    font-size: 10px;
    color: #888;
    flex-shrink: 0;
  }
</style>
