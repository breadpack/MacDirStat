<script lang="ts">
  import type { FlatTreeRow } from "../types";
  import { columns } from "../stores/columnStore";
  import { formatSize, formatNumber, formatDate } from "../utils/format";
  import { selectedPath, hoveredPath } from "../stores/selectionStore";
  import { scanningDirs } from "../stores/scanStore";

  interface Props {
    row: FlatTreeRow;
    parentSize: number;
    onToggle: (path: string) => void;
    onContextMenu?: (e: MouseEvent) => void;
  }

  let { row, parentSize, onToggle, onContextMenu }: Props = $props();

  let columnDefs = $derived($columns);

  let visibleColumns = $derived(columnDefs.filter((c) => c.visible));

  let isSelected = $derived($selectedPath === row.node.path);
  let isHovered = $derived($hoveredPath === row.node.path);
  let isScanning = $derived(row.node.is_dir && $scanningDirs.has(row.node.path));

  // displaySize는 부모(TreeView)에서 throttled sortSizes로 계산하여 전달
  let displaySize = $derived(row.displaySize);

  let displayParentSize = $derived(parentSize);

  let percentage = $derived(displayParentSize > 0 ? (displaySize / displayParentSize) * 100 : 0);

  let attrText = $derived.by(() => {
    const parts: string[] = [];
    if (row.node.is_symlink) parts.push("L");
    if (row.node.is_hidden) parts.push("H");
    if (row.node.is_readonly) parts.push("R");
    return parts.join("");
  });

  function handleClick() {
    selectedPath.set(row.node.path);
  }

  function handleDblClick() {
    if (row.hasChildren) {
      onToggle(row.node.path);
    }
  }

  let hoverRaf: number | undefined;

  function handleMouseEnter() {
    if (hoverRaf) cancelAnimationFrame(hoverRaf);
    hoverRaf = requestAnimationFrame(() => hoveredPath.set(row.node.path));
  }

  function handleMouseLeave() {
    if (hoverRaf) cancelAnimationFrame(hoverRaf);
    hoverRaf = requestAnimationFrame(() => hoveredPath.set(null));
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_interactive_supports_focus -->
<div
  class="tree-row"
  class:selected={isSelected}
  class:hovered={isHovered}
  role="treeitem"
  aria-selected={isSelected}
  aria-expanded={row.hasChildren ? row.expanded : undefined}
  aria-level={row.depth + 1}
  onclick={handleClick}
  ondblclick={handleDblClick}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  oncontextmenu={(e) => { e.preventDefault(); onContextMenu?.(e); }}
>
  {#each visibleColumns as col (col.id)}
    {#if col.id === "name"}
      <div class="cell cell-name" style="flex: 1; min-width: {col.minWidth}px; padding-left: {row.depth * 20 + 8}px;">
        {#if row.hasChildren}
          <button class="toggle" onclick={() => onToggle(row.node.path)}>
            {row.expanded ? "\u25BC" : "\u25B6"}
          </button>
        {:else}
          <span class="toggle-spacer"></span>
        {/if}
        {#if isScanning}
          <span class="icon spinner">&#x21BB;</span>
        {:else}
          <span class="icon">{row.node.is_dir ? "\uD83D\uDCC1" : "\uD83D\uDCC4"}</span>
        {/if}
        {#if row.node.cleanup_pattern_id}
          <span class="cleanup-badge" title="Cleanable: {row.node.cleanup_pattern_id}">&#x1F9F9;</span>
        {/if}
        <span class="name-text" title={row.node.name}>{row.node.name}</span>
      </div>
    {:else if col.id === "size"}
      <div class="cell cell-right" style="width: {col.width}px;">
        {formatSize(displaySize)}
      </div>
    {:else if col.id === "percent"}
      <div class="cell" style="width: {col.width}px;">
        <div class="bar-container">
          <div class="bar" style="width: {percentage}%"></div>
        </div>
      </div>
    {:else if col.id === "files"}
      <div class="cell cell-right" style="width: {col.width}px;">
        {row.node.is_dir ? formatNumber(row.node.file_count) : ""}
      </div>
    {:else if col.id === "subdirs"}
      <div class="cell cell-right" style="width: {col.width}px;">
        {row.node.is_dir ? formatNumber(row.node.dir_count) : ""}
      </div>
    {:else if col.id === "modified"}
      <div class="cell" style="width: {col.width}px;">
        {formatDate(row.node.modified)}
      </div>
    {:else if col.id === "attributes"}
      <div class="cell cell-center" style="width: {col.width}px;" title={attrText || ""}>
        {#if row.node.is_symlink}<span class="attr" title="Symlink">L</span>{/if}
        {#if row.node.is_hidden}<span class="attr" title="Hidden">H</span>{/if}
        {#if row.node.is_readonly}<span class="attr" title="Read-only">R</span>{/if}
      </div>
    {/if}
  {/each}
</div>

<style>
  .tree-row {
    display: flex;
    align-items: center;
    height: 24px;
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
    color: #ccc;
  }

  .tree-row:hover,
  .tree-row.hovered {
    background: #2a2a2a;
  }

  .tree-row.selected {
    background: #264f78;
  }

  .cell {
    display: flex;
    align-items: center;
    height: 100%;
    padding: 0 6px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .cell-name {
    flex-shrink: 1;
    min-width: 0;
  }

  .cell-right {
    justify-content: flex-end;
    font-variant-numeric: tabular-nums;
    color: #999;
  }

  .cell-center {
    justify-content: center;
  }

  .toggle {
    background: none;
    border: none;
    color: #888;
    cursor: pointer;
    padding: 0;
    width: 16px;
    font-size: 10px;
    flex-shrink: 0;
  }

  .toggle-spacer {
    width: 16px;
    flex-shrink: 0;
    display: inline-block;
  }

  .icon {
    margin: 0 4px;
    font-size: 14px;
    flex-shrink: 0;
  }

  .icon.spinner {
    display: inline-block;
    animation: spin 1s linear infinite;
    color: #4A90D9;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .cleanup-badge {
    font-size: 11px;
    margin-right: 2px;
    flex-shrink: 0;
    opacity: 0.7;
  }

  .tree-row:hover .cleanup-badge {
    opacity: 1;
  }

  .name-text {
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .bar-container {
    width: 100%;
    height: 12px;
    background: #222;
    border-radius: 2px;
    overflow: hidden;
  }

  .bar {
    height: 100%;
    background: #4A90D9;
    border-radius: 2px;
    min-width: 1px;
  }

  .attr {
    font-size: 10px;
    font-weight: bold;
    color: #888;
    margin: 0 1px;
  }
</style>
